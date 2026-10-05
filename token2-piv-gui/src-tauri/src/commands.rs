use crate::certs::{self, CertSummary};
use crate::piv::{algo_from_name, GeneratedKey, KeyInfo, Piv, SlotMeta};
use crate::t2piv::*;
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
pub struct Session {
    piv: Option<Piv>,
    reader: String,
    /// Admin PIN (PIV management key) that last authenticated successfully.
    admin_key: Option<Vec<u8>>,
}

const DEFAULT_ADMIN_KEY: &str = "865362865362865362865362865362865362865362865362";
const DEFAULT_PIN: &str = "865362"; // Token2 factory PIV PIN

/// Authenticate with the cached Admin PIN, else the factory default.
#[allow(dead_code)]
pub struct AppState(pub Mutex<Session>);

type R<T> = Result<T, String>;

fn with_piv<T>(state: &State<AppState>, f: impl FnOnce(&Piv) -> R<T>) -> R<T> {
    let s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let piv = s.piv.as_ref().ok_or("No key connected")?;
    // Re-select PIV (a probe/FIDO tunnel/OTP session may have left the shared card
    // on another applet -> PIV instructions would fail 6D00/NOT_SUPPORTED).
    let reader = s.reader.clone();
    if !reader.is_empty() { let _ = piv.connect(&reader); }
    f(piv)
}

/// A PIV write operation (generate, self-sign, import, delete). The reference tool
/// authorises these with: SELECT PIV -> VERIFY PIN (unlocks the PIN-protected
/// management key on Bio/PIN+ keys) -> GENERAL AUTHENTICATE the management key
/// (INS 0x87, ref 0x9b) -> the operation. Verified against a -v CLI trace.
fn with_piv_write<T>(state: &State<AppState>, pin: &str, f: impl FnOnce(&Piv) -> R<T>) -> R<T> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let reader = s.reader.clone();
    let cached = s.admin_key.clone();
    let piv = s.piv.as_ref().ok_or("No key connected")?;
    if !reader.is_empty() { let _ = piv.connect(&reader); }   // SELECT PIV
    piv.verify_pin(pin)?;                                       // unlock mgmt key
    // authenticate the management key (cached, else factory default)
    let mut good: Option<Vec<u8>> = None;
    let mut cands: Vec<Vec<u8>> = Vec::new();
    if let Some(k) = &cached { cands.push(k.clone()); }
    cands.push(hex::decode(DEFAULT_ADMIN_KEY).unwrap());
    for k in &cands { if piv.authenticate(k).is_ok() { good = Some(k.clone()); break; } }
    if good.is_none() {
        return Err("could not authenticate the management key (is the PIV PIN correct?)".into());
    }
    let out = f(piv);
    if let Some(k) = good { s.admin_key = Some(k); }
    out
}

#[derive(Serialize)]
pub struct SlotView {
    pub slot: u8,
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub meta: Option<SlotMeta>,
    pub cert: Option<CertSummary>,
}

const PRIMARY: [(u8, &str); 4] = [
    (SLOT_AUTH, "Authentication"),
    (SLOT_SIGN, "Digital signature"),
    (SLOT_KEYMGM, "Key management"),
    (SLOT_CARDAUTH, "Card authentication"),
];

#[tauri::command]
pub fn list_readers(state: State<AppState>) -> R<Vec<String>> {
    let piv = Piv::new()?;
    let readers = piv.list_readers()?;
    let _ = state; // keep signature uniform
    Ok(readers)
}

#[tauri::command]
pub async fn connect(state: State<'_, AppState>, reader: Option<String>) -> R<KeyInfo> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    s.piv = None;
    let piv = Piv::new()?;
    let wanted = reader.unwrap_or_else(|| "TOKEN2".to_string());
    piv.connect(&wanted)?;
    let info = piv.info(&wanted)?;
    s.piv = Some(piv);
    s.reader = wanted;
    Ok(info)
}

