import os
import re
import subprocess
import sys
import threading
import tkinter as tk
import shutil
from tkinter import messagebox, simpledialog, ttk
import pexpect
import argparse


def parse_resident_line(line):
    """Parse one wrapper output line of the form:
        [Info] Credential ID: <id>, User: <name>, Email: <email>, Handle: <user_id>
    Returns a dict with keys credential_id, user, email, handle, or None if the
    line is not a credential line. Missing trailing fields default to "".
    """
    if "Credential ID: " not in line:
        return None
    result = {"credential_id": "", "user": "", "email": "", "handle": ""}
    mapping = {
        "Credential ID": "credential_id",
        "User": "user",
        "Email": "email",
        "Handle": "handle",
    }
    # Split on ", " but only for the known keys to avoid breaking values.
    for part in re.split(r",\s+(?=(?:Credential ID|User|Email|Handle):)", line):
        if ": " not in part:
            continue
        key, _, val = part.partition(": ")
        key = key.replace("[Info] ", "").strip()
        if key in mapping:
            result[mapping[key]] = val.strip()
    return result


def detect_terminal():
    candidates = [
        ("gnome-terminal", ["--"]),
        ("x-terminal-emulator", ["-e"]),
        ("xterm", ["-e"]),
        ("konsole", ["-e"]),
        ("lxterminal", ["-e"]),
        ("tilix", ["-e"]),
        ("mate-terminal", ["-e"]),
        ("ghostty", ["-e"]),
    ]
    for term, flag in candidates:
        if shutil.which(term):
            return term, flag
    return None, None

FIDO_COMMAND = "./fido2-manage.sh"
TERM, TERM_FLAG = detect_terminal()

PIN = None

def set_dpi_awareness():
    
    # Set rowheight based on screen DPI and size
    # Get screen's DPI (dots per inch)
    try:
        # For Windows, use ctypes to get DPI awareness
        if sys.platform.startswith("win"):
            awareness = ctypes.c_int()
            ctypes.windll.shcore.SetProcessDpiAwareness(2)  # Per-monitor DPI aware
            hdc = ctypes.windll.user32.GetDC(0)
            dpi = ctypes.windll.gdi32.GetDeviceCaps(hdc, 88)
        else:
            # For Linux/Mac, use Tkinter's winfo_fpixels
            root = tk.Tk()
            dpi = root.winfo_fpixels('1i')
            root.destroy()
    except Exception:
        dpi = 96  # Fallback to standard DPI

    # Get screen height in pixels
    try:
        root = tk.Tk()
        screen_height = root.winfo_screenheight()
        root.destroy()
    except Exception:
        screen_height = 1080  # Fallback

    # Calculate rowheight: base it on DPI and screen height
    # Typical rowheight at 96dpi and 1080p is 20, so scale proportionally
    base_rowheight = 40
    base_dpi = 96
    base_screen_height = 1080
    rowheight = int(base_rowheight * (dpi / base_dpi) * (screen_height / base_screen_height))
    rowheight = max(18, min(rowheight, 40))  # Clamp to reasonable range

    style = ttk.Style()
    style.configure("Treeview", rowheight=rowheight)

def get_device_list():
    try:
        result = subprocess.run([FIDO_COMMAND, "-list"], capture_output=True, text=True)
        device_list = result.stdout.strip().split("\n")
        return device_list
    except Exception as e:
        print(f"Error executing device list command: {e}")
        return []

def execute_info_command(device_digit):
    global PIN
    tree.delete(*tree.get_children())

    info_command = [FIDO_COMMAND, "-info", "-device", device_digit]

    try:
        result = subprocess.run(info_command, capture_output=True, text=True)
        if result.returncode == 0:
            for line in result.stdout.splitlines():
                if ": " in line:
                    key, value = line.split(": ", 1)
                    tree.insert("", tk.END, values=(key, value))
        else:
            raise subprocess.CalledProcessError(result.returncode, info_command)
    except Exception as e:
        messagebox.showerror("Error", f"Command execution failed: {e}\nOutput: {result.stderr}")

    storage_command = f"{FIDO_COMMAND} -storage -device {device_digit}"

    try:
        child = pexpect.spawn(storage_command, encoding="utf-8", timeout=10)
        index = child.expect([r"Enter PIN for", pexpect.EOF, pexpect.TIMEOUT])
        output = child.before

        if index == 0:
            pin_button.config(text="Change PIN", state=tk.ACTIVE, command=change_pin)

        if index == 1:
            messagebox.showwarning(
                "Warning",
                "No PIN is set for this key. You must set a PIN before managing passkeys."
            )
            pin_button.config(text="Set PIN", state=tk.ACTIVE, command=set_pin)

        if index == 2:
            if "FIDO_ERR_PIN_REQUIRED" in output:
                pin_button.config(text="Set PIN", state=tk.ACTIVE, command=set_pin)

            if "FIDO_ERR_PIN_INVALID" in output:
                messagebox.showerror("Error", "Invalid PIN provided")

            if "FIDO_ERR_PIN_AUTH_BLOCKED" in output:
                messagebox.showerror("Error", "Wrong PIN provided too many times. Reinsert the key")

            if "FIDO_ERR_INVALID_CBOR" in output:
                messagebox.showerror(
                    "Error",
                    "This is an older key (probably FIDO2.0). No passkey management is possible with this key. Only basic information will be shown.",
                )

            messagebox.showerror("Unexpected Device Output", output)
            return False

    except Exception as e:
        messagebox.showerror("Error", f"Command execution failed: {e}\nOutput: {result.stderr}")

