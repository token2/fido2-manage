import os
import re
import subprocess
import sys
import tempfile
import tkinter as tk
from tkinter import messagebox, simpledialog, ttk



# --- Path Resolution for fido2-token2 Binary ---
def get_fido2_binary_path():
    """
    Determines the absolute path to the fido2-token2 binary, whether running
    as a script or as a bundled pyinstaller app.
    """
    binary_name = "fido2-token2"

    if getattr(sys, 'frozen', False):
        # We are running in a PyInstaller bundle
        if hasattr(sys, '_MEIPASS'):
            # PyInstaller bundle with temporary directory
            base_path = sys._MEIPASS
        else:
            # PyInstaller one-file bundle
            base_path = os.path.dirname(sys.executable)

        # Try direct path first
        binary_path = os.path.join(base_path, binary_name)
        if os.path.exists(binary_path):
            return binary_path

        # Try alternative locations in the bundle
        for alt_path in [
            os.path.join(base_path, "Contents", "MacOS", binary_name),
            os.path.join(base_path, "Contents", "Frameworks", binary_name),
            os.path.join(base_path, "MacOS", binary_name),
            os.path.join(base_path, "Frameworks", binary_name),
            os.path.join(os.path.dirname(base_path), binary_name),
            os.path.join(os.path.dirname(base_path), "MacOS", binary_name),
            os.path.join(os.path.dirname(base_path), "Frameworks", binary_name)
        ]:
            if os.path.exists(alt_path):
                return alt_path

        return binary_path  # Return the expected path even if not found
    else:
        # We are running in a normal Python environment for development.
        script_dir = os.path.dirname(os.path.abspath(__file__))

        # Try various development locations
        dev_paths = [
            os.path.join(script_dir, "build", "staging", binary_name),
            os.path.join(script_dir, "staging", binary_name),
            os.path.join(script_dir, "build", "tools", binary_name),
            os.path.join(script_dir, "tools", binary_name),
            os.path.join(script_dir, binary_name)
        ]

        for path in dev_paths:
            if os.path.exists(path):
                return path

        # Fallback to PATH lookup
        print(binary_name)
        return binary_name

# Define the command to execute directly
FIDO2_TOKEN_CMD = get_fido2_binary_path()




# Checks the terminal emulator from which "gui.py" is executed
# and sets it for the subprocess commands
TERM = os.environ.get("TERM", "x-terminal-emulator")

# Command below for Windows
#FIDO2_TOKEN_CMD = "libfido2-ui.exe"

# Global variables
PIN = None
device_strings = []  # Store actual device strings for fido2-token2


def get_device_list():
    """Get device list directly from fido2-token2"""
    global device_strings
    try:
        # Execute fido2-token2 -L to list devices
        result = subprocess.run([FIDO2_TOKEN_CMD, "-L"], capture_output=True, text=True)
        
        if result.returncode != 0:
            print(f"Error executing {FIDO2_TOKEN_CMD} -L: {result.stderr}")
            return []
        
        device_list = []
        device_strings = []
        device_count = 1
        
        for line in result.stdout.strip().split('\n'):
            if line.strip() and ':' in line:
                # Handle pcsc devices specially
                if 'pcsc://slot0:' in line:
                    device_string = "pcsc://slot0"
                    device_description = "pcsc://slot0"
                else:
                    # Extract device string - everything before the first space or tab
                    parts = line.split()
                    if parts:
                        device_string = parts[0]
                        # For ioreg devices, ensure we don't have trailing colon
                        if device_string.endswith(':'):
                            device_string = device_string[:-1]
                    else:
                        # Fallback: split by colon and reconstruct
                        colon_parts = line.split(':')
                        if len(colon_parts) >= 2:
                            device_string = ':'.join(colon_parts[:2])
                        else:
                            device_string = line.strip()
                    
                    # Extract device description for display
                    match = re.search(r'\(([^)]+)\)', line)
                    if match:
                        device_description = match.group(1)
                    else:
                        device_description = device_string
                
                device_strings.append(device_string)
                device_list.append(f"Device [{device_count}] : {device_description}")
                device_count += 1
        
        return device_list
    
    except Exception as e:
        print(f"Error executing device list command: {e}")
        return []