#[tauri::command]
pub fn disconnect(state: State<AppState>) -> R<()> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    if let Some(p) = s.piv.take() {
        p.disconnect();
    }
    s.admin_key = None;
    Ok(())
}

#[tauri::command]
pub fn get_info(state: State<AppState>) -> R<KeyInfo> {
    let s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let piv = s.piv.as_ref().ok_or("No key connected")?;
    piv.info(&s.reader)
}

#[tauri::command]
pub async fn list_slots(state: State<'_, AppState>) -> R<Vec<SlotView>> {
    with_piv(&state, |piv| {
        let mut out = Vec::new();
        let mut slots: Vec<(u8, String, bool)> =
            PRIMARY.iter().map(|(s, n)| (*s, n.to_string(), true)).collect();
        for i in 0..20u8 {
            slots.push((0x82 + i, format!("Retired {}", i + 1), false));
        }
        for (slot, name, primary) in slots {
            // The applet answers GET METADATA for empty slots too; treat an
            // unknown algorithm as "no key".
            let meta = piv.slot_metadata(slot).ok().filter(|m| !m.algorithm_name.is_empty());
            let cert = piv.read_cert(slot)?.and_then(|der| certs::summarize(&der).ok());
            out.push(SlotView { slot, id: format!("{slot:02x}"), name, primary, meta, cert });
        }
        Ok(out)
    })
}

#[tauri::command]
pub fn verify_pin(state: State<AppState>, pin: String) -> R<()> {
    with_piv(&state, |p| p.verify_pin(&pin))
}

#[tauri::command]
pub fn change_pin(state: State<AppState>, current: String, new: String) -> R<()> {
    with_piv(&state, |p| p.change_pin(&current, &new))
}

#[tauri::command]
pub fn change_puk(state: State<AppState>, current: String, new: String) -> R<()> {
    with_piv(&state, |p| p.change_puk(&current, &new))
}

#[tauri::command]
pub fn unblock_pin(state: State<AppState>, puk: String, new_pin: String) -> R<()> {
    with_piv(&state, |p| p.unblock_pin(&puk, &new_pin))
}

/// Unblock the PIV PIN using the management key (Admin PIN) instead of the PUK.
#[tauri::command]
pub fn unblock_pin_with_admin(state: State<AppState>, admin_hex: String, new_pin: String, pin_retries: i32, puk_retries: i32) -> R<()> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let reader = s.reader.clone();
    let cached = s.admin_key.clone();
    let piv = s.piv.as_ref().ok_or("No key connected")?;
    if !reader.is_empty() { let _ = piv.connect(&reader); }
    let mut key_used: Option<Vec<u8>> = None;
    let mut cands: Vec<Vec<u8>> = Vec::new();
    if !admin_hex.trim().is_empty() {
        let k = hex::decode(admin_hex.trim()).map_err(|_| "Admin PIN (management key) must be hex")?;
        cands.push(k);
    }
    if let Some(k) = &cached { cands.push(k.clone()); }
    cands.push(hex::decode(DEFAULT_ADMIN_KEY).unwrap());
    for k in &cands { if piv.authenticate(k).is_ok() { key_used = Some(k.clone()); break; } }
    if key_used.is_none() { return Err("could not authenticate the management key (Admin PIN)".into()); }
    let pr = if pin_retries >= 1 { pin_retries } else { 3 };
    let ur = if puk_retries >= 1 { puk_retries } else { 3 };
    piv.set_pin_retries(pr, ur).map_err(|e| format!("could not reset the PIN retries: {e}"))?;
    piv.change_pin(DEFAULT_PIN, &new_pin).map_err(|e| format!("PIN retries reset, but setting the new PIN failed: {e}"))?;
    if let Some(k) = key_used { s.admin_key = Some(k); }
    Ok(())
}

#[tauri::command]
pub fn set_retries(state: State<AppState>, pin: i32, puk: i32) -> R<()> {
    with_piv(&state, |p| p.set_pin_retries(pin, puk))
}