def on_device_selected(event):
    selected_device = device_var.get()
    match = re.search(r"\[(\d+)\]", selected_device)

    if match:
        device_digit = match.group(1)
        execute_info_command(device_digit)
        passkeys_button.config(state=tk.NORMAL)
    else:
        messagebox.showinfo("Device Selected", "No digit found in the selected device")

def get_pin():
    global PIN
    PIN = simpledialog.askstring("PIN Code", "Enter your PIN code:", show="*")
    return PIN

def on_passkeys_button_click():
    global PIN
    selected_device = device_var.get()
    match = re.search(r"\[(\d+)\]", selected_device)
    if not match:
        messagebox.showinfo("Device Selected", "No digit found in the selected device")
        return

    device_digit = match.group(1)

    if PIN is None:
        get_pin()
        if PIN is None:
            return

    command = [
        FIDO_COMMAND,
        "-residentKeys",
        "-pin",
        PIN,
        "-device",
        device_digit,
    ]
    try:
        result = subprocess.run(command, capture_output=True, text=True)
        if result.returncode == 0:
            domains = []
            for line in result.stdout.splitlines():
                match = re.search(r"= (.+)$", line)
                if match:
                    domains.append(match.group(1))

            cumulated_output = []
            for domain in domains:
                domain_command = [
                    FIDO_COMMAND,
                    "-residentKeys",
                    "-domain",
                    domain,
                    "-pin",
                    PIN,
                    "-device",
                    device_digit,
                ]
                domain_result = subprocess.run(
                    domain_command, capture_output=True, text=True
                )

                if domain_result.returncode == 0:
                    cumulated_output.append(
                        f"Domain: {domain}\n{domain_result.stdout}"
                    )
                else:
                    raise subprocess.CalledProcessError(
                        domain_result.returncode, domain_command
                    )

            cumulated_output_str = "\n\n".join(cumulated_output)
            show_output_in_new_window(cumulated_output_str, device_digit)
        else:
            raise subprocess.CalledProcessError(result.returncode, command)
    except Exception as e:
        messagebox.showerror(
            "Error", f"Command execution failed: {e}\nOutput: {result.stderr}"
        )

def set_pin():
    global PIN
    selected_device = device_var.get()
    match = re.search(r"\[(\d+)\]", selected_device)
    if not match:
        return

    device_digit = match.group(1)

    while True:
        new_pin = simpledialog.askstring(
            "New PIN", "Enter your new PIN code:", show="*"
        )
        if new_pin is None:
            PIN = None
            return

        new_pin_confirmed = simpledialog.askstring(
            "Confirm new PIN", "Enter your new PIN code:", show="*"
        )
        if new_pin_confirmed is None:
            PIN = None
            return

        if new_pin == new_pin_confirmed:
            break
        else:
            messagebox.showerror("Error", "New PIN entries do not match!")

    command = f"{FIDO_COMMAND} -setPIN -device {device_digit}"

    try:
        child = pexpect.spawn(command, encoding="utf-8", timeout=20)
        child.expect("Enter new PIN")
        child.sendline(new_pin)
        child.expect("Enter the same PIN again")
        child.sendline(new_pin_confirmed)

        PIN = new_pin

        child.expect(pexpect.EOF)
        output = child.before.strip()

        if "FIDO_ERR_PIN_POLICY_VIOLATION" in output:
            match = re.search(r"minpinlen:\s*(\d+)", output)
            if match:
                min_pin_len = match.group(1)
            messagebox.showerror(
                "PIN not accepted.",
                f"The provided PIN does not fulfill the requirements of your device.\n"
                f"The PIN has to be at least {min_pin_len} long and must not be an easily guessable sequence, like e.g. 123456"
            )
            PIN = None
        elif "error" in output.lower() or "FIDO_ERR" in output:
            messagebox.showerror("PIN Change Failed", output)
            PIN = None
        else:
            messagebox.showinfo("Success", "PIN successfully set!")
    except pexpect.exceptions.TIMEOUT:
        messagebox.showerror("Timeout", "The device did not respond in time.")
        PIN = None
    except Exception as e:
        messagebox.showerror("Error", str(e))
        PIN = None

