import os
import re
import subprocess
import sys
import tkinter as tk
import shutil
from tkinter import messagebox, simpledialog, ttk
import pexpect

def detect_terminal():
    candidates = [
        ("gnome-terminal", ["--"]),
        ("x-terminal-emulator", ["-e"]),
        ("xterm", ["-e"]),
        ("konsole", ["-e"]),
        ("lxterminal", ["-e"]),
        ("tilix", ["-e"]),
        ("mate-terminal", ["-e"]),
    ]
    for term, flag in candidates:
        if shutil.which(term):
            return term, flag
    return None, None

FIDO_COMMAND = "./fido2-manage.sh"
TERM, TERM_FLAG = detect_terminal()

if TERM is None:
    messagebox.showerror("Error", "No supported terminal emulator found. Please install xterm or gnome-terminal.")
    sys.exit(1)

PIN = None

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
    new_window.geometry("800x650")
    new_window.title("Resident Keys / Passkeys")

    tree_new_window = ttk.Treeview(
        new_window, columns=("Domain", "Credential ID", "User"), show="headings"
    )
    tree_new_window.heading("Domain", text="Domain")
    tree_new_window.heading("Credential ID", text="Credential ID")
    tree_new_window.heading("User", text="User")
    tree_new_window.pack(expand=True, fill=tk.BOTH, padx=10, pady=10)

    tree_scrollbar_y = ttk.Scrollbar(
        new_window, orient="vertical", command=tree_new_window.yview
    )
    tree_scrollbar_y.pack(side="right", fill="y")
    tree_new_window.configure(yscrollcommand=tree_scrollbar_y.set)
    tree_scrollbar_x = ttk.Scrollbar(
        new_window, orient="horizontal", command=tree_new_window.xview
    )
    tree_scrollbar_x.pack(side="bottom", fill="x")
    tree_new_window.configure(xscrollcommand=tree_scrollbar_x.set)

    current_domain = ""
    for line in output.splitlines():
        if line.startswith("Domain: "):
            current_domain = line.split("Domain: ")[1].strip()
        elif "Credential ID: " in line and "User: " in line:
            credential_id = line.split("Credential ID: ")[1].split(",")[0].strip()
            user = line.split("User: ")[1].strip()
            user = re.sub(re.escape(credential_id), "", user).strip()
            tree_new_window.insert(
                "", tk.END, values=(current_domain, credential_id, user)
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
    show_value_button.pack(pady=10)

def show_about_message():
    messagebox.showinfo(
        "About",
        "The FIDO2.1 Security Key Management Tool is a utility designed to manage and interact with FIDO2.1 security keys.\r\nIt provides functionalities to view information, manage relying parties, and perform various operations on connected FIDO2.1 devices.\r\n\r\n(c)TOKEN2 Sarl\r\nVersoix, Switzerland",
    )

root = tk.Tk()
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
tree.heading("Key", text="Key")
tree.heading("Value", text="Value")
tree.pack(expand=True, fill=tk.BOTH)

passkeys_button = ttk.Button(
    root, text="Passkeys", state=tk.DISABLED, command=on_passkeys_button_click
)
passkeys_button.pack(side=tk.LEFT, padx=5, pady=10)

pin_button = ttk.Button(
    root, text="Set PIN", state=tk.DISABLED, command=set_pin
)
pin_button.pack(side=tk.LEFT, padx=5, pady=10)

about_button = ttk.Button(root, text="About", command=show_about_message)
about_button.pack(side=tk.RIGHT, padx=5, pady=10)

root.mainloop()