def get_device_string(device_digit):
    """Get the actual device string for fido2-token2 command"""
    try:
        device_index = int(device_digit) - 1
        if 0 <= device_index < len(device_strings):
            return device_strings[device_index]
        return None
    except (ValueError, IndexError):
        return None


def execute_storage_command(device_digit):
    """Execute storage command directly with fido2-token2"""
    global PIN
    device_string = get_device_string(device_digit)
    if not device_string:
        messagebox.showerror("Error", "Invalid device selection")
        return
    
    # Build command with PIN option if provided
    command = [FIDO2_TOKEN_CMD, "-I", "-c"]
    if PIN and PIN != "0000":
        command.extend(["-w", PIN])
    command.append(device_string)
    
    try:
        result = subprocess.run(command, capture_output=True, text=True)
        
        if result.returncode == 0:
            # Parse the output and insert into the treeview
            for line in reversed(result.stdout.splitlines()):
                if ": " in line:
                    key, value = line.split(": ", 1)
                    tree.insert("", 0, values=(key, value))
        else:
            raise subprocess.CalledProcessError(result.returncode, command)
    except Exception as e:
        messagebox.showerror("Error", f"Command execution failed: {e}\nOutput: {result.stderr}")


def execute_info_command(device_digit):
    """Execute info command directly with fido2-token2"""
    global PIN
    tree.delete(*tree.get_children())
    
    device_string = get_device_string(device_digit)
    if not device_string:
        messagebox.showerror("Error", "Invalid device selection")
        return
    
    # First execute storage command
    storage_command = [FIDO2_TOKEN_CMD, "-I", "-c"]
    if PIN and PIN != "0000":
        storage_command.extend(["-w", PIN])
    storage_command.append(device_string)
    
    try:
        result = subprocess.run(storage_command, capture_output=True, text=True)
        
        # Check for specific FIDO errors
        if "FIDO_ERR_PIN_INVALID" in result.stderr:
            messagebox.showerror("Error", "Invalid PIN provided")
            return
        if "FIDO_ERR_INVALID_ARGUMENT" in result.stderr:
            messagebox.showerror("Error", "Invalid PIN provided")
            return
                

        if "FIDO_ERR_PIN_AUTH_BLOCKED" in result.stderr:
            messagebox.showerror("Error", "Wrong PIN provided too many times. Reinsert the key")
            return
        
        if "FIDO_ERR_PIN_REQUIRED" in result.stderr:
            messagebox.showerror("Error", 
                "No PIN set for this key. Passkeys can be managed only with a PIN set. "
                "You will be prompted to create a PIN on the next window")
            set_pin_command = [FIDO2_TOKEN_CMD, "-S", device_string]
            cmd_str = " ".join(set_pin_command)

        # macOS: use AppleScript to run in Terminal
            apple_script = f'''
            tell application "Terminal"
                activate
                do script "{cmd_str}"
            end tell
            '''
            subprocess.Popen(["osascript", "-e", apple_script])
            return
        
        if "FIDO_ERR_INVALID_CBOR" in result.stderr:
            messagebox.showerror("Error", 
                "This is an older key (probably FIDO2.0). No passkey management is possible "
                "with this key. Only basic information will be shown.")
        
        if "FIDO_ERR_INTERNAL" in result.stderr:
            messagebox.showerror("Error", 
                "Internal error communicating with the device. Please try unplugging and "
                "replugging the device, then refresh the device list.")
            return
        
        if result.returncode == 0:
            # Parse storage output
            for line in result.stdout.splitlines():
                if ": " in line:
                    key, value = line.split(": ", 1)
                    tree.insert("", tk.END, values=(key, value))
        else:
            raise subprocess.CalledProcessError(result.returncode, storage_command)
    
    except Exception as e:
        messagebox.showerror("Error", f"Storage command execution failed: {e}\nOutput: {result.stderr}")
        return
    
    # Now execute info command
    info_command = [FIDO2_TOKEN_CMD, "-I"]
    if PIN and PIN != "0000":
        info_command.extend(["-w", PIN])
    info_command.append(device_string)
    
    try:
        result = subprocess.run(info_command, capture_output=True, text=True)
        
        if result.returncode == 0:
            # Parse info output
            for line in result.stdout.splitlines():
                if ": " in line:
                    key, value = line.split(": ", 1)
                    tree.insert("", tk.END, values=(key, value))
        else:
            raise subprocess.CalledProcessError(result.returncode, info_command)
    
    except Exception as e:
        messagebox.showerror("Error", f"Info command execution failed: {e}\nOutput: {result.stderr}")