def change_pin():
    global PIN
    if PIN is None:
        get_pin()

    selected_device = device_var.get()
    match = re.search(r"\[(\d+)\]", selected_device)
    if not match:
        return

    device_digit = match.group(1)
    while True:
        old_pin = PIN

        new_pin = simpledialog.askstring(
            "New PIN", "Enter your new PIN code:", show="*"
        )
        new_pin_confirmed = simpledialog.askstring(
            "Confirm new PIN", "Enter your new PIN code:", show="*"
        )
        if new_pin == new_pin_confirmed:
            break
        else:
            messagebox.showerror("Error", "New PIN entries do not match!")

    command = f"{FIDO_COMMAND} -changePIN -device {device_digit}"

    try:
        child = pexpect.spawn(command, encoding="utf-8", timeout=20)

        i = child.expect([
            "Touch",
            "Tap",
            "Waiting for user",
            "Enter current PIN",
            pexpect.EOF,
            pexpect.TIMEOUT
        ])

        if i in [0, 1, 2]:
            messagebox.showinfo(
                "Touch Required",
                "Please touch your FIDO security key to continue."
            )
            child.expect("Enter current PIN")

        child.sendline(old_pin)

        child.expect("Enter new PIN")
        child.sendline(new_pin)
        child.expect("Enter the same PIN again")
        child.sendline(new_pin_confirmed)

        PIN = new_pin

        output = child.before.strip()

        idx = child.expect(["FIDO_ERR_PIN_POLICY_VIOLATION", pexpect.EOF], timeout=1)
        if idx == 0:
            command = f"{FIDO_COMMAND} -info -device {device_digit}"
            info = pexpect.spawn(command, encoding="utf-8")
            info.expect(pexpect.EOF)
            info_text = info.before

            match = re.search(r"minpinlen:\s*(\d+)", info_text)
            if match:
                min_pin_len = match.group(1)
            else:
                min_pin_len = "?"

            messagebox.showerror(
                "PIN not accepted",
                f"The provided PIN violates the device policy.\n"
                f"The PIN must be at least {min_pin_len} digits long and "
                f"must not be an easily guessable sequence (e.g. 123456)."
            )
            return

        child.expect(pexpect.EOF)
        output = child.before.strip()

        if "error" in output.lower() or "FIDO_ERR" in output:
            messagebox.showerror("PIN Change Failed", output)
        else:
            messagebox.showinfo("Success", "PIN successfully changed!")

    except pexpect.exceptions.TIMEOUT:
        messagebox.showerror("Timeout", "The device did not respond in time.")
    except Exception as e:
        messagebox.showerror("Error", str(e))

def refresh_combobox():
    device_combobox.set("")
    tree.delete(*tree.get_children())
    passkeys_button.config(state=tk.DISABLED)
    pin_button.config(state=tk.DISABLED)
    device_list = get_device_list()
    if not device_list:
        print("No devices found.")
    device_combobox["values"] = device_list

