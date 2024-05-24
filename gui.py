import subprocess
import sys
import tkinter as tk
from tkinter import ttk, messagebox, simpledialog
import re

# Define the command to execute
FIDO_COMMAND = './fido2-manage.sh'
#Command below for Windows
#FIDO_COMMAND = 'fido2-manage-ui.exe'
# Global variable to store the PIN
pin = None

# Function to get device list from fido2-manage-ui.exe
def get_device_list():
     
    try:
        # Execute the command with '-list' argument and capture the output
        result = subprocess.run([FIDO_COMMAND, '-list'], capture_output=True, text=True)
        # Split the output into lines and return as a list
        device_list = result.stdout.strip().split('\n')
         
        return device_list
        
    except Exception as e:
        # Handle exceptions (e.g., file not found or command error)
        print(f"Error executing device list command: {e}")
         
        return []

# Function to execute storage command and prepend its output to the grid
def execute_storage_command(device_digit):
    global pin
    command = [FIDO_COMMAND, '-storage', '-pin', pin, '-device', device_digit]
    
    try:
        result = subprocess.run(command, capture_output=True, text=True)
        # Check if the subprocess was executed successfully
        #print (result)
        if result.returncode == 0:
            # Parse the output and insert into the treeview
            for line in reversed(result.stdout.splitlines()):  # Insert in reversed order to prepend
                if ": " in line:
                    key, value = line.split(": ", 1)
                    tree.insert("", 0, values=(key, value))  # Insert at the top of the grid
    
        else:
            raise subprocess.CalledProcessError(result.returncode, command)
    except Exception as e:
        messagebox.showerror("Error", f"Command execution failed: {e}\nOutput: {result.stderr}")

# Function to execute info command and append its output to the grid
def execute_info_command(device_digit):
    global pin
    tree.delete(*tree.get_children())
    command = [FIDO_COMMAND, '-storage', '-pin', pin, '-device', device_digit]
    try:
        result = subprocess.run(command, capture_output=True, text=True)
         
        if result.stderr.find("FIDO_ERR_PIN_INVALID")!= -1:
            #exit
            messagebox.showerror("Error", f"Invalid PIN provided")
            return
            
# Check FIDO_ERR_PIN_AUTH_BLOCKED            
        if result.stderr.find("FIDO_ERR_PIN_AUTH_BLOCKED")!= -1:
            #exit
            messagebox.showerror("Error", f"Wrong PIN provided to many times. Reinsert the key")
            return

# Check FIDO_ERR_PIN_REQUIRED            
        if result.stderr.find("FIDO_ERR_PIN_REQUIRED")!= -1:
            #exit
            messagebox.showerror("Error", f"No PIN set for this key. Passkeys can be managed only with a PIN set. You will be prompted to create a PIN on the next window")
            command = [FIDO_COMMAND, '-setPIN', '-device', device_digit]
            if sys.platform.startswith('win'):
                subprocess.Popen(['start', 'cmd', '/c'] + command, shell=True)
            elif sys.platform.startswith('linux'):
                subprocess.Popen(['x-terminal-emulator', '-e'] + command)

            return            


# Check FIDO version            
        if result.stderr.find("FIDO_ERR_INVALID_CBOR")!= -1:
            #exit
            messagebox.showerror("Error", f"This is an older key (probably FIDO2.0). No passkey management is possible with this key. Only basic information will be shown.")
             

                 
                     
        # Check if the subprocess was executed successfully
        if result.returncode == 0:
            # Parse the output and insert into the treeview
            for line in result.stdout.splitlines():
                if ": " in line:
                    key, value = line.split(": ", 1)
                    tree.insert("", tk.END, values=(key, value))  # Append to the end of the grid
        else:
            raise subprocess.CalledProcessError(result.returncode, command)
    except Exception as e:
        messagebox.showerror("Error", f"Command execution failed: {e}\nOutput: {result.stderr}")
    command = [FIDO_COMMAND, '-info', '-pin', pin, '-device', device_digit]
    try:
        result = subprocess.run(command, capture_output=True, text=True)
        # Check if the subprocess was executed successfully
        if result.returncode == 0:
            # Parse the output and insert into the treeview
            for line in result.stdout.splitlines():
                if ": " in line:
                    key, value = line.split(": ", 1)
                    tree.insert("", tk.END, values=(key, value))  # Append to the end of the grid
        else:
            raise subprocess.CalledProcessError(result.returncode, command)
    except Exception as e:
        messagebox.showerror("Error", f"Command execution failed: {e}\nOutput: {result.stderr}")


# Function to set the PIN
def set_pin():
    global pin
    pin = simpledialog.askstring("PIN Code", "Enter PIN code (enter 0000 if no PIN is set/known):", show='*')



