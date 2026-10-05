# Token2 Key Manager (PIV + FIDO2)

Desktop GUI for the PIV applet on Token2 security keys. Tauri 2 (Rust) front
to `libt2piv` from token2-piv-tool — the private keys never leave the token,
including for self-signed certificate creation (signing is done on-card via
`t2piv_sign_data`).

What it does: connect to a key, show serial / applet version / PIN attempts /
Admin PIN type, list all 24 slots with key algorithm, policies and
certificate summary, generate keys (RSA 2048–4096, P-256/384, Ed25519, X25519)
with PIN/touch policies (user PIN only; the Admin PIN is needed just to reset PIN/PUK), create self-signed certificates, import/export/delete
certificates, import PFX/P12 (key + certificate, via t2piv_util_import_key_blob in libt2piv), show public keys, verify per-slot attestation against the embedded TOKEN2 PIV CA
(src-tauri/assets/token2-piv-ca.pem; chain leaf ← F9 ← CA, key match, device-property
extensions on the 66563.3.x and legacy 41482.3.x arcs), change PIN/PUK,
unblock PIN, set Admin PIN, set retry limits, reset the applet.

## Layout

    ui/          static HTML/CSS/JS (no bundler, `withGlobalTauri`)
    src-tauri/   Rust: t2piv.rs (FFI), piv.rs (safe wrapper), certs.rs
                 (SPKI/PEM, X.509 summary, rcgen remote-key self-signing),
                 commands.rs (Tauri commands), main.rs

## FIDO2 mode

Links upstream libfido2 (Yubico, BSD-2; cloned by build-all.ps1 into `../deps/libfido2`), built with
`USE_PCSC=ON`. Token2 keys speak FIDO over CCID, so they appear as `pcsc://` devices on USB and NFC
without elevation. Features: device info (versions,
AAGUID, firmware, options, retries), set/change PIN, minimum PIN length, force PIN change,
always-UV toggle, factory reset, passkey inventory per relying party with delete,
fingerprint list / rename / delete / enroll (step-by-step with sensor feedback).
Point `FIDO2_LIB_DIR` at the folder containing `t2fido2.dll`/`libfido2.so` (`FIDO2_LIB_NAME`
defaults to `fido2`; the Windows build ships it as `t2fido2`). Windows: only raw HID entries need an elevated process.

## OTP mode

Two applets. **Token2 OTP** (`F0 00 00 01 4F 74 70 01`, T2F2/PIN+; protocol per
token2-otp-cli/docs/Token2-OTP-SDK-Protocol.md): READ_CONFIG feature bits, paginated READ_ALL
with live TOTP codes, READ_ONE for HOTP/button entries, WRITE/DELETE via ECDH P-256 +
AES-256-CBC (IV-1), erase-all, ENABLE_TOTP, button-HOTP slot (IV-2) with Enter/long-press/
numpad options. SET_DEVICE_TYPE is exposed only for the keyboard (HOTP) interface, with FIDO/CCID kept as-is.
OTP-PIN privacy protection (R3.4+): session via READ_AGREEMENT_PUBKEY + HMAC-SHA256 key ladder,
SET/VERIFY/CHANGE/remove/lock, protected writes and encrypted enumeration pages, ported from
token2/T2TOTP_Authenticator (the device's P-521 signature is not verified there either).
Fingerprint protection for OTP (manual V1.3 §1.14/§1.20, from framefilter/keyroost): enabled/disabled via the PIN verify with an EncConfig block; fingerprint unlock = VERIFY body 01 then poll 80 11 00 00 00 until 9000.
The tab tries this applet first and falls back to standard **OATH**, which talks to the OATH applet (`A0 00 00 05 27 21 01`) directly over PC/SC (`pcsc` crate, no extra
library), following the protocol used in Token2's Libre Key Companion: list accounts with live
TOTP codes (CALCULATE ALL), HOTP on demand, touch-protected credentials, add via fields or an
`otpauth://` URI, delete, set/change/remove the applet password (PBKDF2-HMAC-SHA1 over the
device id, mutual challenge-response), erase applet.

## Fingerprint (UV) authorisation in FIDO2 mode

Leaving the PIN empty on a key with a fingerprint sensor makes libfido2 obtain a UV token
instead of a PIN token: "Unlock with fingerprint", passkey list/edit/delete, fingerprint
rename/delete/enroll, minimum PIN length, force-PIN-change and always-UV all work that way.

## Token2 CTAP tunnel (from USBPcap captures of the Token2 Windows tool)

The OTP applet also tunnels CTAP2: `80 C5 03 00 <cmd><CBOR>` is a plain CTAP2 passthrough,
and `80 C5 02 0A <authenticatorConfig setMinPINLength>` applies the value to the
*alphanumeric* minimum PIN length (the numeric one is the standard CTAP setting, done via
libfido2). Implemented with PIN protocol 1 (ECDH P-256 → SHA-256, AES-256-CBC zero IV,
HMAC-SHA256 pinUvAuthParam[:16], permission 0x20).

Applet enable/disable per interface (Settings → Applets) is a challenge-response write:
`80 33 FF FF 10 <16-byte host constant>` → 16 random bytes, then `80 33 FB 00 20 <32-byte
blob>`; read with `80 33 FB 01 01 00` (one mode byte). The blob's key/format is not known
yet, so the toggles stay greyed.

## Build on Windows

1. Build token2-piv-tool first (see its README) — you need
   `build\lib\Release\libt2piv.lib` (import lib) and `libt2piv.dll`.
2. Install Rust (rustup, MSVC toolchain), Node is **not** required.
3. Install the Tauri CLI: `cargo install tauri-cli --version "^2"`.
4. Point the build at libt2piv and run:

       $env:T2PIV_LIB_DIR = "C:\src\token2-piv-tool\build\lib\Release"
       cd token2-piv-gui\src-tauri
       cargo tauri dev            # run
       cargo tauri build          # NSIS installer under target\release\bundle

5. Put `libt2piv.dll`, `libcrypto-3-x64.dll` and `zlib1.dll` next to the exe
   (for the installer, list them under `bundle.resources` in
   `tauri.conf.json` with their absolute paths, or copy them into
   `src-tauri\` and reference them relatively).

WebView2 is already present on Windows 10/11; Tauri uses it.

## Build on Linux / macOS

    export T2PIV_LIB_DIR=/path/to/token2-piv-tool/build/lib
    cargo tauri dev

Linux needs webkit2gtk-4.1, gtk3, librsvg, pcsclite. macOS: nothing extra;
`T2PIV_LIB_NAME=t2piv` (default).

## Notes

* Reader selection defaults to the first reader containing "TOKEN2".
* The applet answers GET METADATA for empty slots too; a slot is shown as
  empty when the algorithm byte is unknown and no certificate is present.
* `reset` needs PIN and PUK blocked first; the UI does that automatically
  after an explicit confirmation.
* Self-signed certificates: RSA uses PKCS#1 v1.5 / SHA-256, ECC uses ECDSA
  with SHA-256 (P-256) or SHA-384 (P-384), Ed25519 is pure EdDSA.