def show_output_in_new_window(output, device_digit):
    new_window = tk.Toplevel(root)
    # Ensure rows are tall enough for the (possibly HiDPI-scaled) font so cell
    # text is not vertically clipped in this window.
    try:
        from tkinter import font as _tkfont
        _line = _tkfont.nametofont("TkDefaultFont").metrics("linespace")
        ttk.Style().configure("Treeview", rowheight=_line + 8)
        _scale = max(1.0, _line / 18.0)
        new_window.geometry(f"{int(800 * _scale)}x{int(650 * _scale)}")
    except Exception:
        new_window.geometry("800x650")
    new_window.title("Resident Keys / Passkeys")

    tree_new_window = ttk.Treeview(
        new_window, columns=("Domain", "Credential ID", "User", "Handle"), show="headings"
    )
    tree_new_window.heading("Domain", text="Domain", anchor="w")
    tree_new_window.heading("Credential ID", text="Credential ID", anchor="w")
    tree_new_window.heading("User", text="User", anchor="w")
    tree_new_window.heading("Handle", text="Handle (User ID)", anchor="w")
    tree_new_window.column("Domain", width=170, minwidth=110, stretch=False, anchor="w")
    tree_new_window.column("Credential ID", width=280, minwidth=180, stretch=False, anchor="w")
    tree_new_window.column("User", width=220, minwidth=140, stretch=True, anchor="w")
    tree_new_window.column("Handle", width=240, minwidth=140, stretch=False, anchor="w")

    # Pack scrollbars first so the tree receives the correct remaining area
    # (prevents rows being vertically compressed / clipped).
    tree_scrollbar_y = ttk.Scrollbar(
        new_window, orient="vertical", command=tree_new_window.yview
    )
    tree_scrollbar_y.pack(side="right", fill="y")
    tree_scrollbar_x = ttk.Scrollbar(
        new_window, orient="horizontal", command=tree_new_window.xview
    )
    tree_scrollbar_x.pack(side="bottom", fill="x")
    tree_new_window.configure(
        yscrollcommand=tree_scrollbar_y.set,
        xscrollcommand=tree_scrollbar_x.set,
    )
    tree_new_window.pack(expand=True, fill=tk.BOTH, padx=10, pady=10)

    current_domain = ""
    for line in output.splitlines():
        if line.startswith("Domain: "):
            current_domain = line.split("Domain: ")[1].strip()
            continue
        parsed = parse_resident_line(line)
        if parsed is None:
            continue
        user_display = parsed["user"]
        if parsed["email"]:
            user_display = f"{user_display} <{parsed['email']}>".strip()
        tree_new_window.insert(
            "",
            tk.END,
            values=(
                current_domain,
                parsed["credential_id"],
                user_display,
                parsed["handle"],
            ),
        )

    def show_selected_value():
        selected_item = tree_new_window.selection()
        if selected_item:
            value = tree_new_window.item(selected_item, "values")[1]
            new_window.destroy()
            command = [
                FIDO_COMMAND,
                "-delete",
                "-device",
                device_digit,
                "-credential",
                value,
            ]
            if sys.platform.startswith("win"):
                subprocess.Popen(["start", "cmd", "/c"] + command, shell=True)
            elif sys.platform.startswith("linux"):
                subprocess.Popen([TERM] + TERM_FLAG + command)

    show_value_button = tk.Button(
        new_window, text="Delete Passkey", command=show_selected_value
    )
    show_value_button.pack(side=tk.LEFT, padx=10, pady=10)

    def edit_selected_metadata():
        selected_item = tree_new_window.selection()
        if not selected_item:
            messagebox.showinfo("No selection", "Select a passkey to edit.")
            return
        row_values = tree_new_window.item(selected_item, "values")
        cred_id = row_values[1]
        prefilled_handle = row_values[3] if len(row_values) > 3 else ""
        user_id = simpledialog.askstring(
            "Edit metadata",
            "User ID (base64 user handle) for this credential:",
            initialvalue=prefilled_handle,
        )
        if not user_id:
            return
        new_name = simpledialog.askstring("Edit metadata", "New name (e.g. user@example.com):") or ""
        new_display = simpledialog.askstring("Edit metadata", "New display name:") or ""
        args = [
            FIDO_COMMAND, "-editCredential",
            "-device", device_digit,
            "-credential", cred_id,
            "-userId", user_id,
            "-name", new_name,
            "-displayName", new_display,
        ]
        if PIN:
            args += ["-pin", PIN]
        if sys.platform.startswith("linux"):
            subprocess.Popen([TERM] + TERM_FLAG + args)
        else:
            subprocess.run(args)

    edit_button = tk.Button(
        new_window, text="Edit Metadata", command=edit_selected_metadata
    )
    edit_button.pack(side=tk.LEFT, padx=10, pady=10)

def _selected_device_digit():
    """Return the device number from the combobox selection, or None."""
    match = re.search(r"\[(\d+)\]", device_var.get())
    if not match:
        messagebox.showinfo("No device", "Please select a device first.")
        return None
    return match.group(1)


def _run_wrapper(args, need_pin=False):
    """Run fido2-manage.sh with args; optionally prompt for and pass the PIN.
    Returns CompletedProcess or None if the user cancelled a PIN prompt."""
    global PIN
    cmd = [FIDO_COMMAND] + args
    if need_pin:
        if PIN is None:
            get_pin()
        if PIN is None:
            return None
        cmd += ["-pin", PIN]
    return subprocess.run(cmd, capture_output=True, text=True)


def _run_async(args, on_done, need_pin=False):
    """Run a wrapper command in a background thread so the GUI stays responsive.
    The PIN prompt (if any) happens on the calling thread; the subprocess runs
    in a worker thread and ``on_done(result)`` is scheduled back on the Tk main
    loop via root.after. Returns the started Thread, or None if PIN cancelled.
    """
    global PIN
    if need_pin:
        if PIN is None:
            get_pin()
        if PIN is None:
            return None
    run_args = list(args)
    if need_pin and PIN:
        run_args += ["-pin", PIN]

    def worker():
        result = subprocess.run([FIDO_COMMAND] + run_args, capture_output=True, text=True)
        root.after(0, lambda: on_done(result))

    t = threading.Thread(target=worker, daemon=True)
    t.start()
    return t


