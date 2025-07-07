import os
import re
import subprocess
import sys
import tkinter as tk
from tkinter import messagebox, simpledialog, ttk

# --- Path Resolution for pyinstaller Bundle ---
def get_fido_command_path():
    """
    Determines the absolute path to the fido2-token2 binary, whether running
    as a script or as a bundled pyinstaller app. Enhanced for self-contained builds.
    """
    if getattr(sys, 'frozen', False):
        # We are running in a PyInstaller bundle
        if hasattr(sys, '_MEIPASS'):
            # PyInstaller bundle with temporary directory
            base_path = sys._MEIPASS
        else:
            # PyInstaller one-file bundle
            base_path = os.path.dirname(sys.executable)

        # Print debug info for troubleshooting
        print(f"[DEBUG] Running in bundle mode, base_path: {base_path}")
        print(f"[DEBUG] Contents of base_path: {os.listdir(base_path) if os.path.exists(base_path) else 'NOT FOUND'}")

        # Primary locations to check for the fido2-token2 binary
        binary_locations = [
            os.path.join(base_path, "fido2-token2"),
            os.path.join(base_path, "Contents", "MacOS", "fido2-token2"),
            os.path.join(base_path, "Contents", "Frameworks", "fido2-token2"),
            os.path.join(base_path, "MacOS", "fido2-token2"),
            os.path.join(base_path, "Frameworks", "fido2-token2"),
            os.path.join(os.path.dirname(base_path), "fido2-token2"),
            os.path.join(os.path.dirname(base_path), "Frameworks", "fido2-token2"),
            os.path.join(os.path.dirname(base_path), "MacOS", "fido2-token2")
        ]

        for binary_path in binary_locations:
            print(f"[DEBUG] Checking for binary: {binary_path}")
            if os.path.exists(binary_path):
                print(f"[DEBUG] Found fido2-token2 binary at: {binary_path}")
                return binary_path

        # Final fallback - return first location even if not found
        print(f"[DEBUG] Binary not found, returning default: {binary_locations[0]}")
        return binary_locations[0]
    else:
        # We are running in a normal Python environment for development.
        # Look for fido2-token2 binary in common development locations
        script_dir = os.path.dirname(os.path.abspath(__file__))
        
        dev_locations = [
            os.path.join(script_dir, "build", "tools", "fido2-token2"),
            os.path.join(script_dir, "tools", "fido2-token2"),
            os.path.join(script_dir, "fido2-token2"),
            "./build/tools/fido2-token2",
            "./tools/fido2-token2", 
            "./fido2-token2",
            "fido2-token2"  # System PATH lookup
        ]
        
        for binary_path in dev_locations:
            if os.path.exists(binary_path):
                print(f"[DEBUG] Found development binary at: {binary_path}")
                return binary_path

        # Fallback to system-wide installation
        print(f"[DEBUG] Using system PATH lookup for fido2-token2")
        return "fido2-token2"

print("FIDO2 Manager GUI starting...")
FIDO_COMMAND = get_fido_command_path()
print(f"FIDO2 Command Path: {FIDO_COMMAND}")
print(f"Script exists: {os.path.exists(FIDO_COMMAND)}")

def get_device_number_from_string(device_string):
    """
    Extract device number from device string format "Device [1] : SoloKeys"
    """
    import re
    match = re.search(r'Device \[(\d+)\]', device_string)
    if match:
        return match.group(1)
    return "1"  # Default to device 1
# --- End of Path Resolution ---

# Global variable to store the PIN for the current session
PIN = None

# --- Core Functions ---

def get_device_list():
    """Gets the list of connected FIDO devices by calling the C binary directly."""
    try:
        if not os.path.exists(FIDO_COMMAND):
            messagebox.showerror("Dependency Error", f"Required tool not found at: {FIDO_COMMAND}")
            return []

        # Call fido2-token2 -L directly to list devices
        result = subprocess.run([FIDO_COMMAND, "-L"], capture_output=True, text=True, check=True)
        
        # Parse the output to format like the shell script
        device_list = []
        device_count = 1
        
        for line in result.stdout.strip().split("\n"):
            if line.strip():
                # Extract device info (look for parentheses with device type)
                import re
                match = re.search(r'\(([^)]+)\)', line)
                if match:
                    device_type = match.group(1)
                    device_list.append(f"Device [{device_count}] : {device_type}")
                    device_count += 1
                else:
                    # Fallback if no parentheses found
                    device_list.append(f"Device [{device_count}] : {line.strip()}")
                    device_count += 1
        
        return device_list
    except Exception as e:
        error_message = f"Error getting device list: {e}\n\nDetails:\n{getattr(e, 'stderr', '')}"
        messagebox.showerror("Execution Error", error_message)
        return []