fn parse_hex_key(hex_key: &str) -> R<Vec<u8>> {
    let k = hex::decode(hex_key.trim()).map_err(|e| format!("Admin PIN is not hex: {e}"))?;
    if ![24, 16, 32].contains(&k.len()) {
        return Err("Admin PIN must be 32, 48 or 64 hex digits".into());
    }
    Ok(k)
}

#[tauri::command]
pub fn authenticate(state: State<AppState>, mgm_key: String) -> R<()> {
    let k = parse_hex_key(&mgm_key)?;
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let piv = s.piv.as_ref().ok_or("No key connected")?;
    piv.authenticate(&k).map_err(|e| format!("Admin PIN not accepted: {e}"))?;
    s.admin_key = Some(k);
    Ok(())
}

#[tauri::command]
pub fn set_mgm_key(state: State<AppState>, new_key: String, algorithm: String, touch: bool) -> R<()> {
    let k = parse_hex_key(&new_key)?;
    let algo = match algorithm.as_str() {
        "3DES" => ALGO_3DES,
        "AES128" => ALGO_AES128,
        "AES192" => ALGO_AES192,
        "AES256" => ALGO_AES256,
        _ => return Err("unknown Admin PIN algorithm".into()),
    };
    with_piv(&state, |p| p.set_mgm_key(&k, algo, touch))?;
    if let Ok(mut s) = state.0.lock() {
        s.admin_key = Some(k);
    }
    Ok(())
}

#[derive(Serialize)]
pub struct AlgoInfo {
    pub id: u8,
    pub name: String,   // value passed back to generate_key
    pub label: String,  // shown in the list
    pub can_sign: bool,
}

#[tauri::command]
pub fn supported_algorithms(state: State<AppState>) -> R<Vec<AlgoInfo>> {
    with_piv(&state, |p| {
        let mut v: Vec<AlgoInfo> = p
            .supported_algorithms()?
            .into_iter()
            .map(|id| {
                let (name, label) = match id {
                    ALGO_RSA1024 => ("RSA1024", "RSA 1024"),
                    ALGO_RSA2048 => ("RSA2048", "RSA 2048"),
                    ALGO_RSA3072 => ("RSA3072", "RSA 3072"),
                    ALGO_RSA4096 => ("RSA4096", "RSA 4096"),
                    ALGO_ECCP256 => ("ECCP256", "ECC P-256 (secp256r1)"),
                    ALGO_ECCP384 => ("ECCP384", "ECC P-384 (secp384r1)"),
                    ALGO_ED25519 => ("ED25519", "Ed25519"),
                    ALGO_X25519 => ("X25519", "X25519 (key agreement only)"),
                    _ => ("", ""),
                };
                if name.is_empty() {
                    AlgoInfo { id, name: format!("0x{id:02x}"), label: format!("algorithm 0x{id:02X}"), can_sign: false }
                } else {
                    AlgoInfo { id, name: name.into(), label: label.into(), can_sign: id != ALGO_X25519 }
                }
            })
            .collect();
        // RSA first (largest first), then curves
        v.sort_by_key(|a| match a.id {
            ALGO_RSA4096 => 0, ALGO_RSA3072 => 1, ALGO_RSA2048 => 2, ALGO_RSA1024 => 3,
            ALGO_ECCP256 => 10, ALGO_ECCP384 => 11, ALGO_ED25519 => 20, ALGO_X25519 => 21, _ => 30,
        });
        Ok(v)
    })
}

#[derive(Serialize)]
pub struct Generated {
    pub algorithm: String,
    pub public_key_pem: String,
}

fn policy(s: &str, touch: bool) -> u8 {
    match (s, touch) {
        ("never", false) => PINPOLICY_NEVER,
        ("once", false) => PINPOLICY_ONCE,
        ("always", false) => PINPOLICY_ALWAYS,
        ("never", true) => TOUCHPOLICY_NEVER,
        ("always", true) => TOUCHPOLICY_ALWAYS,
        ("cached", true) => TOUCHPOLICY_CACHED,
        _ => 0,
    }
}

