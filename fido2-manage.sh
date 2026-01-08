#!/bin/bash

FIDO2_TOKEN_CMD="/usr/local/bin/fido2-token2"

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
setMinimumPIN=""
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
        -setMinimumPIN) setMinimumPIN="$2"; shift ;;
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
This is a wrapper for libfido2 library

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

if $help; then
    show_help
    exit 0
fi

if ! $list && ! $info && [[ -z $device ]] && ! $fingerprint && ! $storage && ! $residentKeys && [[ -z $domain ]] && ! $delete && [[ -z $credential ]] && ! $changePIN && ! $setMinimumPIN && ! $setPIN && ! $reset && ! $uvs && ! $uvd && ! $help; then
    show_help
    exit 1
fi

if $list; then
    command_output=$($FIDO2_TOKEN_CMD -L 2>&1)
    if [ $? -ne 0 ]; then
        show_message "Error executing $FIDO2_TOKEN_CMD -L: $command_output" "Error"
        exit 1
    fi

    device_count=1
    echo "$command_output" | while read -r line; do
        if [[ $line =~ ^([^:]+) ]]; then
            echo "Device [$device_count] : $(echo "${line}" | grep -oP '(?<=\()(.+)(?=\))')"
            device_count=$((device_count + 1))
        fi
    done
    exit 0
fi

if [[ -n $device ]]; then
    device_index=$((device - 1))
    command_output=$($FIDO2_TOKEN_CMD -L 2>&1)
    if [ $? -ne 0 ]; then
        show_message "Error executing $FIDO2_TOKEN_CMD -L: $command_output" "Error"
        exit 1
    fi

    if [[ $command_output =~ pcsc://slot0: ]]; then
        device_string="pcsc://slot0"
    else
        device_string=$(echo "$command_output" | sed -n "$((device_index + 1))p" | cut -d ':' -f 1)
    fi

    if $reset; then
        show_message "WARNING: Factory reset will remove all data and settings of the device, including its PIN, fingerprints, and passkeys stored. The factory reset process is irreversible. Are you sure you want to proceed? (Y/N)"
        read -r confirmation
        if [[ $confirmation =~ [Yy] ]]; then
            show_message "Touch or press the security key button when it starts blinking."
            output=$($FIDO2_TOKEN_CMD -R "$device_string" 2>&1)
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
        $FIDO2_TOKEN_CMD -C "$device_string"
        exit 0
    fi

    if $uvs; then
        show_message "Enforcing user verification."
        $FIDO2_TOKEN_CMD -Su "$device_string"
        exit 0
    fi

    if $uvd; then
        show_message "Disabling user verification."
        $FIDO2_TOKEN_CMD -Du "$device_string"
        exit 0
    fi

    if $setPIN; then
        show_message "Enter and confirm the PIN as prompted below."
        $FIDO2_TOKEN_CMD -S "$device_string"
        exit 0
    fi

    if [[ -n $setMinimumPIN ]]; then
        show_message "Setting minimum PIN length to $setMinimumPIN on device $device"
        "$FIDO2_TOKEN_CMD" -S -l "$setMinimumPIN" "$device_string"
        if [ $? -ne 0 ]; then
            show_message "Error: Failed to set minimum PIN length." "Error"
            exit 1
        fi
        exit 0
    fi

    if $delete && [[ -n $credential ]]; then
        show_message "WARNING: Deleting a credential is irreversible. Are you sure you want to proceed? (Y/N)"
        read -r confirmation
        if [[ $confirmation =~ [Yy] ]]; then
            $FIDO2_TOKEN_CMD -D -i "$credential" "$device_string"
            show_message "Passing credential deletion request"
        else
            show_message "Deletion canceled."
        fi
        exit 0
    fi

    if $fingerprint; then
        echo "Enrolling fingerprints (for bio models only)"
        $FIDO2_TOKEN_CMD -S -e "$device_string" $([[ -n $pin ]] && echo "-w $pin")
        exit 0
    fi

    if $storage; then
        $FIDO2_TOKEN_CMD -I -c "$device_string" $([[ -n $pin ]] && echo "-w $pin")
        exit 0
    elif $residentKeys; then
        if [[ -n $domain ]]; then
            domain_command="$FIDO2_TOKEN_CMD -L -k \"$domain\" \"$device_string\" $([[ -n $pin ]] && echo "-w $pin")"
            domain_output=$(eval $domain_command)

            echo "$domain_output" | while read -r line; do
                key_id=$(echo "$line" | awk '{print $1}')
                credential_id=$(echo "$line" | awk '{print $2}')
                user_field=$(echo "$line" | awk '{print $3 , $4}')
                email_field=$(echo "$line" | awk '{print $5, $6}')

                if [[ "$user_field" == "(null)" ]]; then
                    user_field=""
                fi

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
            $FIDO2_TOKEN_CMD -L -r "$device_string" $(if [[ -n $pin ]]; then echo "-w $pin"; fi)
        fi
        exit 0
    fi

    if $info; then
        command_output=$($FIDO2_TOKEN_CMD -I "$device_string")
        show_message "Device $device Information:"
        echo "$command_output"
        exit 0
    fi
fi