# Function to handle selection event
def on_device_selected(event):
    global pin
    selected_device = device_var.get()
    # Extract the digit inside the first pair of square brackets
    match = re.search(r'\[(\d+)\]', selected_device)
    pin = None

    set_pin()
    #print (pin)
    if match:
        device_digit = match.group(1)
        
        if pin is not None:
            execute_info_command(device_digit)
            check_passkeys_button_state()
            check_changepin_button_state()
    else:
        messagebox.showinfo("Device Selected", "No digit found in the selected device")

# Function to check if the "passkeys" button should be enabled
def check_passkeys_button_state():
    passkeys_button_state = tk.DISABLED
    for child in tree.get_children():
        values = tree.item(child, 'values')
        if values and len(values) == 2 and values[0] == 'existing rk(s)':
            try:
                rk_count = int(values[1])
                if rk_count > 0:
                    passkeys_button_state = tk.NORMAL
                    break
            except ValueError:
                pass
                
    passkeys_button.config(state=passkeys_button_state)


# Function to check if the "passkeys" button should be enabled
def check_changepin_button_state():
    passkeys_button_state = tk.DISABLED
    for child in tree.get_children():
        values = tree.item(child, 'values')
        if values and len(values) == 2 and values[0] == 'remaining rk(s)':
            try:
                rk_count = int(values[1])
                if rk_count > 0:
                    passkeys_button_state = tk.NORMAL
                    break
            except ValueError:
                pass
                
    change_pin_button.config(state=passkeys_button_state)
    
    




# Function to handle "passkeys" button click
def on_passkeys_button_click():
    global pin
    # Get the selected device and PIN
    selected_device = device_var.get()
    match = re.search(r'\[(\d+)\]', selected_device)
    if match:
        device_digit = match.group(1)
        #pin = simpledialog.askstring("PIN Code", "Enter PIN code (enter 0000 if no PIN is set/known):", show='*')
        if pin is not None:
            # Execute the command to get resident keys
            command = [FIDO_COMMAND, '-residentKeys', '-pin', pin, '-device', device_digit]
            try:
                result = subprocess.run(command, capture_output=True, text=True)
                if result.returncode == 0:
                    # Parse the domains from the output
                    domains = []
                    for line in result.stdout.splitlines():
                        match = re.search(r'= (.+)$', line)
                        if match:
                            domains.append(match.group(1))
                    
                    # Execute the command for each domain
                    cumulated_output = []
                    for domain in domains:
                        
                        domain_command = [FIDO_COMMAND, '-residentKeys', '-domain', domain, '-pin', pin, '-device', device_digit]
                        domain_result = subprocess.run(domain_command, capture_output=True, text=True)
                         
                        if domain_result.returncode == 0:
                            cumulated_output.append(f"Domain: {domain}\n{domain_result.stdout}")
                        else:
                            raise subprocess.CalledProcessError(domain_result.returncode, domain_command)

                    # Show the cumulated output in a new window
                    cumulated_output_str = "\n\n".join(cumulated_output)
                    show_output_in_new_window(cumulated_output_str, device_digit)
                else:
                    raise subprocess.CalledProcessError(result.returncode, command)
            except Exception as e:
                messagebox.showerror("Error", f"Command execution failed: {e}\nOutput: {result.stderr}")
    else:
        messagebox.showinfo("Device Selected", "No digit found in the selected device")

def change_pin():
     # Get the selected device and PIN
    selected_device = device_var.get()
    # Extract the digit inside the first pair of square brackets
    match = re.search(r'\[(\d+)\]', selected_device)
    if match:
        device_digit = match.group(1)
        command = [FIDO_COMMAND, '-changePIN', '-device', device_digit]
        if sys.platform.startswith('win'):
            subprocess.Popen(['start', 'cmd', '/c'] + command, shell=True)
        elif sys.platform.startswith('linux'):
            subprocess.Popen(['x-terminal-emulator', '-e'] + command)


        
        
    
    pass
        
        
def refresh_combobox():
    # Implement your refresh logic here
    # For example, you can update the values in the combobox
    # based on some external data source or trigger a refresh action.
    device_combobox.set("")  # Clear the selected value
    tree.delete(*tree.get_children())
    passkeys_button.config(state=tk.DISABLED)
    change_pin_button.config(state=tk.DISABLED)
    device_list = get_device_list()  # Assuming you have a function to get the device list
    if not device_list:
        print("No devices found.")
    device_combobox['values'] = device_list  # Update the combobox values
    
    
 
 