def show_stats():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return

    def _display(result):
        win = tk.Toplevel(root)
        win.title(f"Storage & Statistics - Device {device_digit}")
        try:
            from tkinter import font as _tkfont
            _s = max(1.0, _tkfont.nametofont("TkDefaultFont").metrics("linespace") / 18.0)
            win.geometry(f"{int(600 * _s)}x{int(500 * _s)}")
        except Exception:
            win.geometry("600x500")
        txt = tk.Text(win, wrap="word")
        txt.insert("1.0", (result.stdout or "") + (("\n" + result.stderr) if result.stderr else ""))
        txt.config(state="disabled")
        txt.pack(expand=True, fill=tk.BOTH, padx=10, pady=10)

    _run_async(["-stats", "-device", device_digit], _display, need_pin=True)


def generate_ssh_key():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return

    dlg = tk.Toplevel(root)
    dlg.title("Generate SSH Security-Key")
    dlg.transient(root)

    ttk.Label(dlg, text="Key type:").grid(row=0, column=0, sticky="w", padx=10, pady=6)
    type_var = tk.StringVar(value="ed25519-sk")
    ttk.Combobox(
        dlg, textvariable=type_var, values=["ed25519-sk", "ecdsa-sk"],
        state="readonly", width=20,
    ).grid(row=0, column=1, sticky="w", padx=10, pady=6)

    resident_var = tk.BooleanVar(value=False)
    ttk.Checkbutton(
        dlg, text="Resident (store handle on key)", variable=resident_var
    ).grid(row=1, column=0, columnspan=2, sticky="w", padx=10, pady=6)

    ttk.Label(dlg, text="Output path:").grid(row=2, column=0, sticky="w", padx=10, pady=6)
    out_var = tk.StringVar(value=os.path.expanduser("~/.ssh/id_ed25519_sk"))
    ttk.Entry(dlg, textvariable=out_var, width=40).grid(
        row=2, column=1, sticky="w", padx=10, pady=6
    )

    ttk.Label(dlg, text="Application (optional):").grid(
        row=3, column=0, sticky="w", padx=10, pady=6
    )
    app_var = tk.StringVar(value="")
    ttk.Entry(dlg, textvariable=app_var, width=40).grid(
        row=3, column=1, sticky="w", padx=10, pady=6
    )

    def do_generate():
        args = [
            "-sshKeygen", "-device", device_digit,
            "-sshType", type_var.get(),
            "-sshOutput", out_var.get(),
        ]
        if resident_var.get():
            args.append("-sshResident")
        if app_var.get().strip():
            args += ["-sshApplication", app_var.get().strip()]
        dlg.destroy()
        messagebox.showinfo(
            "Touch required",
            "Touch your security key when it blinks to complete key generation.",
        )
        # ssh-keygen is interactive (touch); run in a terminal so prompts show.
        if sys.platform.startswith("linux"):
            subprocess.Popen([TERM] + TERM_FLAG + [FIDO_COMMAND] + args)
        else:
            subprocess.run([FIDO_COMMAND] + args)

    ttk.Button(dlg, text="Generate", command=do_generate).grid(
        row=4, column=0, columnspan=2, pady=12
    )


def manage_large_blob():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return

    dlg = tk.Toplevel(root)
    dlg.title("Large Blob Management")
    dlg.transient(root)

    ttk.Label(dlg, text="Relying-party ID:").grid(
        row=0, column=0, sticky="w", padx=10, pady=6
    )
    rp_var = tk.StringVar(value="")
    ttk.Entry(dlg, textvariable=rp_var, width=36).grid(
        row=0, column=1, sticky="w", padx=10, pady=6
    )

    ttk.Label(dlg, text="Credential ID (if multiple):").grid(
        row=1, column=0, sticky="w", padx=10, pady=6
    )
    cred_var = tk.StringVar(value="")
    ttk.Entry(dlg, textvariable=cred_var, width=36).grid(
        row=1, column=1, sticky="w", padx=10, pady=6
    )

    ttk.Label(dlg, text="Blob file:").grid(row=2, column=0, sticky="w", padx=10, pady=6)
    file_var = tk.StringVar(value="")
    ttk.Entry(dlg, textvariable=file_var, width=36).grid(
        row=2, column=1, sticky="w", padx=10, pady=6
    )

    def _base_args():
        args = ["-device", device_digit, "-rpId", rp_var.get().strip()]
        if cred_var.get().strip():
            args += ["-credential", cred_var.get().strip()]
        return args

    def _require(*fields):
        for label, val in fields:
            if not val.strip():
                messagebox.showerror("Missing", f"{label} is required.")
                return False
        return True

    def do_get():
        if not _require(("Relying-party ID", rp_var.get()), ("Blob file", file_var.get())):
            return
        res = _run_wrapper(
            ["-largeBlobGet"] + _base_args() + ["-blobFile", file_var.get().strip()],
            need_pin=True,
        )
        if res is not None:
            messagebox.showinfo("Large Blob", (res.stdout or "") + (res.stderr or ""))

    def do_set():
        if not _require(("Relying-party ID", rp_var.get()), ("Blob file", file_var.get())):
            return
        res = _run_wrapper(
            ["-largeBlobSet"] + _base_args() + ["-blobFile", file_var.get().strip()],
            need_pin=True,
        )
        if res is not None:
            messagebox.showinfo("Large Blob", (res.stdout or "") + (res.stderr or ""))

    def do_delete():
        if not _require(("Relying-party ID", rp_var.get())):
            return
        if not messagebox.askyesno("Confirm", "Delete the large-blob? This is irreversible."):
            return
        args = [FIDO_COMMAND, "-largeBlobDelete"] + _base_args()
        if PIN:
            args += ["-pin", PIN]
        subprocess.Popen([TERM] + TERM_FLAG + args)

    ttk.Button(dlg, text="Get", command=do_get).grid(row=3, column=0, pady=12, padx=6)
    ttk.Button(dlg, text="Set", command=do_set).grid(row=3, column=1, sticky="w", pady=12)
    ttk.Button(dlg, text="Delete", command=do_delete).grid(
        row=4, column=0, columnspan=2, pady=4
    )


