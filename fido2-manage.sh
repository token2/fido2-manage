#!/bin/bash

# Locate fido2-token2: check next to this script first (dev builds),
# then fall back to PATH (handles any install prefix).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FIDO2_TOKEN_CMD=""
for candidate in \
    "$SCRIPT_DIR/fido2-token2" \
    "$SCRIPT_DIR/build/tools/fido2-token2" \
    "$SCRIPT_DIR/tools/fido2-token2"
do
    if [[ -f "$candidate" ]]; then
        FIDO2_TOKEN_CMD="$candidate"
        break
    fi
done

if [[ -z "$FIDO2_TOKEN_CMD" ]]; then
    FIDO2_TOKEN_CMD="$(command -v fido2-token2 2>/dev/null)"
fi

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

# --- Security-key workbench extensions ---
stats=false
sshKeygen=false
sshType="ed25519-sk"       # ed25519-sk | ecdsa-sk
sshResident=false          # store the key handle on the device (discoverable)
sshOutput=""               # output path for the generated key
sshApplication=""          # application string, e.g. ssh:hostname
largeBlobGet=false
largeBlobSet=false
largeBlobDelete=false
blobFile=""                # path to read/write large-blob payload
rpId=""                    # relying-party id that owns the large-blob
keyFile=""                 # optional base64 AES-256 key file addressing the blob
editCredential=false
userId=""
name=""
displayName=""

# --- Track 1: remaining binary primitives ---
bioList=false
bioDelete=false
bioRename=false
templateId=""
templateName=""
setPinMinRPs=""            # comma-separated rp_id list for -S -m
genBlobKey=""             # output path to generate a base64 AES-256 blob key

# --- Track 2: SSH key lifecycle ---
sshDownload=false          # ssh-keygen -K (rehydrate resident SSH creds)
sshList=false              # list SSH-SK resident creds on the key
sshDir=""                  # dir for -sshDownload output
sshAddKey=false            # ssh-add the generated/downloaded key
sshUpload=false            # ssh-copy-id the pubkey to a remote host
sshHost=""                 # [user@]host target for -sshUpload
sshPort=""                 # optional ssh port for -sshUpload
sshKey=""                  # identity/key path for -sshUpload / -sshAddKey

# --- Track 3: audit / export ---
audit=false
auditFormat="json"        # json | csv
auditOutput=""             # optional output file

# --- Track 4: age / LUKS orchestration ---
ageSetup=false
ageOutput=""               # identity output path for age-plugin-fido2-hmac
luksEnroll=false
luksDevice=""              # block device for systemd-cryptenroll (guarded)

show_message() {
    local message=$1
    local type=${2:-"Info"}
    echo "[$type] $message"
}