def set_pin():
    """Set the PIN global variable"""
    global PIN
    PIN = simpledialog.askstring(
        "PIN Code", "Enter PIN code (enter 0000 if no PIN is set/known):", show="*"
    )


def on_device_selected(event):
    """Handle device selection event"""
    global PIN
    selected_device = device_var.get()
    # Extract the digit inside the first pair of square brackets
    match = re.search(r"\[(\d+)\]", selected_device)
    PIN = None

    set_pin()
    
    if match:
        device_digit = match.group(1)
        if PIN is not None:
            execute_info_command(device_digit)
            check_passkeys_button_state()
            check_fingerprint_button_state()
            check_changepin_button_state()
    else:
        messagebox.showinfo("Device Selected", "No digit found in the selected device")



def check_fingerprint_button_state():
    """Check if the passkeys button should be enabled"""
    fp_button_state = tk.DISABLED

    # Assuming `tree` is a tkinter.ttk.Treeview widget
    for child in tree.get_children():
        values = tree.item(child, "values")
        if values and len(values) >= 1:
            # Check if 'fingerprint' is in any of the values
            if any("fingerprint" in str(value).lower() for value in values):
                fp_button_state = tk.NORMAL
                break

    fingerprints_button.config(state=fp_button_state)
    
    
def check_passkeys_button_state():
    """Check if the passkeys button should be enabled"""
    passkeys_button_state = tk.DISABLED
    for child in tree.get_children():
        values = tree.item(child, "values")
        if values and len(values) == 2 and values[0] == "existing rk(s)":
            try:
                rk_count = int(values[1])
                if rk_count > 0:
                    passkeys_button_state = tk.NORMAL
                    break
            except ValueError:
                pass
    
    passkeys_button.config(state=passkeys_button_state)


def check_changepin_button_state():
    """Check if the change PIN button should be enabled"""
    change_pin_button_state = tk.DISABLED
    for child in tree.get_children():
        values = tree.item(child, "values")
        if values and len(values) == 2 and values[0] == "remaining rk(s)":
            try:
                rk_count = int(values[1])
                if rk_count > 0:
                    change_pin_button_state = tk.NORMAL
                    break
            except ValueError:
                pass
    
    change_pin_button.config(state=change_pin_button_state)

#fp management start 

def show_message_and_lift(window, message, title="Info"):
    messagebox.showinfo(title, message)
    window.lift()  # Bring the window back to focus

def open_terminal(device_string, window):
    selected_device = device_var.get()
    show_message_and_lift(window, f"Opening a new terminal for {selected_device} with device: {device_string}")

def delete_selected(device_string, window):
    try:
        selected_index = listbox.curselection()
        selected_device = device_var.get()
        if selected_index:
            selected_item = listbox.get(selected_index)
            show_message_and_lift(window, f"Device: {selected_device}\nSelected: {selected_item}\nDevice String: {device_string}")
        else:
            show_message_and_lift(window, "No item selected.", "Warning")
    except Exception as e:
        messagebox.showerror("Error", f"An error occurred: {e}")
        window.lift()

def rename_selected(device_string, window):
    try:
        selected_index = listbox.curselection()
        selected_device = device_var.get()
        if selected_index:
            selected_item = listbox.get(selected_index)
            show_message_and_lift(window, f"Device: {selected_device}\nSelected: {selected_item}\nDevice String: {device_string}")
        else:
            show_message_and_lift(window, "No item selected.", "Warning")
    except Exception as e:
        messagebox.showerror("Error", f"An error occurred: {e}")
        window.lift()