def get_device_path_by_number(device_number):
    """Get the actual device path string for a given device number."""
    try:
        # Get device list from fido2-token2 -L
        result = subprocess.run([FIDO_COMMAND, "-L"], capture_output=True, text=True, check=True)
        lines = [line.strip() for line in result.stdout.split('\n') if line.strip()]
        
        # Convert device_number to index (1-based to 0-based)
        device_index = int(device_number) - 1
        
        if 0 <= device_index < len(lines):
            device_line = lines[device_index]
            # Extract device path (everything before the first colon and space)
            # Handle both "path: info" and "path:info" formats
            if ':' in device_line:
                # Split on ':' and take first part, but handle "pcsc://slot0:" specially
                if device_line.startswith('pcsc://'):
                    return "pcsc://slot0"
                else:
                    # For other formats like "/dev/hidraw0: ..."
                    parts = device_line.split(':', 1)
                    return parts[0].strip()
            else:
                # If no colon, return the whole line
                return device_line
        
        return None
    except Exception as e:
        print(f"[ERROR] Failed to get device path: {e}")
        return None

def execute_info_command(device_string):
    """Fetches and displays device info in the treeview."""
    tree.delete(*tree.get_children())
    # Extract device number from device_string
    device_number = get_device_number_from_string(device_string)
    
    # Get the actual device path for this device number
    device_path = get_device_path_by_number(device_number)
    if not device_path:
        messagebox.showerror("Error", f"Could not find device path for device {device_number}")
        return
    
    print(f"[DEBUG] Using device path: {device_path}")
    
    # Build PIN arguments for fido2-token2 (-w pin)
    pin_args = ["-w", PIN] if PIN else []

    # Commands to run with fido2-token2 directly
    commands_to_run = {
        "info": [FIDO_COMMAND, "-I"] + pin_args + [device_path],
        "storage": [FIDO_COMMAND, "-I", "-c"] + pin_args + [device_path],
    }

    for key, command in commands_to_run.items():
        try:
            print(f"[DEBUG] Running command: {' '.join(command)}")
            result = subprocess.run(command, capture_output=True, text=True)

            if result.returncode != 0:
                print(f"[DEBUG] Command {key} failed with code {result.returncode}")
                print(f"[DEBUG] stderr: {result.stderr}")
                
                if "FIDO_ERR_PIN_INVALID" in result.stderr:
                    messagebox.showerror("Error", "Invalid PIN provided.")
                    return # Stop processing on bad PIN
                if "FIDO_ERR_PIN_REQUIRED" in result.stderr:
                    messagebox.showerror("Error", "A PIN is required for this operation.")
                    return # Stop processing
                continue # Try next command even if one fails for other reasons

            # Parse output and add to tree
            for line in result.stdout.splitlines():
                if ": " in line:
                    k, v = line.split(": ", 1)
                    tree.insert("", tk.END, values=(k.strip(), v.strip()))
                elif line.strip():
                    # For lines without colon, add as single value
                    tree.insert("", tk.END, values=(line.strip(), ""))
                    
        except Exception as e:
            messagebox.showerror("Error", f"Command '{key}' failed: {e}\nOutput: {getattr(e, 'stderr', '')}")

    check_passkeys_button_state()
    check_changepin_button_state()

# --- UI Event Handlers ---

def set_pin_and_get_info(device_string):
    """Prompts for PIN and then fetches device info."""
    global PIN
    PIN = simpledialog.askstring("PIN Code", "Enter PIN code:", show="*")
    if PIN is not None: # Proceed even if PIN is empty, but not if cancelled
        execute_info_command(device_string)

def on_device_selected(event):
    """Handles the device selection from the dropdown."""
    selected_text = device_var.get()
    # Robust regex to find device paths on Linux, macOS, or Windows
    match = re.search(r"(/dev/\w+)|(pcsc://\S+)|(windows://\S+)|(ioreg://\S+)", selected_text)
    if match:
        device_string = match.group(0).split(':')[0] # Get the path part before any colon
        set_pin_and_get_info(device_string)
    else:
        messagebox.showerror("Device Error", "Could not parse device path from selection.")