#[tauri::command]
pub async fn generate_key(
    state: State<'_, AppState>,
    slot: u8,
    algorithm: String,
    pin_policy: String,
    touch_policy: String,
    pin: String,
) -> R<Generated> {
    let algo = algo_from_name(&algorithm).ok_or("unknown algorithm")?;
    with_piv_write(&state, &pin, |p| {
        let k = p.generate_key(slot, algo, policy(&pin_policy, false), policy(&touch_policy, true))?;
        let public_key_pem = certs::spki_der(&k).map(|d| certs::to_pem("PUBLIC KEY", &d)).unwrap_or_default();
        Ok(Generated { algorithm: algorithm.clone(), public_key_pem })
    })}


/// Public key TLV from GET METADATA -> GeneratedKey shape.
fn key_from_metadata(m: &SlotMeta) -> R<GeneratedKey> {
    let mut i = 0;
    let d = &m.public_key;
    let mut modulus = vec![];
    let mut exponent = vec![];
    let mut point = vec![];
    while i + 2 <= d.len() {
        let tag = d[i];
        i += 1;
        let mut len = d[i] as usize;
        i += 1;
        if len == 0x81 {
            len = d[i] as usize;
            i += 1;
        } else if len == 0x82 {
            len = ((d[i] as usize) << 8) | d[i + 1] as usize;
            i += 2;
        }
        if i + len > d.len() {
            break;
        }
        let v = d[i..i + len].to_vec();
        i += len;
        match tag {
            0x81 => modulus = v,
            0x82 => exponent = v,
            0x86 => point = v,
            _ => {}
        }
    }
    if modulus.is_empty() && point.is_empty() {
        return Err("slot has no readable public key".into());
    }
    Ok(GeneratedKey { algorithm: m.algorithm, modulus, exponent, point })
}

#[tauri::command]
pub fn public_key_pem(state: State<AppState>, slot: u8) -> R<String> {
    with_piv(&state, |p| {
        let m = p.slot_metadata(slot)?;
        let spki = certs::spki_der(&key_from_metadata(&m)?)?;
        Ok(certs::to_pem("PUBLIC KEY", &spki))
    })
}

#[tauri::command]
pub async fn self_sign(state: State<'_, AppState>, slot: u8, common_name: String, days: i64, pin: String) -> R<CertSummary> {
    with_piv_write(&state, &pin, |p| {
        let m = p.slot_metadata(slot)?;
        if m.algorithm == ALGO_X25519 {
            return Err("X25519 keys cannot sign".into());
        }
        let spki = certs::spki_der(&key_from_metadata(&m)?)?;
        let der = certs::self_signed(p, slot, m.algorithm, &spki, &common_name, days)?;
        p.write_cert(slot, &der)?;
        certs::summarize(&der)
    })
}

#[tauri::command]
pub async fn import_cert(state: State<'_, AppState>, slot: u8, pin: String) -> R<Option<CertSummary>> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Certificate", &["pem", "crt", "cer", "der"])
        .pick_file()
    else {
        return Ok(None);
    };
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let der = certs::cert_to_der(&bytes)?;
    let summary = certs::summarize(&der)?;
    with_piv(&state, |p| {
        p.verify_pin(&pin)?;
        p.write_cert(slot, &der)
    })?;
    Ok(Some(summary))
}

#[derive(Serialize)]
pub struct Imported {
    pub algorithm: String,
    pub cert_written: bool,
    pub file: String,
}

/// Import a .pfx/.p12 (or PEM key bundle) into a slot: private key + certificate.
#[derive(Serialize)]
pub struct PfxPick {
    pub path: String,
    pub file: String,
}

/// Step 1: pick the file. Does not read a PIN or the file password yet.
#[tauri::command]
pub async fn pfx_pick() -> R<Option<PfxPick>> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("PKCS#12 / private key", &["pfx", "p12", "pem", "key"])
        .pick_file()
    else {
        return Ok(None);
    };
    Ok(Some(PfxPick {
        file: path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(),
        path: path.to_string_lossy().into_owned(),
    }))
}

