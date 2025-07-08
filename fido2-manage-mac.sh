#!/bin/bash

# Determine the path to fido2-token2 binary
# Check if we're running from a bundle or development environment

# Resolve the real script location (in case we're called via symlink)
SCRIPT_PATH="${BASH_SOURCE[0]}"
if [[ -L "$SCRIPT_PATH" ]]; then
    # We're called via symlink, resolve the real path
    REAL_SCRIPT_PATH="$(readlink "$SCRIPT_PATH")"
    if [[ "$REAL_SCRIPT_PATH" != /* ]]; then
        # Relative path, make it absolute
        REAL_SCRIPT_PATH="$(cd "$(dirname "$SCRIPT_PATH")" && cd "$(dirname "$REAL_SCRIPT_PATH")" && pwd)/$(basename "$REAL_SCRIPT_PATH")"
    fi
    SCRIPT_DIR="$(cd "$(dirname "$REAL_SCRIPT_PATH")" && pwd)"
    echo "[INFO] Script called via symlink, resolved to: $REAL_SCRIPT_PATH"
else
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

# Try bundled binary first (self-contained), then development build, then system installation
# Prioritize Frameworks directory for better library resolution
if [[ -f "$SCRIPT_DIR/../Frameworks/fido2-token2" ]]; then
    FIDO2_TOKEN_CMD="$SCRIPT_DIR/../Frameworks/fido2-token2"
    echo "[INFO] Using bundled fido2-token2 binary from Frameworks"
elif [[ -f "$SCRIPT_DIR/fido2-token2" ]]; then
    FIDO2_TOKEN_CMD="$SCRIPT_DIR/fido2-token2"
    echo "[INFO] Using bundled fido2-token2 binary"
elif [[ -f "$SCRIPT_DIR/../tools/fido2-token2" ]]; then
    FIDO2_TOKEN_CMD="$SCRIPT_DIR/../tools/fido2-token2"
    echo "[INFO] Using development build fido2-token2 binary"
elif [[ -f "$SCRIPT_DIR/build/tools/fido2-token2" ]]; then
    FIDO2_TOKEN_CMD="$SCRIPT_DIR/build/tools/fido2-token2"
    echo "[INFO] Using development build fido2-token2 binary"
else
    # Look in common bundle locations
    for bundle_path in \
        "$SCRIPT_DIR/../Frameworks/fido2-token2" \
        "$SCRIPT_DIR/../MacOS/fido2-token2" \
        "$SCRIPT_DIR/../Resources/fido2-token2" \
        "$(dirname "$SCRIPT_DIR")/fido2-token2" \
        "$(dirname "$SCRIPT_DIR")/Frameworks/fido2-token2"; do
        if [[ -f "$bundle_path" ]]; then
            FIDO2_TOKEN_CMD="$bundle_path"
            echo "[INFO] Using bundled fido2-token2 binary at $bundle_path"
            break
        fi
    done
    
    # Skip system installation fallback when called via CLI installer
    # (This prevents using the broken system binary with Homebrew dependencies)
    
    # Default fallback
    if [[ -z "$FIDO2_TOKEN_CMD" ]]; then
        FIDO2_TOKEN_CMD="fido2-token2"
        echo "[WARNING] fido2-token2 binary not found - using PATH lookup"
    fi
fi

echo "[INFO] FIDO2_TOKEN_CMD set to: $FIDO2_TOKEN_CMD"

list=false
info=false
device=""
pin=""
storage=false
residentKeys=false
domain=""
delete=false
credential=""
changePIN=false
setPIN=false
reset=false
uvs=false
uvd=false
fingerprint=false
help=false

show_message() {
    local message=$1
    local type=${2:-"Info"}
    echo "[$type] $message"
}

while [[ "$#" -gt 0 ]]; do
    case $1 in
        -list) list=true ;;
        -info) info=true ;;
        -device) device="$2"; shift ;;
        -pin) pin="$2"; shift ;;
        -storage) storage=true ;;
        -fingerprint) fingerprint=true ;;        
        -residentKeys) residentKeys=true ;;
        -domain) domain="$2"; shift ;;
        -delete) delete=true ;;
        -credential) credential="$2"; shift ;;
        -changePIN) changePIN=true ;;
        -setPIN) setPIN=true ;;
        -reset) reset=true ;;
        -uvs) uvs=true ;;
        -uvd) uvd=true ;;
        -help) help=true ;;
        *) show_message "Unknown parameter: $1" "Error"; exit 1 ;;
    esac
    shift
done



show_help() {
    cat << EOF
FIDO2 Token Management Tool
v 0.2.2
This is a wrapper for libfido2 library - modified for macOS

(c) Token2 Sarl

Usage: ./fido2-manage.sh [-list] [-info -device <number>] [-storage -device <number>] [-residentKeys -device <number> -domain <domain>] [-uvs] [-uvd] [-delete -device <number> -credential <credential>] [-help]

Examples:
- List available devices:
  ./fido2-manage.sh -list

- Retrieve information about a specific device:
  ./fido2-manage.sh -info -device 1

- Retrieve storage data for credentials (number of resident keys stored and available) on a specific device:
  ./fido2-manage.sh -storage -device 2

- Retrieve resident keys on a specific device for a domain:
  ./fido2-manage.sh -residentKeys -device 1 -domain login.microsoft.com

- Enforce user verification to be always requested on a specific device:
  ./fido2-manage.sh -uvs -device 1

- Disable enforcing user verification to be always requested on a specific device:
  ./fido2-manage.sh -uvd -device 1

- Sets PIN of a specific device:
  ./fido2-manage.sh -setPIN -device 1
  
- Enrolls a fingerprint to a specific device (biometric models only, simplified method - does not allow deleting fingerprints):
  ./fido2-manage.sh -fingerprint -device 1
  
- Perform a factory reset on a specific device:
  ./fido2-manage.sh -reset -device 1

- Change PIN of a specific device:
  ./fido2-manage.sh -changePIN -device 1

- Delete a credential on a specific device:
  ./fido2-manage.sh -delete -device 2 -credential Y+Dh/tSy/Q2IdZt6PW/G1A==

- Display script help information:
  ./fido2-manage.sh -help
EOF
}

# Display help if -help parameter is provided
if $help; then
    show_help
    exit 0
fi

# Check if no arguments are specified, then show help
if ! $list && ! $info && [[ -z $device ]] && ! $fingerprint && ! $storage && ! $residentKeys && [[ -z $domain ]] && ! $delete && [[ -z $credential ]] && ! $changePIN && ! $setPIN && ! $reset && ! $uvs && ! $uvd && ! $help; then
    show_help
    exit 1
fi

if $list; then
    command_output=$("$FIDO2_TOKEN_CMD" -L 2>&1)
    if [ $? -ne 0 ]; then
        show_message "Error executing $FIDO2_TOKEN_CMD -L: $command_output" "Error"
        exit 1
    fi

    device_count=1
  	
    echo "$command_output" | while read -r line; do
        if [[ $line =~ ^([^:]+) ]]; then
        
         echo "Device [$device_count] : $(echo "${line}" | ggrep -oP '\(([^)]+)\)' | sed 's/(\(.*\))/\1/')"

            device_count=$((device_count + 1))
        fi
    done
    exit 0
fi

if [[ -n $device ]]; then
    device_index=$((device - 1))
    command_output=$("$FIDO2_TOKEN_CMD" -L 2>&1)
    if [ $? -ne 0 ]; then
        show_message "Error executing $FIDO2_TOKEN_CMD -L: $command_output" "Error"
        exit 1
    fi

    if [[ $command_output =~ pcsc://slot0: ]]; then
        device_string="pcsc://slot0"
    else
        #device_string=$(echo "$command_output" | sed -n "$((device_index + 1))p" | cut -d ':' -f 1)
	device_string=$(echo "$command_output" | sed -n "$((device_index + 1))p" | awk -F':' '{print $1":"$2}')

    fi

    if $reset; then
        show_message "WARNING: Factory reset will remove all data and settings of the device, including its PIN, fingerprints, and passkeys stored. The factory reset process is irreversible. Are you sure you want to proceed? (Y/N)"
        read -r confirmation
        if [[ $confirmation =~ [Yy] ]]; then
            show_message "Touch or press the security key button when it starts blinking."
            output=$("$FIDO2_TOKEN_CMD" -R "$device_string" 2>&1)
            if [[ $output == *"FIDO_ERR_NOT_ALLOWED"* ]]; then
                show_message "Error: Factory reset not allowed. Factory reset is only allowed within 10 seconds of powering up of the security key. Please unplug and plug the device back in and retry within 10 seconds after plugging in."
            else
                show_message "Factory reset completed."
            fi
        else
            show_message "Factory reset canceled."
        fi
        exit 0
    fi

    if $changePIN; then
        show_message "Enter the old and new PIN below."
        "$FIDO2_TOKEN_CMD" -C "$device_string"
        exit 0
    fi

    if $uvs; then
        show_message "Enforcing user verification."
        "$FIDO2_TOKEN_CMD" -Su "$device_string"
        exit 0
    fi

    if $uvd; then
        show_message "Disabling user verification."
        "$FIDO2_TOKEN_CMD" -Du "$device_string"
        exit 0
    fi

    if $setPIN; then
        show_message "Enter and confirm the PIN as prompted below."
        "$FIDO2_TOKEN_CMD" -S "$device_string"
        exit 0
    fi

    if $delete && [[ -n $credential ]]; then
        show_message "WARNING: Deleting a credential is irreversible. Are you sure you want to proceed? (Y/N)"
        read -r confirmation
        if [[ $confirmation =~ [Yy] ]]; then
            "$FIDO2_TOKEN_CMD" -D -i "$credential" "$device_string"
            show_message "Credential deleted successfully."
        else
            show_message "Deletion canceled."
        fi
        exit 0
    fi

pin_option=$([[ -n $pin ]] && echo "-w $pin")

# Fingerprint enrollment
if $fingerprint; then
echo "Enrolling fingerprints (for bio models only)"
    "$FIDO2_TOKEN_CMD" $pin_option  -S -e "$device_string"  
    exit 0
fi    
# Main logic
if $storage; then
    "$FIDO2_TOKEN_CMD" -I -c  $pin_option  "$device_string" 
 
    exit 0
elif $residentKeys; then
    if [[ -n $domain ]]; then
        domain_command="\"$FIDO2_TOKEN_CMD\" -L -k \"$domain\" $pin_option  \"$device_string\"  "
	#echo $domain_command
        domain_output=$(eval $domain_command)
        
        

        # Process the output line by line
        echo "$domain_output" | while read -r line; do
            key_id=$(echo "$line" | awk '{print $1}')
            credential_id=$(echo "$line" | awk '{print $2}')
            user_field=$(echo "$line" | awk '{print $3 , $4}')
            email_field=$(echo "$line" | awk '{print $5, $6}')

            if [[ "$user_field" == "(null)" ]]; then
                user_field=""
            fi

            # Determine if user_field is an email
            if [[ "$user_field" == *"@"* ]]; then
                email=$user_field
                user=""
            else
                user=$user_field
                email=$email_field
            fi

            show_message "Credential ID: $credential_id, User: $user $email"
        done
    else
        "$FIDO2_TOKEN_CMD" -L -r  $pin_option  "$device_string"  
    fi
    exit 0
fi


    if $info; then
        command_output=$("$FIDO2_TOKEN_CMD" -I "$device_string")
        show_message "Device $device Information:"
        echo "$command_output"
	 
        exit 0
    fi
fi