def refresh_terminal(device_string, window):
    selected_device = device_var.get()
    show_message_and_lift(window, f"Refreshing the terminal for {selected_device} with device: {device_string}")
    update_fingerprint_list(device_string, window)

def update_fingerprint_list(device_string, window):
    try:
        selected_device = device_var.get()
        match = re.search(r"\[(\d+)\]", selected_device)

        if match:
            device_digit = match.group(1)
            if not device_string:
                messagebox.showerror("Error", "Invalid device selection")
                window.lift()
                return

            if PIN is not None:
                command = [FIDO2_TOKEN_CMD, "-L", "-e"]
                if PIN and PIN != "0000":
                    command.extend(["-w", PIN])
                command.append(device_string)

                result = subprocess.run(command, capture_output=True, text=True)

                if result.returncode == 0:
                    listbox.delete(0, tk.END)
                    fingerprints = result.stdout.strip().split('\n')
                    for fp in fingerprints:
                        listbox.insert(tk.END, fp)
                else:
                    messagebox.showerror("Error", f"Command failed with return code {result.returncode}")
                    window.lift()
    except Exception as e:
        messagebox.showerror("Error", f"An error occurred: {e}")
        window.lift()

def fingerprints():
    global PIN

    selected_device = device_var.get()
    match = re.search(r"\[(\d+)\]", selected_device)

    if match:
        device_digit = match.group(1)
        device_string = get_device_string(device_digit)

        if not device_string:
            messagebox.showerror("Error", "Invalid device selection")
            return

        if PIN is not None:
            command = [FIDO2_TOKEN_CMD, "-L", "-e"]
            if PIN and PIN != "0000":
                command.extend(["-w", PIN])
            command.append(device_string)

            try:
                result = subprocess.run(command, capture_output=True, text=True)

                if result.returncode == 0:
                    fingerprint_window = tk.Toplevel()
                    fingerprint_window.title("Fingerprints")

                    tk.Label(fingerprint_window, text=f"List of Fingerprints for {selected_device}:").pack(pady=10)

                    global listbox
                    listbox = tk.Listbox(fingerprint_window, width=50)
                    listbox.pack(padx=10, pady=10)

                    fingerprints = result.stdout.strip().split('\n')
                    for fp in fingerprints:
                        listbox.insert(tk.END, fp)

                    button_frame = tk.Frame(fingerprint_window)
                    button_frame.pack(pady=10)

                    add_button = tk.Button(button_frame, text="Add", command=lambda: open_terminal(device_string, fingerprint_window))
                    add_button.pack(side=tk.LEFT, padx=5)

                    delete_button = tk.Button(button_frame, text="Delete", command=lambda: delete_selected(device_string, fingerprint_window))
                    delete_button.pack(side=tk.LEFT, padx=5)

                    rename_button = tk.Button(button_frame, text="Rename", command=lambda: rename_selected(device_string, fingerprint_window))
                    rename_button.pack(side=tk.LEFT, padx=5)

                    refresh_button = tk.Button(button_frame, text="Refresh", command=lambda: refresh_terminal(device_string, fingerprint_window))
                    refresh_button.pack(side=tk.LEFT, padx=5)

                else:
                    messagebox.showerror("Error", f"Command failed with return code {result.returncode}")

            except Exception as e:
                messagebox.showerror("Error", f"An error occurred: {e}")

#fp management end





                
                