def show_bio_list():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    result = _run_wrapper(["-bioList", "-device", device_digit], need_pin=True)
    if result is None:
        return
    messagebox.showinfo("Biometric enrollments", (result.stdout or "") + (result.stderr or ""))


def bio_delete():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    template_id = simpledialog.askstring("Delete biometric", "Template ID to delete:")
    if not template_id:
        return
    args = [FIDO_COMMAND, "-bioDelete", "-device", device_digit, "-templateId", template_id]
    if PIN:
        args += ["-pin", PIN]
    if sys.platform.startswith("linux"):
        subprocess.Popen([TERM] + TERM_FLAG + args)
    else:
        subprocess.run(args)


def bio_rename():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    template_id = simpledialog.askstring("Rename biometric", "Template ID:")
    if not template_id:
        return
    new_name = simpledialog.askstring("Rename biometric", "New template name:")
    if not new_name:
        return
    res = _run_wrapper(
        ["-bioRename", "-device", device_digit, "-templateId", template_id,
         "-templateName", new_name],
        need_pin=True,
    )
    if res is not None:
        messagebox.showinfo("Biometric rename", (res.stdout or "") + (res.stderr or ""))


def set_pin_min_rps():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    rps = simpledialog.askstring(
        "Min-PIN RP list", "Comma-separated relying-party IDs allowed to read min PIN length:"
    )
    if not rps:
        return
    res = _run_wrapper(["-setPinMinRPs", rps, "-device", device_digit], need_pin=True)
    if res is not None:
        messagebox.showinfo("Min-PIN RPs", (res.stdout or "") + (res.stderr or ""))


def gen_blob_key():
    path = simpledialog.askstring("Generate blob key", "Output path for the AES-256 base64 key:")
    if not path:
        return
    res = _run_wrapper(["-genBlobKey", path])
    if res is not None:
        messagebox.showinfo("Blob key", (res.stdout or "") + (res.stderr or ""))


def show_ssh_list():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    result = _run_wrapper(["-sshList", "-device", device_digit], need_pin=True)
    if result is None:
        return
    messagebox.showinfo("SSH resident credentials", (result.stdout or "") + (result.stderr or ""))


def ssh_download():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    target = simpledialog.askstring(
        "Download SSH keys", "Target directory:", initialvalue=os.path.expanduser("~/.ssh")
    )
    if not target:
        return
    messagebox.showinfo("Touch required", "Touch your security key when it blinks to download resident SSH keys.")
    args = [FIDO_COMMAND, "-sshDownload", "-device", device_digit, "-sshDir", target]
    if sys.platform.startswith("linux"):
        subprocess.Popen([TERM] + TERM_FLAG + args)
    else:
        subprocess.run(args)


def export_audit():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    fmt = simpledialog.askstring("Audit format", "Format (json or csv):", initialvalue="json")
    if fmt not in ("json", "csv"):
        messagebox.showerror("Invalid", "Format must be 'json' or 'csv'.")
        return
    out = simpledialog.askstring("Audit output", "Output file path:")
    if not out:
        return
    res = _run_wrapper(
        ["-audit", "-device", device_digit, "-auditFormat", fmt, "-auditOutput", out],
        need_pin=True,
    )
    if res is not None:
        messagebox.showinfo("Audit export", (res.stdout or "") + (res.stderr or ""))


def age_setup():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    out = simpledialog.askstring(
        "age setup", "age identity output path:",
        initialvalue=os.path.expanduser("~/.age/fido2-hmac.txt"),
    )
    if not out:
        return
    messagebox.showinfo("Touch required", "Touch your security key when prompted to create the age identity.")
    args = [FIDO_COMMAND, "-ageSetup", "-device", device_digit, "-ageOutput", out]
    if sys.platform.startswith("linux"):
        subprocess.Popen([TERM] + TERM_FLAG + args)
    else:
        subprocess.run(args)