/// Step 2: check the file opens with this password (no device access).
#[tauri::command]
pub async fn pfx_probe(path: String, password: String) -> R<(String, bool)> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let (algo, has_cert) = Piv::probe_key_blob(&bytes, &password)?;
    Ok((crate::piv::algo_name(algo).to_string(), has_cert))
}

/// Step 3: import the already-picked, already-validated file.
#[tauri::command]
pub async fn import_pfx(
    state: State<'_, AppState>,
    slot: u8,
    path: String,
    pin: String,
    password: String,
    pin_policy: String,
    touch_policy: String,
) -> R<Imported> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    // the file already validated in pfx_probe; a -5 from here is the applet
    // refusing the import, not a wrong PKCS#12 password
    Piv::probe_key_blob(&bytes, &password).map_err(|e| format!("file error: {e}"))?;
    // key import is administrative on the PIV applet — authenticate the Admin PIN first
    let (algo, cert_written) = with_piv_write(&state, &pin, |p| {
        p.import_key_blob(slot, &bytes, &password, policy(&pin_policy, false), policy(&touch_policy, true), true)
            .map_err(|e| if e.contains("Wrong password") || e.contains("authentication") {
                "the device refused the key import (Admin PIN not accepted, or the slot is locked)".to_string()
            } else { e })
    })?;
    Ok(Imported {
        algorithm: crate::piv::algo_name(algo).to_string(),
        cert_written,
        file: std::path::Path::new(&path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(),
    })
}