def on_passkeys_button_click():
    """Handle passkeys button click"""
    global PIN
    selected_device = device_var.get()
    match = re.search(r"\[(\d+)\]", selected_device)
    
    if match:
        device_digit = match.group(1)
        device_string = get_device_string(device_digit)
        
        if not device_string:
            messagebox.showerror("Error", "Invalid device selection")
            return
        
        if PIN is not None:
            # Execute command to get resident keys (list domains)
            command = [FIDO2_TOKEN_CMD, "-L", "-r"]
            if PIN and PIN != "0000":
                command.extend(["-w", PIN])
            command.append(device_string)
            
            try:
                result = subprocess.run(command, capture_output=True, text=True)
                if result.returncode == 0:
                    # Parse domains from output
                    domains = []
                    for line in result.stdout.splitlines():
                        match = re.search(r"= (.+)$", line)
                        if match:
                            domains.append(match.group(1))
                    
                    # Execute command for each domain
                    cumulated_output = []
                    for domain in domains:
                        domain_command = [FIDO2_TOKEN_CMD, "-L", "-k", domain]
                        if PIN and PIN != "0000":
                            domain_command.extend(["-w", PIN])
                        domain_command.append(device_string)
                        
                        domain_result = subprocess.run(domain_command, capture_output=True, text=True)
                        
                        if domain_result.returncode == 0:
                            # Process the output line by line
                            processed_lines = []
                            for line in domain_result.stdout.splitlines():
                                if line.strip():
                                    parts = line.split()
                                    if len(parts) >= 3:
                                        key_id = parts[0]
                                        credential_id = parts[1]
                                        user_field = " ".join(parts[2:4]) if len(parts) > 3 else parts[2]
                                        email_field = " ".join(parts[4:6]) if len(parts) > 5 else ""
                                        
                                        if user_field == "(null)":
                                            user_field = ""
                                        
                                        # Determine if user_field is an email
                                        if "@" in user_field:
                                            email = user_field
                                            user = ""
                                        else:
                                            user = user_field
                                            email = email_field
                                        
                                        processed_lines.append(f"Credential ID: {credential_id}, User: {user} {email}")
                            
                            cumulated_output.append(f"Domain: {domain}\n" + "\n".join(processed_lines))
                        else:
                            raise subprocess.CalledProcessError(domain_result.returncode, domain_command)
                    
                    # Show cumulated output in new window
                    cumulated_output_str = "\n\n".join(cumulated_output)
                    show_output_in_new_window(cumulated_output_str, device_digit)
                else:
                    raise subprocess.CalledProcessError(result.returncode, command)
            
            except Exception as e:
                messagebox.showerror("Error", f"Command execution failed: {e}\nOutput: {result.stderr}")
    else:
        messagebox.showinfo("Device Selected", "No digit found in the selected device")


def change_pin():
    """Change PIN using direct fido2-token2 command"""
    selected_device = device_var.get()
    match = re.search(r"\[(\d+)\]", selected_device)
    
    if match:
        device_digit = match.group(1)
        device_string = get_device_string(device_digit)
        
        if not device_string:
            messagebox.showerror("Error", "Invalid device selection")
            return
        
        command = [FIDO2_TOKEN_CMD, "-C", device_string]
        cmd_str = " ".join(command)

        # macOS: use AppleScript to run in Terminal
        apple_script = f'''
        tell application "Terminal"
            activate
            do script "{cmd_str}"
        end tell
        '''
        subprocess.Popen(["osascript", "-e", apple_script])
        
 


def refresh_combobox():
    """Refresh the device combobox"""
    device_combobox.set("")
    tree.delete(*tree.get_children())
    passkeys_button.config(state=tk.DISABLED)
    change_pin_button.config(state=tk.DISABLED)
    
    device_list = get_device_list()
    if not device_list:
        print("No devices found.")
    device_combobox["values"] = device_list