def refresh_combobox():
    """Refreshes the device list in the dropdown."""
    device_list = get_device_list()
    if not device_list:
        device_combobox['values'] = ["No FIDO devices found."]
    else:
        device_combobox['values'] = device_list
    device_combobox.set("")
    tree.delete(*tree.get_children())
    passkeys_button.config(state=tk.DISABLED)
    change_pin_button.config(state=tk.DISABLED)

def show_about_message():
    """Displays the about dialog."""
    messagebox.showinfo(
        "About",
        "FIDO2 Manager\n\n"
        "A utility to manage and interact with FIDO2 security keys.\n\n"
        "(c) TOKEN2 Sàrl, Yubico AB"
    )

def check_passkeys_button_state():
    """Enables 'Passkeys' button if resident keys exist."""
    state = tk.DISABLED
    for child in tree.get_children():
        values = tree.item(child, "values")
        if values and len(values) > 1 and values[0] == "existing rk(s)":
            try:
                if int(values[1]) > 0:
                    state = tk.NORMAL
                break
            except (ValueError, IndexError):
                pass
    passkeys_button.config(state=state)

def check_changepin_button_state():
    """Enables 'Change PIN' button if PIN is set."""
    state = tk.DISABLED
    for child in tree.get_children():
        values = tree.item(child, "values")
        if values and len(values) > 1 and values[0] == "pin retries":
             state = tk.NORMAL
             break
    change_pin_button.config(state=state)

# Placeholder functions for buttons
def on_passkeys_button_click():
    messagebox.showinfo("Not Implemented", "Passkey management functionality is not yet implemented.")

def change_pin():
    messagebox.showinfo("Not Implemented", "Change PIN functionality is not yet implemented.")

# --- GUI Layout ---

root = tk.Tk()
root.title("FIDO2 Manager")
root.geometry("800x600")
root.minsize(700, 400)

# Main container
main_frame = ttk.Frame(root, padding="10")
main_frame.pack(expand=True, fill="both")

# --- Top Frame: Device Selection & Refresh ---
top_frame = ttk.Frame(main_frame)
top_frame.pack(side="top", fill="x", pady=(0, 10))

ttk.Label(top_frame, text="Select Device:").pack(side="left")

device_var = tk.StringVar()
device_combobox = ttk.Combobox(top_frame, textvariable=device_var, state="readonly")
device_combobox.pack(side="left", expand=True, fill="x", padx=5)
device_combobox.bind("<<ComboboxSelected>>", on_device_selected)

refresh_button = ttk.Button(top_frame, text="Refresh", command=refresh_combobox)
refresh_button.pack(side="left")

# --- Center Frame: Information Treeview ---
tree_frame = ttk.Frame(main_frame)
tree_frame.pack(expand=True, fill="both")

tree = ttk.Treeview(tree_frame, columns=("Key", "Value"), show="headings")
tree.heading("Key", text="Key")
tree.heading("Value", text="Value")
tree.column("Key", width=200, stretch=tk.NO, anchor="w")
tree.column("Value", width=550, anchor="w")

# Scrollbars
vsb = ttk.Scrollbar(tree_frame, orient="vertical", command=tree.yview)
hsb = ttk.Scrollbar(tree_frame, orient="horizontal", command=tree.xview)
tree.configure(yscrollcommand=vsb.set, xscrollcommand=hsb.set)

vsb.pack(side="right", fill="y")
hsb.pack(side="bottom", fill="x")
tree.pack(side="left", expand=True, fill="both")

# --- Bottom Frame: Action Buttons ---
bottom_frame = ttk.Frame(main_frame)
bottom_frame.pack(side="bottom", fill="x", pady=(10, 0))

passkeys_button = ttk.Button(bottom_frame, text="Passkeys", state=tk.DISABLED, command=on_passkeys_button_click)
passkeys_button.pack(side="left")

change_pin_button = ttk.Button(bottom_frame, text="Change PIN", state=tk.DISABLED, command=change_pin)
change_pin_button.pack(side="left", padx=5)

about_button = ttk.Button(bottom_frame, text="About", command=show_about_message)
about_button.pack(side="right")

# --- Initial Load ---
refresh_combobox()
root.mainloop()