while [[ "$#" -gt 0 ]]; do
    case $1 in
        -list|--list) list=true ;;
        -info|--info) info=true ;;
        -device|--device) device="$2"; shift ;;
        -pin|--pin) pin="$2"; shift ;;
        -storage|--storage) storage=true ;;
        -fingerprint|--fingerprint) fingerprint=true ;;
        -residentKeys|--residentKeys) residentKeys=true ;;
        -domain|--domain) domain="$2"; shift ;;
        -delete|--delete) delete=true ;;
        -credential|--credential) credential="$2"; shift ;;
        -changePIN|--changePIN) changePIN=true ;;
        -setPIN|--setPIN) setPIN=true ;;
        -reset|--reset) reset=true ;;
        -setMinimumPIN|--setMinimumPIN) setMinimumPIN="$2"; shift ;;
        -uvs|--uvs) uvs=true ;;
        -uvd|--uvd) uvd=true ;;
        -stats|--stats) stats=true ;;
        -sshKeygen|--sshKeygen) sshKeygen=true ;;
        -sshType|--sshType) sshType="$2"; shift ;;
        -sshResident|--sshResident) sshResident=true ;;
        -sshOutput|--sshOutput) sshOutput="$2"; shift ;;
        -sshApplication|--sshApplication) sshApplication="$2"; shift ;;
        -largeBlobGet|--largeBlobGet) largeBlobGet=true ;;
        -largeBlobSet|--largeBlobSet) largeBlobSet=true ;;
        -largeBlobDelete|--largeBlobDelete) largeBlobDelete=true ;;
        -blobFile|--blobFile) blobFile="$2"; shift ;;
        -rpId|--rpId) rpId="$2"; shift ;;
        -keyFile|--keyFile) keyFile="$2"; shift ;;
        -editCredential|--editCredential) editCredential=true ;;
        -userId|--userId) userId="$2"; shift ;;
        -name|--name) name="$2"; shift ;;
        -displayName|--displayName) displayName="$2"; shift ;;
        -bioList|--bioList) bioList=true ;;
        -bioDelete|--bioDelete) bioDelete=true ;;
        -bioRename|--bioRename) bioRename=true ;;
        -templateId|--templateId) templateId="$2"; shift ;;
        -templateName|--templateName) templateName="$2"; shift ;;
        -setPinMinRPs|--setPinMinRPs) setPinMinRPs="$2"; shift ;;
        -genBlobKey|--genBlobKey) genBlobKey="$2"; shift ;;
        -sshDownload|--sshDownload) sshDownload=true ;;
        -sshList|--sshList) sshList=true ;;
        -sshDir|--sshDir) sshDir="$2"; shift ;;
        -sshAddKey|--sshAddKey) sshAddKey=true ;;
        -sshUpload|--sshUpload) sshUpload=true ;;
        -sshHost|--sshHost) sshHost="$2"; shift ;;
        -sshPort|--sshPort) sshPort="$2"; shift ;;
        -sshKey|--sshKey) sshKey="$2"; shift ;;
        -audit|--audit) audit=true ;;
        -auditFormat|--auditFormat) auditFormat="$2"; shift ;;
        -auditOutput|--auditOutput) auditOutput="$2"; shift ;;
        -ageSetup|--ageSetup) ageSetup=true ;;
        -ageOutput|--ageOutput) ageOutput="$2"; shift ;;
        -luksEnroll|--luksEnroll) luksEnroll=true ;;
        -luksDevice|--luksDevice) luksDevice="$2"; shift ;;
        -help|--help) help=true ;;
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

- Show storage / capacity statistics for a device:
  ./fido2-manage.sh -stats -device 1

- Generate an SSH security-key (FIDO2) key pair bound to a device:
  ./fido2-manage.sh -sshKeygen -device 1
  ./fido2-manage.sh -sshKeygen -device 1 -sshType ecdsa-sk -sshResident -sshOutput ~/.ssh/id_thetis_sk

- Read the FIDO2 large-blob for a relying party into a file:
  ./fido2-manage.sh -largeBlobGet -device 1 -rpId ssh:myhost -blobFile ./blob.bin

- Write a file into the FIDO2 large-blob for a relying party:
  ./fido2-manage.sh -largeBlobSet -device 1 -rpId ssh:myhost -blobFile ./blob.bin

- Delete the FIDO2 large-blob for a relying party:
  ./fido2-manage.sh -largeBlobDelete -device 1 -rpId ssh:myhost

  (Large-blobs may also be addressed by an explicit key file with -keyFile <path>.)

- Edit metadata (user id / name / display name) of a resident credential:
  ./fido2-manage.sh -editCredential -device 1 -credential <cred_id> -userId <user_id> -name user@example.com -displayName "Full Name"

- Delete a credential on a specific device:
  ./fido2-manage.sh -delete -device 2 -credential Y+Dh/tSy/Q2IdZt6PW/G1A==

- Display script help information:
  ./fido2-manage.sh -help

- List biometric enrollments (biometric models only):
  ./fido2-manage.sh -bioList -device 1

- Delete / rename a biometric template:
  ./fido2-manage.sh -bioDelete -device 1 -templateId <id>
  ./fido2-manage.sh -bioRename -device 1 -templateId <id> -templateName "Left index"

- Set the relying-party allow-list for min PIN length:
  ./fido2-manage.sh -setPinMinRPs login.microsoft.com,google.com -device 1

- Generate an AES-256 base64 key for keyFile-addressed large-blobs:
  ./fido2-manage.sh -genBlobKey ./blob.key

- List SSH resident credentials / download resident SSH keys:
  ./fido2-manage.sh -sshList -device 1
  ./fido2-manage.sh -sshDownload -device 1 -sshDir ~/.ssh