def show_output_in_new_window(output, device_digit):
    """Show output in a new window for passkey management"""
    new_window = tk.Toplevel(root)
    new_window.geometry("800x650")
    new_window.title("Resident Keys / Passkeys")

    # Create Treeview for displaying output
    tree_new_window = ttk.Treeview(
        new_window, columns=("Domain", "Credential ID", "User"), show="headings"
    )
    tree_new_window.heading("Domain", text="Domain")
    tree_new_window.heading("Credential ID", text="Credential ID")
    tree_new_window.heading("User", text="User")
    tree_new_window.pack(expand=True, fill=tk.BOTH, padx=10, pady=10)

    # Add scrollbars
    tree_scrollbar_y = ttk.Scrollbar(new_window, orient="vertical", command=tree_new_window.yview)
    tree_scrollbar_y.pack(side="right", fill="y")
    tree_new_window.configure(yscrollcommand=tree_scrollbar_y.set)
    
    tree_scrollbar_x = ttk.Scrollbar(new_window, orient="horizontal", command=tree_new_window.xview)
    tree_scrollbar_x.pack(side="bottom", fill="x")
    tree_new_window.configure(xscrollcommand=tree_scrollbar_x.set)

    # Parse output and insert into Treeview
    current_domain = ""
    for line in output.splitlines():
        if line.startswith("Domain: "):
            current_domain = line.split("Domain: ")[1].strip()
        elif "Credential ID: " in line and "User: " in line:
            credential_id = line.split("Credential ID: ")[1].split(",")[0].strip()
            user = line.split("User: ")[1].strip()
            user = re.sub(re.escape(credential_id), "", user).strip()
            tree_new_window.insert("", tk.END, values=(current_domain, credential_id, user))

    def show_selected_value():
        """Delete selected passkey"""
        selected_item = tree_new_window.selection()
        if selected_item:
            credential_id = tree_new_window.item(selected_item, "values")[1]
            device_string = get_device_string(device_digit)
            
            if not device_string:
                messagebox.showerror("Error", "Invalid device selection")
                return
            
            new_window.destroy()
            command = [FIDO2_TOKEN_CMD, "-D", "-i", credential_id, device_string]
            cmd_str = " ".join(command)
  # macOS: use AppleScript to run in Terminal
            apple_script = f'''
            tell application "Terminal"
                activate
                do script "{cmd_str}"
            end tell
            '''
            subprocess.Popen(["osascript", "-e", apple_script])
            
            
    # Create delete button
    show_value_button = tk.Button(new_window, text="delete passkey", command=show_selected_value)
    show_value_button.pack(pady=10)


def show_about_message():
    """Show about dialog"""
    messagebox.showinfo(
        "About",
        "The FIDO2.1 Security Key Management Tool is a utility designed to manage and interact with FIDO2.1 security keys.\r\n"
        "It provides functionalities to view information, manage relying parties, and perform various operations on connected FIDO2.1 devices.\r\n\r\n"
        "(c)TOKEN2 Sarl\r\nVersoix, Switzerland"
    )


def factory_reset():
    # Step 1: Confirm factory reset
    confirm_reset = messagebox.askyesno(
        "Confirm Factory Reset",
        "Are you sure you want to factory reset?"
    )
    if not confirm_reset:
        return

    # Step 2: Ask to unplug and replug the key
    replug_key = messagebox.askyesno(
        "Replug Key",
        "Please unplug the key, plug it back in, then click Yes to continue. Important - the reset command can be completed only within 10 seconds after plugging the key"
    )
    if not replug_key:
        return

    # Step 3: Tell user to touch key when it starts blinking
    messagebox.showinfo(
        "Touch Key",
        "When the key starts blinking, touch the sensor to complete reset."
    )

    try:
        # Run -L to list the device
        list_command = [FIDO2_TOKEN_CMD, "-L"]
        result = subprocess.run(list_command, capture_output=True, text=True, check=True)
        output = result.stdout.strip()

        # Extract string before first colon (e.g., ioreg://4302783856)
        match = re.search(r"(ioreg://\d+):", output)
        if not match:
            messagebox.showerror("Error", "No valid device found in output")
            return

        device_string = match.group(1)

        # Run -R with extracted device string
        reset_command = [FIDO2_TOKEN_CMD, "-R", device_string]
        subprocess.run(reset_command, check=True)

        messagebox.showinfo("Success", "Factory reset command sent successfully.")

    except subprocess.CalledProcessError as e:
        messagebox.showerror("Error", f"Command failed:\n{e.stderr}")
    except Exception as e:
        messagebox.showerror("Error", f"Failed to reset: {e}")