#[tauri::command]
pub async fn export_cert(state: State<'_, AppState>, slot: u8) -> R<bool> {
    let der = with_piv(&state, |p| p.read_cert(slot))?.ok_or("slot has no certificate")?;
    let Some(path) = rfd::FileDialog::new().set_file_name(format!("slot-{slot:02x}.pem")).save_file() else {
        return Ok(false);
    };
    std::fs::write(&path, certs::to_pem("CERTIFICATE", &der)).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn delete_cert(state: State<AppState>, slot: u8, pin: String) -> R<()> {
    delete_slot(state, slot, pin, true, false)
}

/// Delete the certificate and/or the private key in a slot.
#[tauri::command]
pub fn delete_slot(state: State<AppState>, slot: u8, pin: String, cert: bool, key: bool) -> R<()> {
    if !cert && !key { return Err("nothing selected to delete".into()); }
    // Key delete (vendor 00 EE) needs only the user PIN; cert delete (PUT DATA)
    // needs the management key. Do the PIN-gated key delete first, then the
    // mgmt-key-gated cert delete, in one session.
    let mut s2 = state.0.lock().map_err(|_| "state lock poisoned")?;
    let reader = s2.reader.clone();
    let cached = s2.admin_key.clone();
    let piv = s2.piv.as_ref().ok_or("No key connected")?;
    if !reader.is_empty() { let _ = piv.connect(&reader); }   // SELECT PIV
    piv.verify_pin(&pin)?;                                      // user PIN
    // Private-key delete is PIN-only on Token2 (its vendor sequence). Do it first.
    if key {
        piv.delete_key(slot)?;   // verifies internally that the key is gone
        let still = piv.slot_metadata(slot).ok().map(|m| !m.algorithm_name.is_empty()).unwrap_or(false);
        if still { return Err("the private key is still present after the delete".into()); }
    }
    // Certificate delete (PUT DATA) is gated on the management key.
    let mut good = None;
    if cert {
        let mut cands: Vec<Vec<u8>> = Vec::new();
        if let Some(k) = &cached { cands.push(k.clone()); }
        cands.push(hex::decode(DEFAULT_ADMIN_KEY).unwrap());
        for k in &cands { if piv.authenticate(k).is_ok() { good = Some(k.clone()); break; } }
        if good.is_none() { return Err("could not authenticate to delete the certificate (is the PIV PIN correct?)".into()); }
        piv.delete_cert(slot)?;
    }
    let _ = piv;
    if let Some(k) = good { s2.admin_key = Some(k); }
    Ok(())
}

#[tauri::command]
pub fn attest(state: State<AppState>, slot: u8) -> R<String> {
    with_piv(&state, |p| Ok(certs::to_pem("CERTIFICATE", &p.attest(slot)?)))
}

#[tauri::command]
pub async fn verify_attestation(state: State<'_, AppState>, slot: u8) -> R<certs::AttestationReport> {
    with_piv(&state, |p| {
        let leaf = p.attest(slot)?;
        let f9 = p.read_cert(SLOT_ATTESTATION)?.ok_or("no F9 attestation certificate on the key")?;
        let spki = p.slot_metadata(slot).ok().and_then(|m| key_from_metadata(&m).ok()).and_then(|k| certs::spki_der(&k).ok());
        certs::verify_attestation(&leaf, &f9, spki.as_deref())
    })
}

#[tauri::command]
pub fn attestation_chain(state: State<AppState>) -> R<String> {
    with_piv(&state, |p| {
        let der = p.read_cert(SLOT_ATTESTATION)?.ok_or("no attestation certificate on key")?;
        Ok(certs::to_pem("CERTIFICATE", &der))
    })
}

#[tauri::command]
pub async fn save_text(suggested_name: String, text: String) -> R<bool> {
    let Some(path) = rfd::FileDialog::new().set_file_name(suggested_name).save_file() else {
        return Ok(false);
    };
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn reset(state: State<'_, AppState>) -> R<()> {
    with_piv(&state, |p| p.reset())
}

#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn piv_supports_key_delete(state: State<AppState>) -> R<bool> {
    with_piv(&state, |p| {
        let v = p.version().unwrap_or_default();
        // parse "a.b.c" and compare to 5.7.0
        let parts: Vec<u32> = v.split('.').filter_map(|x| x.trim().parse().ok()).collect();
        let ok = match parts.as_slice() {
            [a, b, ..] => *a > 5 || (*a == 5 && *b >= 7),
            _ => false,
        };
        Ok(ok)
    })
}

/// Autodetect HID-only Token2 keys that expose the OTP applet over the CTAPHID
/// tunnel (PIN+ Release2, no CCID). Returns [{path, product, serial}] for keys
/// that answered the OTP tunnel probe. Empty when none / not applicable.
#[derive(serde::Serialize)]
pub struct OtpHidDevice { pub path: String, pub product: String, pub serial: String }

#[tauri::command]
pub fn otp_hid_detect() -> R<Vec<OtpHidDevice>> {
    let mut out = Vec::new();
    for (path, product) in crate::t2otp_hid::list_otp_hid_devices() {
        if let Some(serial) = crate::t2otp_hid::probe_otp_hid(&path) {
            out.push(OtpHidDevice { path, product, serial });
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn otp_hid_debug() -> R<String> { Ok(crate::t2otp_hid::probe_otp_hid_debug()) }

/// Read whether fingerprint-for-PIV-login is available/bound on this key.
/// Returns true if the biometric (OCC 0x96) reference responds as usable.
#[derive(serde::Serialize)]
pub struct FpBinding { pub supported: bool, pub enabled: bool }

#[tauri::command]
pub fn piv_fp_binding_status(state: State<AppState>) -> R<FpBinding> {
    with_piv(&state, |p| Ok(FpBinding {
        supported: p.fingerprint_binding_supported(),
        enabled: p.fingerprint_binding_status().unwrap_or(false),
    }))
}

/// Enable or disable fingerprint-for-PIV-login (the Token2 biometric-PIV flag).
/// Mirrors the Companion app: VERIFY 0x96 with 03 09 <01|00> <PIN> FF FF.
/// Requires a fingerprint already enrolled (FIDO -> Fingerprints) to enable.
#[tauri::command]
pub fn piv_set_fp_binding(state: State<AppState>, pin: String, enable: bool) -> R<()> {
    with_piv(&state, |p| p.set_fingerprint_binding(&pin, enable))
}