- Upload an SSH public key to a remote host's authorized_keys:
  ./fido2-manage.sh -sshUpload -sshKey ~/.ssh/id_ed25519_sk.pub -sshHost user@host
  ./fido2-manage.sh -sshUpload -sshKey ~/.ssh/id_ed25519_sk.pub -sshHost user@host -sshPort 2222

- Add an SSH (FIDO) key to the local ssh-agent:
  ./fido2-manage.sh -sshAddKey -sshKey ~/.ssh/id_ed25519_sk

- Export an audit report (json or csv):
  ./fido2-manage.sh -audit -device 1 -auditFormat json -auditOutput ./audit.json

- Set up an age identity backed by the key's hmac-secret:
  ./fido2-manage.sh -ageSetup -device 1 -ageOutput ~/.age/fido2.txt

- Print the (guarded) LUKS enrollment command for a device:
  ./fido2-manage.sh -luksEnroll -device 1 -luksDevice /dev/sda2
EOF
}

if $help; then
    show_help
    exit 0
fi

if ! $list && ! $info && [[ -z $device ]] && ! $fingerprint && ! $storage && ! $residentKeys && [[ -z $domain ]] && ! $delete && [[ -z $credential ]] && ! $changePIN && [[ -z $setMinimumPIN ]] && ! $setPIN && ! $reset && ! $uvs && ! $uvd && ! $stats && ! $sshKeygen && ! $largeBlobGet && ! $largeBlobSet && ! $largeBlobDelete && ! $editCredential && ! $bioList && ! $bioDelete && ! $bioRename && [[ -z $setPinMinRPs ]] && [[ -z $genBlobKey ]] && ! $sshDownload && ! $sshList && ! $sshUpload && ! $sshAddKey && ! $audit && ! $ageSetup && ! $luksEnroll && ! $help; then
    show_help
    exit 1
fi

if [[ -z "$FIDO2_TOKEN_CMD" ]]; then
    show_message "fido2-token2 not found. Install it or ensure it is on your PATH." "Error"
    exit 1
fi

if [[ -n $genBlobKey ]]; then
    # Generate a base64-encoded 32-byte AES-256 key for keyFile-addressed
    # large-blobs. Device-independent.
    if command -v openssl >/dev/null 2>&1; then
        umask 077
        openssl rand -base64 32 > "$genBlobKey"
        show_message "Generated 32-byte AES-256 base64 blob key at $genBlobKey"
        exit 0
    else
        show_message "openssl not found; cannot generate blob key." "Error"
        exit 1
    fi
fi

if $sshUpload; then
    # Push a public key to a remote host's authorized_keys via ssh-copy-id.
    # Device-independent: operates on the generated key file, not the token.
    [[ -z "$sshHost" ]] && { show_message "-sshHost [user@]host is required for -sshUpload." "Error"; exit 1; }
    [[ -z "$sshKey" ]] && { show_message "-sshKey <path to public/identity key> is required for -sshUpload." "Error"; exit 1; }
    if ! command -v ssh-copy-id >/dev/null 2>&1; then
        show_message "ssh-copy-id not found. Install openssh-client." "Error"
        exit 1
    fi
    copy_args=(-i "$sshKey")
    [[ -n "$sshPort" ]] && copy_args+=(-p "$sshPort")
    copy_args+=("$sshHost")
    show_message "Uploading $sshKey to $sshHost via ssh-copy-id (you may be prompted to authenticate / touch the key)."
    ssh-copy-id "${copy_args[@]}"
    rc=$?
    [[ $rc -eq 0 ]] && show_message "Public key installed on $sshHost." || show_message "ssh-copy-id failed (exit $rc)." "Error"
    exit $rc
fi