# Function to show the output in a new window
def show_output_in_new_window(output, device_digit):
    # Create a new window
    new_window = tk.Toplevel(root)
    new_window.geometry("700x650")
    new_window.title("Resident Keys / Passkeys")

    # Create a Treeview widget for displaying output
    tree_new_window = ttk.Treeview(new_window, columns=("Domain", "Credential ID", "User"), show="headings")
    # Set column headings
    tree_new_window.heading("Domain", text="Domain")
    tree_new_window.heading("Credential ID", text="Credential ID")
    tree_new_window.heading("User", text="User")
    tree_new_window.pack(expand=True, fill=tk.BOTH, padx=10, pady=10)

    # Add scrollbars to the Treeview
    tree_scrollbar_y = ttk.Scrollbar(new_window, orient="vertical", command=tree_new_window.yview)
    tree_scrollbar_y.pack(side="right", fill="y")
    tree_new_window.configure(yscrollcommand=tree_scrollbar_y.set)
    tree_scrollbar_x = ttk.Scrollbar(new_window, orient="horizontal", command=tree_new_window.xview)
    tree_scrollbar_x.pack(side="bottom", fill="x")
    tree_new_window.configure(xscrollcommand=tree_scrollbar_x.set)

    # Parse the output and insert into the Treeview
    current_domain = ""
    for line in output.splitlines():
        if line.startswith("Domain: "):
            current_domain = line.split("Domain: ")[1].strip()
        elif "Credential ID: " in line and "User: " in line:
            credential_id = line.split("Credential ID: ")[1].split(",")[0].strip()
            user = line.split("User: ")[1].strip()
            user = user.replace(credential_id, "").strip()
            tree_new_window.insert("", tk.END, values=(current_domain, credential_id, user))

    # Function to handle show value button click
    def show_selected_value():
        selected_item = tree_new_window.selection()
        if selected_item:
            value = tree_new_window.item(selected_item, 'values')[1]  # Get the Credential ID of the selected item
            new_window.destroy()
            command = [FIDO_COMMAND, '-delete', '-device', device_digit, '-credential', value]
            if sys.platform.startswith('win'):
                subprocess.Popen(['start', 'cmd', '/c'] + command, shell=True)
            elif sys.platform.startswith('linux'):
                subprocess.Popen(['x-terminal-emulator', '-e'] + command)

    # Create the "Show Value" button
    show_value_button = tk.Button(new_window, text="delete passkey", command=show_selected_value)
    show_value_button.pack(pady=10)





def show_about_message():
    messagebox.showinfo("About", "The FIDO2.1 Security Key Management Tool is a utility designed to manage and interact with FIDO2.1 security keys.\r\nIt provides functionalities to view information, manage relying parties, and perform various operations on connected FIDO2.1 devices.\r\n\r\n(c)TOKEN2 Sarl\r\nVersoix, Switzerland")
    

# Create the main application window
root = tk.Tk()
root.geometry("700x600")  # Width x Height
root.title("FIDO2.1 Manager - Python version 0.1 - (c) Token2")

# Create a frame for the first three elements
top_frame = ttk.Frame(root)
top_frame.pack(side=tk.TOP, fill=tk.X)

# Create a label for the dropdown
label = tk.Label(top_frame, text="Select Device:")
label.pack(side=tk.LEFT, padx=10, pady=10)

# Create a ComboBox (dropdown) and populate it with device list
device_list = get_device_list()
if not device_list:
    device_list = "No devices found."
device_var = tk.StringVar()
device_combobox = ttk.Combobox(top_frame, textvariable=device_var, values=device_list, width=60)
device_combobox.pack(side=tk.LEFT, padx=10, pady=10)
device_combobox.bind("<<ComboboxSelected>>", on_device_selected)
# Create the refresh button
refresh_button = tk.Button(top_frame, text="Refresh", command=refresh_combobox)
refresh_button.pack(side=tk.LEFT, padx=10, pady=10)


# Create a Treeview widget for displaying output with scrollbars
tree_frame = ttk.Frame(root)
tree_frame.pack(expand=True, fill=tk.BOTH, padx=10, pady=10)
tree_scrollbar_y = ttk.Scrollbar(tree_frame, orient="vertical")
tree_scrollbar_x = ttk.Scrollbar(tree_frame, orient="horizontal")
tree = ttk.Treeview(tree_frame, columns=("Key", "Value"), show="headings", yscrollcommand=tree_scrollbar_y.set, xscrollcommand=tree_scrollbar_x.set)
tree_scrollbar_y.config(command=tree.yview)
tree_scrollbar_x.config(command=tree.xview)
tree_scrollbar_y.pack(side="right", fill="y")
tree_scrollbar_x.pack(side="bottom", fill="x")
# Set column headings
tree.heading("Key", text="Key")
tree.heading("Value", text="Value")
tree.pack(expand=True, fill=tk.BOTH)

# Create the "passkeys" button
passkeys_button = ttk.Button(root, text="Passkeys", state=tk.DISABLED, command=on_passkeys_button_click)
passkeys_button.pack(side=tk.LEFT, padx=5, pady=10)

# Create the "Change PIN" button
change_pin_button = ttk.Button(root, text="Change PIN", state=tk.DISABLED, command=change_pin)
change_pin_button.pack(side=tk.LEFT, padx=5, pady=10)



about_button = ttk.Button(root, text="About", command=show_about_message)
about_button.pack(side=tk.RIGHT, padx=5, pady=10)




# Run the Tkinter main loop
root.mainloop()