def luks_enroll():
    device_digit = _selected_device_digit()
    if device_digit is None:
        return
    dev = simpledialog.askstring("LUKS enroll", "Block device (e.g. /dev/sda2):")
    if not dev:
        return
    res = _run_wrapper(["-luksEnroll", "-device", device_digit, "-luksDevice", dev])
    if res is not None:
        messagebox.showwarning(
            "LUKS enrollment (manual)",
            "This high-risk command is NOT run automatically. Review and run it "
            "yourself:\n\n" + (res.stdout or "") + (res.stderr or ""),
        )


def hide_to_tray():
    """Hide (withdraw) the main window instead of destroying it, so closing the
    window minimises to the tray rather than quitting."""
    if root is not None:
        root.withdraw()


def show_window(*_args):
    """Re-show the main window from the tray."""
    if root is not None:
        root.deiconify()
        root.lift()


def quit_app(*_args):
    """Actually quit the application (tray 'Quit')."""
    if root is not None:
        root.destroy()


def start_tray():
    """Create an in-process AppIndicator tray icon with Open/Quit so the window
    can minimise to tray. Returns the indicator, or None if AppIndicator is
    unavailable (in which case closing the window simply quits)."""
    try:
        import gi
        gi.require_version("Gtk", "3.0")
        gi.require_version("AppIndicator3", "0.1")
        from gi.repository import Gtk, AppIndicator3
    except (ImportError, ValueError):
        return None

    indicator = AppIndicator3.Indicator.new(
        "fido2-manage",
        "security-high",
        AppIndicator3.IndicatorCategory.APPLICATION_STATUS,
    )
    indicator.set_status(AppIndicator3.IndicatorStatus.ACTIVE)

    menu = Gtk.Menu()
    open_item = Gtk.MenuItem(label="Open")
    open_item.connect("activate", lambda _w: root.after(0, show_window))
    menu.append(open_item)
    quit_item = Gtk.MenuItem(label="Quit")
    quit_item.connect("activate", lambda _w: root.after(0, quit_app))
    menu.append(quit_item)
    menu.show_all()
    indicator.set_menu(menu)
    return indicator


def show_about_message():
    messagebox.showinfo(
        "About",
        "The FIDO2.1 Security Key Management Tool is a utility designed to manage and interact with FIDO2.1 security keys.\r\nIt provides functionalities to view information, manage relying parties, and perform various operations on connected FIDO2.1 devices.\r\n\r\n(c)TOKEN2 Sarl\r\nVersoix, Switzerland",
    )