def terminal_path():
    bash_script = """#!/bin/bash
# CLI installer for FIDO2 Manager

set -e

APP_PATH="/Applications/fido2-manage.app"
CLI_SCRIPT="$APP_PATH/Contents/MacOS/fido2-manage-mac.sh"
SYMLINK_TARGET="/usr/local/bin/fido2-manage"

echo "=== FIDO2 Manager CLI Installer ==="
echo ""

if [[ ! -f "$CLI_SCRIPT" ]]; then
    echo "❌ FIDO2 Manager app not found at: $APP_PATH"
    echo "Please install the app first by dragging it to Applications folder."
    exit 1
fi

echo "✅ Found FIDO2 Manager app"

if [[ -L "$SYMLINK_TARGET" ]]; then
    existing_target=$(readlink "$SYMLINK_TARGET")
    if [[ "$existing_target" == "$CLI_SCRIPT" ]]; then
        echo "✅ CLI already installed - fido2-manage command is available"
        echo ""
        echo "Test it: fido2-manage -help"
        exit 0
    else
        echo "⚠️  Existing fido2-manage command found pointing to: $existing_target"
        echo "Do you want to replace it? (y/N)"
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            echo "Installation cancelled."
            exit 1
        fi
        sudo rm -f "$SYMLINK_TARGET"
    fi
elif [[ -f "$SYMLINK_TARGET" ]]; then
    echo "⚠️  Existing fido2-manage file found (not a symlink)"
    echo "Do you want to replace it? (y/N)"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 1
    fi
    sudo rm -f "$SYMLINK_TARGET"
fi

echo "Creating symlink: $SYMLINK_TARGET -> $CLI_SCRIPT"
if sudo ln -sf "$CLI_SCRIPT" "$SYMLINK_TARGET"; then
    echo "✅ CLI installed successfully!"
    echo ""
    echo "You can now use: fido2-manage -help"
    echo "Examples:"
    echo "  fido2-manage -list"
    echo "  fido2-manage -info -device 1"
    echo "  fido2-manage -storage -device 1"
    echo ""
else
    echo "❌ Failed to create symlink. You may need administrator privileges."
    exit 1
fi

echo ""
read -p "Press Enter to close this window..."
"""

    # Save script to a temp file
    with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.sh') as f:
        f.write(bash_script)
        script_path = f.name

    os.chmod(script_path, 0o755)

    # Escape the path for AppleScript
    escaped_path = script_path.replace(" ", "\\ ")

    # AppleScript to run in Terminal
    applescript = f'''
    tell application "Terminal"
        activate
        do script "{escaped_path}"
    end tell
    '''

    subprocess.run(["osascript", "-e", applescript])



# Create main application window
root = tk.Tk()
root.geometry("800x600")
root.title("FIDO2.1 Manager - Python GUI 0.2 - (c) Token2 ")

# Create top frame for controls
top_frame = ttk.Frame(root)
top_frame.pack(side=tk.TOP, fill=tk.X)

# Create label for dropdown
label = tk.Label(top_frame, text="Select Device:")
label.pack(side=tk.LEFT, padx=10, pady=10)

# Create ComboBox and populate with device list
device_list = get_device_list()
if not device_list:
    device_list = ["No devices found."]

device_var = tk.StringVar()
device_combobox = ttk.Combobox(top_frame, textvariable=device_var, values=device_list, width=60)
device_combobox.pack(side=tk.LEFT, padx=10, pady=10)
device_combobox.bind("<<ComboboxSelected>>", on_device_selected)

# Create refresh button
refresh_button = tk.Button(top_frame, text="Refresh", command=refresh_combobox)
refresh_button.pack(side=tk.LEFT, padx=10, pady=10)

# Create Treeview for displaying output
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

# Create buttons
passkeys_button = ttk.Button(root, text="Passkeys", state=tk.DISABLED, command=on_passkeys_button_click)
passkeys_button.pack(side=tk.LEFT, padx=5, pady=10)

change_pin_button = ttk.Button(root, text="Change PIN", state=tk.DISABLED, command=change_pin)
change_pin_button.pack(side=tk.LEFT, padx=5, pady=10)

fingerprints_button = tk.Button(root, text="fingerprints", state=tk.DISABLED, command=fingerprints )
fingerprints_button.pack(side=tk.LEFT, padx=10, pady=10)

terminal_path = tk.Button(root, text="fido2-manage CLI", command=terminal_path )
terminal_path.pack(side=tk.LEFT, padx=15, pady=10)

about_button = ttk.Button(root, text="About", command=show_about_message)
about_button.pack(side=tk.RIGHT, padx=5, pady=10)

# Run the main loop
root.mainloop()
