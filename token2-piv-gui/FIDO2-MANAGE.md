# fido2-manage — command-line FIDO2 token manager

`fido2-manage` is a command-line tool for managing FIDO2 security keys. It ships
in the same portable folder as the Token2 Key Manager GUI (`token2-key-manager`)
and shares the same underlying libfido2 engine. It is **not** a graphical app —
run it from a terminal (PowerShell or `cmd` on Windows, a shell on macOS/Linux).

## Administrator rights on Windows — when you need them

Whether you need an elevated (Administrator) terminal depends on **how the key is
accessed**, not on the operation:

| Access path | Example keys | Admin needed? |
|-------------|--------------|---------------|
| **Smart-card / PC-SC** (`pcsc://…`) | Token2 keys over **USB** (CCID) and over **NFC** | **No** — works as a normal user |
| **Raw HID FIDO** | YubiKey over USB; non-CCID HID-only keys | **Yes** — needs an elevated terminal |

### Why

- **Token2 keys present a smart-card (CCID) interface over USB and NFC.** The
  Windows smart-card service (PC/SC) brokers that access, so a normal user
  process can talk to the key. **No administrator rights are required** for
  listing, info, PIN set/change, passkey management, fingerprint enrollment, or
  reset on a Token2 key over USB or NFC.
- **Raw-HID FIDO devices** (the plain USB HID FIDO interface, e.g. a YubiKey, or
  a Token2 key accessed as HID rather than CCID) are opened directly, and Windows
  restricts raw HID access to **elevated processes**. For those keys you must run
  the terminal **as Administrator**.

### In practice

- Token2 key plugged into USB, or tapped on an NFC reader → **just open
  PowerShell normally** and run `fido2-manage`.
- A YubiKey or other HID-only FIDO key over USB → **right-click PowerShell →
  Run as administrator**, then run `fido2-manage`.
- If a command fails with a message about *"raw HID FIDO devices need an elevated
  process"*, you are on the HID path — re-run from an elevated terminal, or use
  the key over its smart-card/NFC interface instead.

> macOS and Linux do not need elevation for either path (HID FIDO works for the
> logged-in user); on Linux make sure the usual FIDO udev rules are installed.

## Usage

```
fido2-manage -list
fido2-manage -info            -device N
fido2-manage -storage         -device N
fido2-manage -residentKeys    -device N [-domain DOMAIN]
fido2-manage -delete          -device N -credential <base64>
fido2-manage -setPIN          -device N [-pin PIN]
fido2-manage -changePIN       -device N [-pin PIN -newpin NEWPIN]
fido2-manage -reset           -device N
fido2-manage -forcePINchange  -device N [-pin PIN]
fido2-manage -setMinimumPIN M -device N [-pin PIN]
fido2-manage -uvs | -uvd      -device N [-pin PIN]
fido2-manage -fingerprint        -device N [-pin PIN] [-fingerprintname NAME]
fido2-manage -fingerprintlist    -device N [-pin PIN]
fido2-manage -deletefingerprint ID -device N [-pin PIN]
fido2-manage -renamefingerprint ID -fingerprintname NAME -device N [-pin PIN]
```

- `-device N` selects the Nth key from `-list` (1-based). Defaults to 1.
- If `-pin` is omitted, the tool prompts for it and masks the input with `*`.
- `-reset` only works shortly after the key is plugged in (CTAP power-up window);
  unplug and replug the key, then run the command and touch it when it blinks.

## Relationship to the GUI

- `token2-key-manager(.exe)` — the graphical application.
- `fido2-manage(.exe)` — this command-line tool.
- `token2-piv-tool`, `token2-fido2-token`, … — the other bundled command-line
  utilities.

All of them live in the same portable folder and use the shared libraries
(`libt2piv`, `t2fido2`/libfido2, …) that sit alongside them.