# parse command-line arguments
def main():
    global root, device_var, device_combobox, tree
    global passkeys_button, pin_button

    parser = argparse.ArgumentParser(description="FIDO2.1 Manager GUI")
    parser.add_argument("-dpi", action="store_true", help="Set DPI awareness for high-DPI displays")
    args = parser.parse_args()

    root = tk.Tk()

    if TERM is None:
        messagebox.showerror(
            "Error",
            "No supported terminal emulator found. Please install xterm or gnome-terminal.",
        )
        sys.exit(1)

    # Set DPI awareness if requested
    if args.dpi:
        set_dpi_awareness()

    # --- HiDPI / Wayland layout fix ---
    # On a scaled HiDPI panel Tk enlarges the default font (e.g. line height ~37px)
    # but the ttk.Treeview keeps its default rowheight (~20px), so cell text gets
    # clipped. Sync the Treeview rowheight (and default column width) to the real
    # font metrics, and size the window to fit.
    try:
        from tkinter import font as _tkfont
        _f = _tkfont.nametofont("TkDefaultFont")
        _line = _f.metrics("linespace")            # actual pixel height of a text line
        _rowheight = _line + 8                      # padding above/below text
        _style = ttk.Style()
        _style.configure("Treeview", rowheight=_rowheight)
        _style.configure("Treeview.Heading", padding=4)
        # Scale the default window with the font so columns have room.
        _scale = max(1.0, _line / 18.0)             # 18px ~= line height at 96 DPI
        _w, _h = int(700 * _scale), int(600 * _scale)
        root.geometry(f"{_w}x{_h}")
    except Exception:
        root.geometry("700x600")

    root.title("FIDO2.1 Manager - Python version 0.1 - (c) Token2")

    top_frame = ttk.Frame(root)
    top_frame.pack(side=tk.TOP, fill=tk.X)

    label = tk.Label(top_frame, text="Select Device:")
    label.pack(side=tk.LEFT, padx=10, pady=10)

    device_list = get_device_list()
    if not device_list:
        device_list = ["No devices found."]
    device_var = tk.StringVar()
    device_combobox = ttk.Combobox(
        top_frame, textvariable=device_var, values=device_list, width=60
    )
    device_combobox.pack(side=tk.LEFT, padx=10, pady=10)
    device_combobox.bind("<<ComboboxSelected>>", on_device_selected)

    refresh_button = tk.Button(top_frame, text="Refresh", command=refresh_combobox)
    refresh_button.pack(side=tk.LEFT, padx=10, pady=10)

    tree_frame = ttk.Frame(root)
    tree_frame.pack(expand=True, fill=tk.BOTH, padx=10, pady=10)
    tree_scrollbar_y = ttk.Scrollbar(tree_frame, orient="vertical")
    tree_scrollbar_x = ttk.Scrollbar(tree_frame, orient="horizontal")
    tree = ttk.Treeview(
        tree_frame,
        columns=("Key", "Value"),
        show="headings",
        yscrollcommand=tree_scrollbar_y.set,
        xscrollcommand=tree_scrollbar_x.set,
    )
    tree_scrollbar_y.config(command=tree.yview)
    tree_scrollbar_x.config(command=tree.xview)
    tree_scrollbar_y.pack(side="right", fill="y")
    tree_scrollbar_x.pack(side="bottom", fill="x")
    tree.heading("Key", text="Key", anchor="w")
    tree.heading("Value", text="Value", anchor="w")
    tree.column("Key", width=200, minwidth=120, stretch=False, anchor="w")
    tree.column("Value", width=460, minwidth=200, stretch=True, anchor="w")
    tree.pack(expand=True, fill=tk.BOTH)

    # --- Tabbed action panel (ttk.Notebook) ---
    notebook = ttk.Notebook(root)
    notebook.pack(side=tk.TOP, fill=tk.X, padx=10, pady=(0, 10))

    def _tab(title):
        f = ttk.Frame(notebook)
        notebook.add(f, text=title)
        return f

    # Credentials tab
    creds_tab = _tab("Credentials")
    passkeys_button = ttk.Button(
        creds_tab, text="Passkeys", state=tk.DISABLED, command=on_passkeys_button_click
    )
    passkeys_button.pack(side=tk.LEFT, padx=5, pady=8)

    # PIN & Device tab
    pin_tab = _tab("PIN & Device")
    pin_button = ttk.Button(pin_tab, text="Set PIN", state=tk.DISABLED, command=set_pin)
    pin_button.pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(pin_tab, text="Stats", command=show_stats).pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(pin_tab, text="Min-PIN RPs", command=set_pin_min_rps).pack(side=tk.LEFT, padx=5, pady=8)

    # SSH tab
    ssh_tab = _tab("SSH")
    ttk.Button(ssh_tab, text="Generate Key", command=generate_ssh_key).pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(ssh_tab, text="List Resident", command=show_ssh_list).pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(ssh_tab, text="Download", command=ssh_download).pack(side=tk.LEFT, padx=5, pady=8)

    # Blobs tab
    blob_tab = _tab("Blobs")
    ttk.Button(blob_tab, text="Large Blob", command=manage_large_blob).pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(blob_tab, text="Generate Blob Key", command=gen_blob_key).pack(side=tk.LEFT, padx=5, pady=8)

    # Biometrics tab
    bio_tab = _tab("Biometrics")
    ttk.Button(bio_tab, text="List", command=show_bio_list).pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(bio_tab, text="Rename", command=bio_rename).pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(bio_tab, text="Delete", command=bio_delete).pack(side=tk.LEFT, padx=5, pady=8)

    # Audit & Encryption tab
    audit_tab = _tab("Audit & Encryption")
    ttk.Button(audit_tab, text="Export Audit", command=export_audit).pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(audit_tab, text="age Setup", command=age_setup).pack(side=tk.LEFT, padx=5, pady=8)
    ttk.Button(audit_tab, text="LUKS Enroll", command=luks_enroll).pack(side=tk.LEFT, padx=5, pady=8)

    about_button = ttk.Button(root, text="About", command=show_about_message)
    about_button.pack(side=tk.RIGHT, padx=5, pady=10)

    # Minimise-to-tray: closing the window hides it; the tray icon restores it.
    indicator = start_tray()
    if indicator is not None:
        root.protocol("WM_DELETE_WINDOW", hide_to_tray)
        # AppIndicator needs a GTK main loop; run it on a daemon thread so it
        # coexists with Tk's mainloop on the main thread.
        def _gtk_loop():  # pragma: no cover - requires live GTK
            import gi
            gi.require_version("Gtk", "3.0")
            from gi.repository import Gtk
            Gtk.main()
        threading.Thread(target=_gtk_loop, daemon=True).start()

    root.mainloop()


if __name__ == "__main__":
    main()