if $sshAddKey; then
    # Load a (FIDO) SSH key into the local ssh-agent.
    [[ -z "$sshKey" ]] && { show_message "-sshKey <path to private/identity key> is required for -sshAddKey." "Error"; exit 1; }
    if ! command -v ssh-add >/dev/null 2>&1; then
        show_message "ssh-add not found. Install openssh-client." "Error"
        exit 1
    fi
    show_message "Adding $sshKey to the ssh-agent (touch the key if prompted)."
    ssh-add "$sshKey"
    rc=$?
    [[ $rc -eq 0 ]] && show_message "Key added to the ssh-agent." || show_message "ssh-add failed (exit $rc). Is an agent running (ssh-agent)?" "Error"
    exit $rc
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

            # Line format from fido2-token -L -k:
            #   NN: <cred_id> <display name...> <email> <user_id_b64> <alg> <uv>
            # The last three tokens are always user_id, algorithm and uv policy;
            # the email is the token immediately before user_id; the credential
            # id is the token after "NN:"; everything in between is the (multi
            # word) display name.
            echo "$domain_output" | while read -r line; do
                [[ -z "$line" ]] && continue
                # strip leading "NN:" index label
                rest=${line#*: }
                # shellcheck disable=SC2206
                fields=($rest)
                n=${#fields[@]}
                if (( n < 5 )); then
                    show_message "Credential ID: $rest, User: , Email: , Handle: "
                    continue
                fi
                credential_id=${fields[0]}
                uv=${fields[n-1]}
                alg=${fields[n-2]}
                user_id=${fields[n-3]}
                email=${fields[n-4]}
                # display name = tokens 1 .. n-5 (inclusive)
                user=""
                for (( i=1; i<=n-5; i++ )); do
                    user+="${fields[i]} "
                done
                user=${user% }
                [[ "$user" == "(null)" ]] && user=""
                [[ "$email" == "(null)" ]] && email=""
                show_message "Credential ID: $credential_id, User: $user, Email: $email, Handle: $user_id"
            done
        else
            $FIDO2_TOKEN_CMD -L -r "$device_string" $(if [[ -n $pin ]]; then echo "-w $pin"; fi)
        fi
        exit 0
    fi

    if $stats; then
        show_message "Storage / capacity statistics for device $device:"
        # -I -c prints credential-management metadata (existing/remaining rk,
        # etc). Combine with the general -I output for min PIN and limits.
        stats_out=$($FIDO2_TOKEN_CMD -I -c "$device_string" $([[ -n $pin ]] && echo "-w $pin") 2>&1)
        info_out=$($FIDO2_TOKEN_CMD -I "$device_string" 2>&1)
        echo "$stats_out"
        echo "$info_out" | grep -iE "remaining rk|maxlargeblob|maxcredblob|maxcredlen|maxmsgsiz|minpinlen|pin retries|uv retries|maxcredcntlst" || true
        exit 0
    fi

    if $sshKeygen; then
        # SSH FIDO2 (security-key) key generation. Uses the system ssh-keygen,
        # which talks to the authenticator via libfido2. The key type must be a
        # *-sk type. Resident keys are discoverable and can be re-derived on
        # another machine with `ssh-keygen -K`.
        if ! command -v ssh-keygen >/dev/null 2>&1; then
            show_message "ssh-keygen not found. Install openssh-client." "Error"
            exit 1
        fi
        case "$sshType" in
            ed25519-sk|ecdsa-sk) ;;
            *) show_message "Invalid -sshType '$sshType'. Use ed25519-sk or ecdsa-sk." "Error"; exit 1 ;;
        esac
        out_path="$sshOutput"
        if [[ -z "$out_path" ]]; then
            out_path="$HOME/.ssh/id_${sshType//-/_}"
        fi
        mkdir -p "$(dirname "$out_path")"
        ssh_opts=()
        $sshResident && ssh_opts+=("-O" "resident")
        [[ -n "$sshApplication" ]] && ssh_opts+=("-O" "application=$sshApplication")
        show_message "Generating $sshType key at $out_path. Touch the security key when it blinks."
        ssh-keygen -t "$sshType" "${ssh_opts[@]}" -f "$out_path"
        rc=$?
        if [[ $rc -eq 0 ]]; then
            show_message "SSH key generated: $out_path (public key: ${out_path}.pub)"
        else
            show_message "ssh-keygen failed (exit $rc)." "Error"
            exit $rc
        fi
        exit 0
    fi

    # Build large-blob addressing args: either -k <keyFile> or -n <rpId>.
    blob_addr=()
    if [[ -n "$keyFile" ]]; then
        blob_addr=(-k "$keyFile")
    elif [[ -n "$rpId" ]]; then
        blob_addr=(-n "$rpId")
        [[ -n "$credential" ]] && blob_addr+=(-i "$credential")
    fi
    blob_pin_args=()
    [[ -n "$pin" ]] && blob_pin_args=(-w "$pin")

    if $largeBlobGet; then
        [[ -z "$blobFile" ]] && { show_message "-blobFile <path> is required for -largeBlobGet." "Error"; exit 1; }
        [[ ${#blob_addr[@]} -eq 0 ]] && { show_message "Provide -rpId <rp> (optionally -credential) or -keyFile <path> to address the large-blob." "Error"; exit 1; }
        show_message "Reading large-blob from device $device into $blobFile"
        "$FIDO2_TOKEN_CMD" -G -b "${blob_addr[@]}" "$blobFile" "$device_string" "${blob_pin_args[@]}"
        exit $?
    fi

    if $largeBlobSet; then
        [[ -z "$blobFile" ]] && { show_message "-blobFile <path> is required for -largeBlobSet." "Error"; exit 1; }
        [[ ! -f "$blobFile" ]] && { show_message "Blob file not found: $blobFile" "Error"; exit 1; }
        [[ ${#blob_addr[@]} -eq 0 ]] && { show_message "Provide -rpId <rp> (optionally -credential) or -keyFile <path> to address the large-blob." "Error"; exit 1; }
        show_message "Writing $blobFile into the large-blob of device $device"
        "$FIDO2_TOKEN_CMD" -S -b "${blob_addr[@]}" "$blobFile" "$device_string" "${blob_pin_args[@]}"
        exit $?
    fi

    if $largeBlobDelete; then
        [[ ${#blob_addr[@]} -eq 0 ]] && { show_message "Provide -rpId <rp> (optionally -credential) or -keyFile <path> to address the large-blob." "Error"; exit 1; }
        show_message "WARNING: Deleting the large-blob is irreversible. Proceed? (Y/N)"
        read -r confirmation
        if [[ $confirmation =~ [Yy] ]]; then
            "$FIDO2_TOKEN_CMD" -D -b "${blob_addr[@]}" "$device_string" "${blob_pin_args[@]}"
            exit $?
        fi
        show_message "Large-blob deletion canceled."
        exit 0
    fi

    if $editCredential; then
        [[ -z "$credential" ]] && { show_message "-credential <cred_id> is required for -editCredential." "Error"; exit 1; }
        [[ -z "$userId" ]] && { show_message "-userId <user_id> is required for -editCredential." "Error"; exit 1; }
        show_message "Updating metadata for credential $credential on device $device"
        # -Sc -i cred_id -k user_id -n name -p display_name device
        "$FIDO2_TOKEN_CMD" -Sc -i "$credential" -k "$userId" -n "$name" -p "$displayName" "$device_string" $([[ -n $pin ]] && echo "-w $pin")
        exit $?
    fi

    # --- Track 1: biometric template management ---
    if $bioList; then
        show_message "Biometric enrollments on device $device:"
        "$FIDO2_TOKEN_CMD" -L -e "$device_string" $([[ -n $pin ]] && echo "-w $pin")
        exit $?
    fi

    if $bioDelete; then
        [[ -z "$templateId" ]] && { show_message "-templateId <id> is required for -bioDelete." "Error"; exit 1; }
        show_message "Deleting biometric enrollment $templateId on device $device"
        "$FIDO2_TOKEN_CMD" -D -e -i "$templateId" "$device_string" $([[ -n $pin ]] && echo "-w $pin")
        exit $?
    fi

    if $bioRename; then
        [[ -z "$templateId" || -z "$templateName" ]] && { show_message "-templateId and -templateName are required for -bioRename." "Error"; exit 1; }
        show_message "Renaming biometric template $templateId to '$templateName' on device $device"
        "$FIDO2_TOKEN_CMD" -S -i "$templateId" -n "$templateName" "$device_string" $([[ -n $pin ]] && echo "-w $pin")
        exit $?
    fi

    # --- Track 1: min-PIN-length RP allow-list ---
    if [[ -n $setPinMinRPs ]]; then
        show_message "Setting min-PIN-length RP allow-list to '$setPinMinRPs' on device $device"
        "$FIDO2_TOKEN_CMD" -S -m "$setPinMinRPs" "$device_string" $([[ -n $pin ]] && echo "-w $pin")
        exit $?
    fi

    # --- Track 2: SSH key lifecycle ---
    if $sshList; then
        # SSH-SK resident credentials use rp_id prefixed with "ssh:".
        show_message "SSH resident credentials on device $device:"
        "$FIDO2_TOKEN_CMD" -L -r "$device_string" $([[ -n $pin ]] && echo "-w $pin") | grep -i "ssh:" || show_message "No SSH resident credentials found."
        exit 0
    fi

    if $sshDownload; then
        if ! command -v ssh-keygen >/dev/null 2>&1; then
            show_message "ssh-keygen not found. Install openssh-client." "Error"
            exit 1
        fi
        target_dir="${sshDir:-$PWD}"
        mkdir -p "$target_dir"
        show_message "Downloading resident SSH keys into $target_dir. Touch the key when it blinks."
        ( cd "$target_dir" && ssh-keygen -K )
        rc=$?
        [[ $rc -eq 0 ]] && show_message "Resident SSH keys downloaded to $target_dir." || show_message "ssh-keygen -K failed (exit $rc)." "Error"
        exit $rc
    fi

    # --- Track 3: audit / export ---
    if $audit; then
        info_out=$("$FIDO2_TOKEN_CMD" -I "$device_string" 2>&1)
        rp_out=$("$FIDO2_TOKEN_CMD" -L -r "$device_string" $([[ -n $pin ]] && echo "-w $pin") 2>&1)
        emit=""
        if [[ "$auditFormat" == "csv" ]]; then
            emit="section,key,value"$'\n'
            while IFS= read -r l; do
                [[ "$l" == *": "* ]] && emit+="info,\"${l%%: *}\",\"${l#*: }\""$'\n'
            done <<< "$info_out"
            while IFS= read -r l; do
                [[ -n "$l" ]] && emit+="rp,\"credential\",\"${l//\"/\"\"}\""$'\n'
            done <<< "$rp_out"
        else
            # JSON
            emit="{\"device\": $device, \"info\": {"
            first=true
            while IFS= read -r l; do
                if [[ "$l" == *": "* ]]; then
                    k="${l%%: *}"; v="${l#*: }"
                    k="${k//\"/\\\"}"; v="${v//\"/\\\"}"
                    $first || emit+=", "
                    emit+="\"$k\": \"$v\""
                    first=false
                fi
            done <<< "$info_out"
            emit+="}, \"relying_parties\": ["
            first=true
            while IFS= read -r l; do
                if [[ -n "$l" ]]; then
                    v="${l//\"/\\\"}"
                    $first || emit+=", "
                    emit+="\"$v\""
                    first=false
                fi
            done <<< "$rp_out"
            emit+="]}"
        fi
        if [[ -n "$auditOutput" ]]; then
            printf '%s\n' "$emit" > "$auditOutput"
            show_message "Audit written to $auditOutput ($auditFormat)."
        else
            printf '%s\n' "$emit"
        fi
        exit 0
    fi

    # --- Track 4: age-plugin-fido2-hmac setup (guided) ---
    if $ageSetup; then
        if ! command -v age-plugin-fido2-hmac >/dev/null 2>&1; then
            show_message "age-plugin-fido2-hmac not found. Install it, then re-run. See https://github.com/olastor/age-plugin-fido2-hmac" "Error"
            exit 1
        fi
        out="${ageOutput:-$HOME/.age/fido2-hmac.txt}"
        mkdir -p "$(dirname "$out")"
        show_message "Generating age identity backed by the security key's hmac-secret. Touch the key when prompted."
        age-plugin-fido2-hmac -g -o "$out"
        rc=$?
        [[ $rc -eq 0 ]] && show_message "age identity written to $out." || show_message "age-plugin-fido2-hmac failed (exit $rc)." "Error"
        exit $rc
    fi

    # --- Track 4: LUKS enrollment (guarded, does NOT auto-run) ---
    if $luksEnroll; then
        [[ -z "$luksDevice" ]] && { show_message "-luksDevice <block device> is required for -luksEnroll." "Error"; exit 1; }
        cmd="sudo systemd-cryptenroll --fido2-device=auto $luksDevice"
        show_message "LUKS enrollment is a HIGH-RISK operation on a live volume and is NOT run automatically."
        show_message "Review and run this command manually if you are certain:"
        printf '    %s\n' "$cmd"
        exit 0
    fi

    if $info; then
        command_output=$($FIDO2_TOKEN_CMD -I "$device_string")
        show_message "Device $device Information:"
        echo "$command_output"
        exit 0
    fi
fi
