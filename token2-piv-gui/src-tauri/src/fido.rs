//! FIDO2 management through upstream libfido2 built with USE_PCSC: Token2 keys
//! expose the FIDO applet over CCID, so they enumerate as `pcsc://` devices on
//! USB and NFC alike, with no HID access (and no elevation) required.

#![allow(non_camel_case_types, dead_code)]

use serde::Serialize;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;
use tauri::State;

// ---------------------------------------------------------------- FFI ------
#[repr(C)] pub struct fido_dev_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_dev_info_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_cbor_info_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_cred_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_credman_metadata_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_credman_rp_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_credman_rk_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_bio_template_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_bio_template_array_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_bio_enroll_t { _p: [u8; 0] }
#[repr(C)] pub struct fido_bio_info_t { _p: [u8; 0] }

const FIDO_OK: c_int = 0;
const FIDO_ERR_PIN_INVALID: c_int = 0x31;
const FIDO_ERR_PIN_BLOCKED: c_int = 0x32;
const FIDO_ERR_PIN_AUTH_BLOCKED: c_int = 0x34;
const FIDO_ERR_PIN_REQUIRED: c_int = 0x36;
const FIDO_ERR_ACTION_TIMEOUT: c_int = 0x3a;
const FIDO_ERR_NOT_ALLOWED: c_int = 0x30;
const FIDO_ERR_INVALID_COMMAND: c_int = 0x01;

extern "C" {
    fn fido_init(flags: c_int);
    fn fido_set_log_handler(handler: Option<extern "C" fn(*const c_char)>);
    fn fido_strerr(n: c_int) -> *const c_char;
    // enumeration
    fn fido_dev_info_new(n: usize) -> *mut fido_dev_info_t;
    fn fido_dev_info_free(list: *mut *mut fido_dev_info_t, n: usize);
    fn fido_dev_info_manifest(list: *mut fido_dev_info_t, ilen: usize, olen: *mut usize) -> c_int;
    fn fido_dev_info_ptr(list: *const fido_dev_info_t, i: usize) -> *const fido_dev_info_t;
    fn fido_dev_info_path(di: *const fido_dev_info_t) -> *const c_char;
    fn fido_dev_info_manufacturer_string(di: *const fido_dev_info_t) -> *const c_char;
    fn fido_dev_info_product_string(di: *const fido_dev_info_t) -> *const c_char;
    fn fido_dev_info_vendor(di: *const fido_dev_info_t) -> i16;
    fn fido_dev_info_product(di: *const fido_dev_info_t) -> i16;
    // device
    fn fido_dev_new() -> *mut fido_dev_t;
    fn fido_dev_free(dev: *mut *mut fido_dev_t);
    fn fido_dev_open(dev: *mut fido_dev_t, path: *const c_char) -> c_int;
    fn fido_dev_token2_apdu(dev: *mut fido_dev_t, apdu: *const u8, apdu_len: usize, rx: *mut u8, rx_len: *mut usize) -> c_int;
    fn fido_dev_close(dev: *mut fido_dev_t) -> c_int;
    fn fido_dev_cancel(dev: *mut fido_dev_t) -> c_int;
    fn fido_dev_is_fido2(dev: *const fido_dev_t) -> bool;
    fn fido_dev_has_pin(dev: *const fido_dev_t) -> bool;
    fn fido_dev_has_uv(dev: *const fido_dev_t) -> bool;
    fn fido_dev_supports_pin(dev: *const fido_dev_t) -> bool;
    fn fido_dev_supports_uv(dev: *const fido_dev_t) -> bool;
    fn fido_dev_supports_credman(dev: *const fido_dev_t) -> bool;
    fn fido_dev_supports_permissions(dev: *const fido_dev_t) -> bool;
    fn fido_dev_get_retry_count(dev: *mut fido_dev_t, n: *mut c_int) -> c_int;
    fn fido_dev_get_uv_retry_count(dev: *mut fido_dev_t, n: *mut c_int) -> c_int;
    fn fido_dev_set_pin(dev: *mut fido_dev_t, pin: *const c_char, oldpin: *const c_char) -> c_int;
    fn fido_dev_reset(dev: *mut fido_dev_t) -> c_int;
    fn fido_dev_force_pin_change(dev: *mut fido_dev_t, pin: *const c_char) -> c_int;
    fn fido_dev_toggle_always_uv(dev: *mut fido_dev_t, pin: *const c_char) -> c_int;
    fn fido_dev_set_pin_minlen(dev: *mut fido_dev_t, len: usize, pin: *const c_char) -> c_int;
    // info
    fn fido_cbor_info_new() -> *mut fido_cbor_info_t;
    fn fido_cbor_info_free(ci: *mut *mut fido_cbor_info_t);
    fn fido_dev_get_cbor_info(dev: *mut fido_dev_t, ci: *mut fido_cbor_info_t) -> c_int;
    fn fido_cbor_info_versions_ptr(ci: *const fido_cbor_info_t) -> *mut *mut c_char;
    fn fido_cbor_info_versions_len(ci: *const fido_cbor_info_t) -> usize;
    fn fido_cbor_info_extensions_ptr(ci: *const fido_cbor_info_t) -> *mut *mut c_char;
    fn fido_cbor_info_extensions_len(ci: *const fido_cbor_info_t) -> usize;
    fn fido_cbor_info_transports_ptr(ci: *const fido_cbor_info_t) -> *mut *mut c_char;
    fn fido_cbor_info_transports_len(ci: *const fido_cbor_info_t) -> usize;
    fn fido_cbor_info_options_name_ptr(ci: *const fido_cbor_info_t) -> *mut *mut c_char;
    fn fido_cbor_info_options_value_ptr(ci: *const fido_cbor_info_t) -> *const bool;
    fn fido_cbor_info_options_len(ci: *const fido_cbor_info_t) -> usize;
    fn fido_cbor_info_aaguid_ptr(ci: *const fido_cbor_info_t) -> *const u8;
    fn fido_cbor_info_aaguid_len(ci: *const fido_cbor_info_t) -> usize;
    fn fido_cbor_info_fwversion(ci: *const fido_cbor_info_t) -> u64;
    fn fido_cbor_info_minpinlen(ci: *const fido_cbor_info_t) -> u64;
    fn fido_cbor_info_maxcredcntlst(ci: *const fido_cbor_info_t) -> u64;
    fn fido_cbor_info_maxlargeblob(ci: *const fido_cbor_info_t) -> u64;
    fn fido_cbor_info_rk_remaining(ci: *const fido_cbor_info_t) -> i64;
    fn fido_cbor_info_new_pin_required(ci: *const fido_cbor_info_t) -> bool;
    fn fido_cbor_info_algorithm_count(ci: *const fido_cbor_info_t) -> usize;
    fn fido_cbor_info_algorithm_type(ci: *const fido_cbor_info_t, i: usize) -> *const c_char;
    fn fido_cbor_info_algorithm_cose(ci: *const fido_cbor_info_t, i: usize) -> c_int;
    // credential management
    fn fido_credman_metadata_new() -> *mut fido_credman_metadata_t;
    fn fido_credman_metadata_free(m: *mut *mut fido_credman_metadata_t);
    fn fido_credman_get_dev_metadata(dev: *mut fido_dev_t, m: *mut fido_credman_metadata_t, pin: *const c_char) -> c_int;
    fn fido_credman_rk_existing(m: *const fido_credman_metadata_t) -> u64;
    fn fido_credman_rk_remaining(m: *const fido_credman_metadata_t) -> u64;
    fn fido_credman_rp_new() -> *mut fido_credman_rp_t;
    fn fido_credman_rp_free(rp: *mut *mut fido_credman_rp_t);
    fn fido_credman_get_dev_rp(dev: *mut fido_dev_t, rp: *mut fido_credman_rp_t, pin: *const c_char) -> c_int;
    fn fido_credman_rp_count(rp: *const fido_credman_rp_t) -> usize;
    fn fido_credman_rp_id(rp: *const fido_credman_rp_t, i: usize) -> *const c_char;
    fn fido_credman_rp_name(rp: *const fido_credman_rp_t, i: usize) -> *const c_char;
    fn fido_credman_rk_new() -> *mut fido_credman_rk_t;
    fn fido_credman_rk_free(rk: *mut *mut fido_credman_rk_t);
    fn fido_credman_get_dev_rk(dev: *mut fido_dev_t, rp_id: *const c_char, rk: *mut fido_credman_rk_t, pin: *const c_char) -> c_int;
    fn fido_credman_rk_count(rk: *const fido_credman_rk_t) -> usize;
    fn fido_credman_rk(rk: *const fido_credman_rk_t, i: usize) -> *const fido_cred_t;
    fn fido_credman_del_dev_rk(dev: *mut fido_dev_t, cred_id: *const u8, len: usize, pin: *const c_char) -> c_int;
    fn fido_cred_new() -> *mut fido_cred_t;
    fn fido_cred_free(c: *mut *mut fido_cred_t);
    fn fido_cred_set_id(c: *mut fido_cred_t, id: *const u8, len: usize) -> c_int;
    fn fido_cred_set_user(c: *mut fido_cred_t, user_id: *const u8, len: usize, name: *const c_char, display_name: *const c_char, icon: *const c_char) -> c_int;
    fn fido_credman_set_dev_rk(dev: *mut fido_dev_t, c: *mut fido_cred_t, pin: *const c_char) -> c_int;
    fn fido_cred_id_ptr(c: *const fido_cred_t) -> *const u8;
    fn fido_cred_id_len(c: *const fido_cred_t) -> usize;
    fn fido_cred_user_name(c: *const fido_cred_t) -> *const c_char;
    fn fido_cred_display_name(c: *const fido_cred_t) -> *const c_char;
    fn fido_cred_user_id_ptr(c: *const fido_cred_t) -> *const u8;
    fn fido_cred_user_id_len(c: *const fido_cred_t) -> usize;
    fn fido_cred_prot(c: *const fido_cred_t) -> c_int;
    fn fido_cred_type(c: *const fido_cred_t) -> c_int;
    // biometrics
    fn fido_bio_info_new() -> *mut fido_bio_info_t;
    fn fido_bio_info_free(i: *mut *mut fido_bio_info_t);
    fn fido_bio_dev_get_info(dev: *mut fido_dev_t, i: *mut fido_bio_info_t) -> c_int;
    fn fido_bio_info_max_samples(i: *const fido_bio_info_t) -> u8;
    fn fido_bio_info_type(i: *const fido_bio_info_t) -> u8;
    fn fido_bio_template_array_new() -> *mut fido_bio_template_array_t;
    fn fido_bio_template_array_free(ta: *mut *mut fido_bio_template_array_t);
    fn fido_bio_dev_get_template_array(dev: *mut fido_dev_t, ta: *mut fido_bio_template_array_t, pin: *const c_char) -> c_int;
    fn fido_bio_template_array_count(ta: *const fido_bio_template_array_t) -> usize;
    fn fido_bio_template(ta: *const fido_bio_template_array_t, i: usize) -> *const fido_bio_template_t;
    fn fido_bio_template_new() -> *mut fido_bio_template_t;
    fn fido_bio_template_free(t: *mut *mut fido_bio_template_t);
    fn fido_bio_template_name(t: *const fido_bio_template_t) -> *const c_char;
    fn fido_bio_template_id_ptr(t: *const fido_bio_template_t) -> *const u8;
    fn fido_bio_template_id_len(t: *const fido_bio_template_t) -> usize;
    fn fido_bio_template_set_id(t: *mut fido_bio_template_t, id: *const u8, len: usize) -> c_int;
    fn fido_bio_template_set_name(t: *mut fido_bio_template_t, name: *const c_char) -> c_int;
    fn fido_bio_dev_set_template_name(dev: *mut fido_dev_t, t: *const fido_bio_template_t, pin: *const c_char) -> c_int;
    fn fido_bio_dev_enroll_remove(dev: *mut fido_dev_t, t: *const fido_bio_template_t, pin: *const c_char) -> c_int;
    fn fido_bio_enroll_new() -> *mut fido_bio_enroll_t;
    fn fido_bio_enroll_free(e: *mut *mut fido_bio_enroll_t);
    fn fido_bio_dev_enroll_begin(dev: *mut fido_dev_t, t: *mut fido_bio_template_t, e: *mut fido_bio_enroll_t, timeout_ms: u32, pin: *const c_char) -> c_int;
    fn fido_bio_dev_enroll_continue(dev: *mut fido_dev_t, t: *const fido_bio_template_t, e: *mut fido_bio_enroll_t, timeout_ms: u32) -> c_int;
    fn fido_bio_dev_enroll_cancel(dev: *mut fido_dev_t) -> c_int;
    fn fido_bio_enroll_last_status(e: *const fido_bio_enroll_t) -> u8;
    fn fido_bio_enroll_remaining_samples(e: *const fido_bio_enroll_t) -> u8;
}

// --------------------------------------------------------------- helpers ---
type R<T> = Result<T, String>;

fn cstr(p: *const c_char) -> String {
    if p.is_null() { String::new() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() }
}
fn strv(p: *mut *mut c_char, n: usize) -> Vec<String> {
    (0..n).map(|i| cstr(unsafe { *p.add(i) })).collect()
}
fn rc(r: c_int) -> R<()> {
    if r == FIDO_OK {
        return Ok(());
    }
    let base = cstr(unsafe { fido_strerr(r) });
    Err(match r {
        FIDO_ERR_PIN_INVALID => "wrong PIN".into(),
        FIDO_ERR_PIN_BLOCKED => "PIN is blocked — the device must be reset".into(),
        FIDO_ERR_PIN_AUTH_BLOCKED => "PIN authentication temporarily blocked — unplug and reinsert the key".into(),
        0x35 => "no PIN is set on this key yet — set a FIDO PIN first (FIDO → PIN)".into(),
        FIDO_ERR_PIN_REQUIRED => "a PIN is required for this operation".into(),
        0x33 => "PIN authentication failed on this key (it may not allow this command over this transport)".into(),
        0x37 => "the new PIN doesn't meet the key's PIN policy. Token2 PIN+ keys reject weak PINs (sequences like 123456, repeats like 111111, or palindromes) and PINs below the minimum length. Choose a less predictable PIN. See the PIN complexity rules at token2.com.".into(),
        0x25 => "unsupported algorithm".into(),
        0x26 => "this operation is not permitted".into(),
        0x27 => "the key is out of storage for new credentials".into(),
        0x28 => "the key's credential storage is full".into(),
        0x18 => "invalid data in the request".into(),
        0x19 => "the operation was already in progress".into(),
        0x11 => "the request was cancelled".into(),
        0x3b => "the user declined or the operation was cancelled".into(),
        0x3e => "no fingerprints are enrolled — enroll one first (FIDO → Fingerprints)".into(),
        FIDO_ERR_ACTION_TIMEOUT => "timed out waiting for the key (touch it when it blinks)".into(),
        FIDO_ERR_NOT_ALLOWED => "not allowed by the device (for a reset: unplug the key, plug it back in and reset within 10 seconds, then touch it)".into(),
        FIDO_ERR_INVALID_COMMAND => "this device does not support that command".into(),
        0x2b => "option not supported".into(),
        0x2c => "the requested option is not supported by this key".into(),
        _ => format!("{base} (0x{r:02x})"),
    })
}
/// Empty PIN means "use built-in user verification" (fingerprint): libfido2
/// obtains a UV token instead of a PIN token when pin == NULL.
fn fido_log_rk_err(r: c_int) { if std::env::var("T2_FIDO_DEBUG").is_ok() { eprintln!("credman_get_dev_rk failed: {r}"); } }
fn pin_or_uv(pin: &str) -> R<Option<CString>> {
    if pin.is_empty() { Ok(None) } else { CString::new(pin).map(Some).map_err(|e| e.to_string()) }
}
fn opt_pin(pin: &Option<String>) -> Option<CString> {
    pin.as_ref().filter(|p| !p.is_empty()).and_then(|p| CString::new(p.as_str()).ok())
}
fn pin_ptr(c: &Option<CString>) -> *const c_char {
    c.as_ref().map(|c| c.as_ptr()).unwrap_or(ptr::null())
}

static INIT: std::sync::Once = std::sync::Once::new();

/// Read the serial from a HID FIDO key (e.g. Feitian ePass / PIN+ firmware) using
/// the vendor APDU 80 E3 EE 80 00 over CTAPHID_MSG. The reply is a TLV where tag 82
/// holds the serial as ASCII digits. Returns "" if unavailable.
fn hid_serial(path: &str) -> String {
    let dev = unsafe { fido_dev_new() };
    if dev.is_null() { return String::new(); }
    let mut out = String::new();
    if let Ok(c) = CString::new(path) {
        if unsafe { fido_dev_open(dev, c.as_ptr()) } == FIDO_OK {
            let apdu = [0x80u8, 0xe3, 0xee, 0x80, 0x00];
            let mut rx = vec![0u8; 512];
            let mut rx_len: usize = rx.len();
            let r = unsafe { fido_dev_token2_apdu(dev, apdu.as_ptr(), apdu.len(), rx.as_mut_ptr(), &mut rx_len) };
            if r == FIDO_OK && rx_len >= 2 && rx[0] == 0x80 {
                let body = &rx[2..rx_len.min(rx.len())];
                let mut j = 0usize;
                while j + 2 <= body.len() {
                    let t = body[j]; let l = body[j+1] as usize;
                    if j + 2 + l > body.len() { break; }
                    if t == 0x82 {
                        out = String::from_utf8_lossy(&body[j+2..j+2+l]).trim_matches(char::from(0)).to_string();
                        break;
                    }
                    j += 2 + l;
                }
            }
            let mut d = dev; unsafe { fido_dev_close(d); fido_dev_free(&mut d); }
            return out;
        }
    }
    let mut d = dev; unsafe { fido_dev_free(&mut d); }
    out
}

extern "C" fn t2_fido_log(msg: *const c_char) {
    if msg.is_null() { return; }
    let s = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    if let Ok(path) = std::env::var("T2_FIDO_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{}", s.trim_end());
        }
    }
}
fn ensure_init() {
    // T2_FIDO_DEBUG=1 turns on libfido2's own debug log. GUI apps have no console,
    // so also set T2_FIDO_LOG=<path> to capture it to a file.
    // exe from a console to see it).
    INIT.call_once(|| unsafe {
        let dbg = std::env::var_os("T2_FIDO_DEBUG").is_some() || std::env::var_os("T2_FIDO_LOG").is_some();
        fido_init(if dbg { 1 } else { 0 });
        if std::env::var_os("T2_FIDO_LOG").is_some() { fido_set_log_handler(Some(t2_fido_log)); }
    });
}

// ------------------------------------------------------------- session -----
pub struct FidoDev {
    dev: *mut fido_dev_t,
    path: String,
}
unsafe impl Send for FidoDev {}
impl Drop for FidoDev {
    fn drop(&mut self) {
        unsafe {
            fido_dev_close(self.dev);
            fido_dev_free(&mut self.dev);
        }
    }
}

pub struct Enrollment {
    template: *mut fido_bio_template_t,
    enroll: *mut fido_bio_enroll_t,
    name: String,
    pin: String,
}
unsafe impl Send for Enrollment {}
impl Drop for Enrollment {
    fn drop(&mut self) {
        unsafe {
            fido_bio_template_free(&mut self.template);
            fido_bio_enroll_free(&mut self.enroll);
        }
    }
}

#[derive(Default)]
pub struct FidoSession {
    dev: Option<FidoDev>,
    enrollment: Option<Enrollment>,
}
pub struct FidoState(pub Mutex<FidoSession>);

fn with_dev<T>(state: &State<FidoState>, f: impl FnOnce(*mut fido_dev_t) -> R<T>) -> R<T> {
    let s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let d = s.dev.as_ref().ok_or("No FIDO2 device connected")?;
    f(d.dev)
}

// ------------------------------------------------------------- commands ----
#[derive(Serialize, Clone)]
pub struct FidoDevice {
    pub path: String,
    pub manufacturer: String,
    pub product: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub transport: String, // "ccid" (PC/SC: USB smart-card channel or NFC) | "hid"
}

#[tauri::command]
pub fn fido_list_devices() -> R<Vec<FidoDevice>> {
    ensure_init();
    let cap = 64usize;
    let list = unsafe { fido_dev_info_new(cap) };
    if list.is_null() {
        return Err("out of memory".into());
    }
    let mut n = 0usize;
    // The manifest fills HID devices even when the PC/SC sub-scan fails (e.g. no
    // readers -> 0x8010002E). libfido2's own CLI ignores the return code and
    // uses whatever devices came back, so we do the same and only error when
    // nothing at all was found.
    let r = unsafe { fido_dev_info_manifest(list, cap, &mut n) };
    let mut out = Vec::new();
    for i in 0..n {
        let di = unsafe { fido_dev_info_ptr(list, i) };
        let path = cstr(unsafe { fido_dev_info_path(di) });
        if path.starts_with("windows://") {
            continue; // Windows Hello pseudo-device
        }
        out.push(FidoDevice {
            transport: if path.starts_with("pcsc://") || path.starts_with("nfc:") { "ccid".into() } else { "hid".into() },
            manufacturer: cstr(unsafe { fido_dev_info_manufacturer_string(di) }),
            product: cstr(unsafe { fido_dev_info_product_string(di) }),
            vendor_id: unsafe { fido_dev_info_vendor(di) } as u16,
            product_id: unsafe { fido_dev_info_product(di) } as u16,
            path,
        });
    }
    let mut l = list;
    unsafe { fido_dev_info_free(&mut l, cap) };
    if out.is_empty() && r != FIDO_OK {
        return rc(r).map(|_| out);
    }
    Ok(out)
}

#[derive(Serialize, Clone, Default)]
pub struct FidoInfo {
    pub path: String,
    pub transport: String,
    pub is_fido2: bool,
    pub versions: Vec<String>,
    pub extensions: Vec<String>,
    pub transports: Vec<String>,
    pub options: Vec<(String, bool)>,
    pub aaguid: String,
    pub firmware: String,
    pub min_pin_len: u64,
    pub max_cred_count_in_list: u64,
    pub supports_largeblob: bool,
    pub rk_remaining: i64,
    pub has_pin: bool,
    pub has_uv: bool,
    pub supports_pin: bool,
    pub supports_uv: bool,
    pub supports_config: bool,
    pub supports_permissions: bool,
    pub supports_credman: bool,
    pub supports_bio: bool,
    pub always_uv: bool,
    pub new_pin_required: bool,
    pub pin_retries: i32,
    pub uv_retries: i32,
    pub algorithms: Vec<String>,
}

fn read_info(dev: *mut fido_dev_t, path: &str) -> R<FidoInfo> {
    let mut info = FidoInfo {
        path: path.to_string(),
        transport: if path.starts_with("pcsc://") || path.starts_with("nfc:") { "ccid".into() } else { "hid".into() },
        ..Default::default()
    };
    unsafe {
        info.is_fido2 = fido_dev_is_fido2(dev);
        info.has_pin = fido_dev_has_pin(dev);
        info.has_uv = fido_dev_has_uv(dev);
        info.supports_pin = fido_dev_supports_pin(dev);
        info.supports_permissions = fido_dev_supports_permissions(dev);
        info.supports_uv = fido_dev_supports_uv(dev);
        info.supports_credman = fido_dev_supports_credman(dev);
        let mut n: c_int = -1;
        if fido_dev_get_retry_count(dev, &mut n) == FIDO_OK { info.pin_retries = n; } else { info.pin_retries = -1; }
        let mut u: c_int = -1;
        if fido_dev_get_uv_retry_count(dev, &mut u) == FIDO_OK { info.uv_retries = u; } else { info.uv_retries = -1; }
    }
    if !info.is_fido2 {
        return Ok(info);
    }
    let ci = unsafe { fido_cbor_info_new() };
    let r = unsafe { fido_dev_get_cbor_info(dev, ci) };
    if r == FIDO_OK {
        unsafe {
            info.versions = strv(fido_cbor_info_versions_ptr(ci), fido_cbor_info_versions_len(ci));
            info.extensions = strv(fido_cbor_info_extensions_ptr(ci), fido_cbor_info_extensions_len(ci));
            info.transports = strv(fido_cbor_info_transports_ptr(ci), fido_cbor_info_transports_len(ci));
            let names = strv(fido_cbor_info_options_name_ptr(ci), fido_cbor_info_options_len(ci));
            let vals = fido_cbor_info_options_value_ptr(ci);
            info.options = names.iter().enumerate().map(|(i, n)| (n.clone(), *vals.add(i))).collect();
            // Some 2.1 keys under-report via fido_dev_supports_credman; trust the
            // advertised options too (credMgmt / credentialMgmtPreview).
            if info.options.iter().any(|(k, v)| (k == "credMgmt" || k == "credentialMgmtPreview") && *v) {
                info.supports_credman = true;
            }
            let ag = fido_cbor_info_aaguid_ptr(ci);
            let al = fido_cbor_info_aaguid_len(ci);
            if !ag.is_null() && al == 16 {
                let b = std::slice::from_raw_parts(ag, 16);
                let h = hex::encode(b);
                info.aaguid = format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]);
            }
            let fw = fido_cbor_info_fwversion(ci);
            info.firmware = if fw == 0 { String::new() } else { format!("{}.{}.{}", (fw >> 16) & 0xff, (fw >> 8) & 0xff, fw & 0xff) };
            info.min_pin_len = fido_cbor_info_minpinlen(ci);
            info.max_cred_count_in_list = fido_cbor_info_maxcredcntlst(ci);
            info.supports_largeblob = fido_cbor_info_maxlargeblob(ci) > 0;
            info.rk_remaining = fido_cbor_info_rk_remaining(ci);
            info.new_pin_required = fido_cbor_info_new_pin_required(ci);
            for i in 0..fido_cbor_info_algorithm_count(ci) {
                let t = cstr(fido_cbor_info_algorithm_type(ci, i));
                let c = fido_cbor_info_algorithm_cose(ci, i);
                let name = match c { -7 => "ES256", -8 => "EdDSA", -35 => "ES384", -36 => "ES512", -257 => "RS256", -47 => "ES256K", _ => "" };
                info.algorithms.push(if name.is_empty() { format!("{t} (COSE {c})") } else { name.to_string() });
            }
        }
        info.always_uv = info.options.iter().any(|(k, v)| k == "alwaysUv" && *v);
        // authenticatorConfig (setMinPINLength / toggleAlwaysUv / forceChangePin)
        // is usable only when the applet advertises the authnrCfg option AND can
        // issue a pinUvAuthToken with the authenticatorConfig permission.
        let has_authnr_cfg = info.options.iter().any(|(k, _)| k == "authnrCfg");
        info.supports_config = has_authnr_cfg && info.supports_permissions;
        info.supports_bio = info.options.iter().any(|(k, _)| k == "bioEnroll" || k == "userVerificationMgmtPreview");
    }
    let mut c = ci;
    unsafe { fido_cbor_info_free(&mut c) };
    rc(r)?;
    Ok(info)
}

#[tauri::command]
pub async fn fido_connect(state: State<'_, FidoState>, path: String) -> R<FidoInfo> {
    ensure_init();
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    s.dev = None;
    s.enrollment = None;
    let dev = unsafe { fido_dev_new() };
    if dev.is_null() {
        return Err("out of memory".into());
    }
    let c = CString::new(path.as_str()).map_err(|e| e.to_string())?;
    let r = unsafe { fido_dev_open(dev, c.as_ptr()) };
    if r != FIDO_OK {
        let mut d = dev;
        unsafe { fido_dev_free(&mut d) };
        return rc(r).map(|_| unreachable!()).map_err(|e| {
            if cfg!(windows) && !path.starts_with("pcsc://") {
                format!("{e}. Raw HID FIDO devices need an elevated process on Windows; pick the smart-card (pcsc://) entry for a Token2 key, which works without elevation.")
            } else {
                e
            }
        });
    }
    let info = read_info(dev, &path)?;
    s.dev = Some(FidoDev { dev, path });
    Ok(info)
}

#[tauri::command]
pub fn fido_disconnect(state: State<FidoState>) -> R<()> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    s.enrollment = None;
    s.dev = None;
    Ok(())
}

#[tauri::command]
pub async fn fido_info(state: State<'_, FidoState>) -> R<FidoInfo> {
    let s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let d = s.dev.as_ref().ok_or("No FIDO2 device connected")?;
    read_info(d.dev, &d.path)
}

#[tauri::command]
pub async fn fido_set_pin(state: State<'_, FidoState>, new_pin: String, old_pin: Option<String>) -> R<()> {
    let n = CString::new(new_pin).map_err(|e| e.to_string())?;
    let o = opt_pin(&old_pin);
    with_dev(&state, |d| rc(unsafe { fido_dev_set_pin(d, n.as_ptr(), pin_ptr(&o)) }))
}

#[tauri::command]
pub async fn fido_set_min_pin_len(state: State<'_, FidoState>, len: usize, pin: String) -> R<()> {
    let p = pin_or_uv(&pin)?;
    with_dev(&state, |d| rc(unsafe { fido_dev_set_pin_minlen(d, len, pin_ptr(&p)) }))
}

#[tauri::command]
pub async fn fido_force_pin_change(state: State<'_, FidoState>, pin: String) -> R<()> {
    let p = pin_or_uv(&pin)?;
    with_dev(&state, |d| rc(unsafe { fido_dev_force_pin_change(d, pin_ptr(&p)) }))
}

#[tauri::command]
pub async fn fido_toggle_always_uv(state: State<'_, FidoState>, pin: String) -> R<()> {
    {
        let s = state.0.lock().map_err(|_| "state lock poisoned")?;
        let d = s.dev.as_ref().ok_or("No FIDO2 device connected")?.dev;
        if !unsafe { fido_dev_has_pin(d) } && !unsafe { fido_dev_has_uv(d) } {
            return Err("set a FIDO PIN (or enroll a fingerprint) before enabling always-UV".into());
        }
    }
    let p = pin_or_uv(&pin)?;
    with_dev(&state, |d| {
        rc(unsafe { fido_dev_toggle_always_uv(d, pin_ptr(&p)) }).map_err(|e| {
            if e.contains("0x2c") || e.contains("unsupported option") || e.contains("rejected the option") {
                "this key does not support the always-UV option".to_string()
            } else if e.contains("0x36") || e.contains("PIN required") {
                "set a PIN (and, on a bio key, enroll a fingerprint) before enabling always-UV".to_string()
            } else { e }
        })
    })
}

/// Factory reset. Most authenticators require this within ~10 s of power-up
/// and a touch; the UI explains that.
#[tauri::command]
pub async fn fido_reset(state: State<'_, FidoState>) -> R<()> {
    with_dev(&state, |d| rc(unsafe { fido_dev_reset(d) }))
}

// ---- per-key note via the bundled token2-fido2-token CLI ----
// The CLI does the full CTAP/PIN/largeBlob exchange correctly over pcsc://slotN
// (proven working). We shell out to it rather than reimplement the crypto. The
// note text is framed (magic + length) and stored as the largeBlob entry,
// encrypted by the CLI under a FIXED key file, so it round-trips for us. We
// release the libfido2 session first so the CLI is the sole PC/SC owner.

const NOTE_MAGIC: [u8; 2] = [0x54, 0x4e]; // "TN"
pub const NOTE_MAX_BYTES: usize = 56;

fn note_frame(text: &str) -> Vec<u8> {
    let mut body: &[u8] = text.as_bytes();
    if body.len() > NOTE_MAX_BYTES {
        let mut e = NOTE_MAX_BYTES; while e > 0 && (body[e] & 0xc0) == 0x80 { e -= 1; } body = &body[..e];
    }
    let len = body.len() as u16;
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&NOTE_MAGIC);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(body);
    // pad with incompressible random to a fixed 64 bytes (keeps the CLI's
    // compressed+encrypted blob in the size window that writes over CCID)
    use rand::RngCore;
    while out.len() < 64 { let mut b = [0u8; 8]; rand::thread_rng().fill_bytes(&mut b);
        for x in b { if out.len() < 64 { out.push(x); } } }
    out
}
fn note_unframe(buf: &[u8]) -> Option<String> {
    if buf.len() < 4 || buf[0] != NOTE_MAGIC[0] || buf[1] != NOTE_MAGIC[1] { return None; }
    let len = u16::from_le_bytes([buf[2], buf[3]]) as usize;
    if 4 + len > buf.len() { return None; }
    Some(String::from_utf8_lossy(&buf[4..4 + len]).into_owned())
}

/// Locate the bundled token2-fido2-token(.exe) next to our own executable.
fn cli_path() -> R<std::path::PathBuf> {
    let dir = std::env::current_exe().map_err(|e| e.to_string())?
        .parent().ok_or("no exe dir")?.to_path_buf();
    let name = if cfg!(windows) { "token2-fido2-token.exe" } else { "token2-fido2-token" };
    let p = dir.join(name);
    if p.exists() { Ok(p) } else { Err(format!("{name} not found next to the app")) }
}

/// A stable per-key temp dir + the fixed largeBlob key file (base64 32 bytes).
/// The key is derived deterministically so reads and writes use the same one.
fn note_keyfile() -> R<std::path::PathBuf> {
    let dir = std::env::temp_dir().join("token2-key-manager");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let kf = dir.join("note.key");
    if !kf.exists() {
        // fixed 32-byte app key, base64 — obfuscation only (note is not secret)
        const K: [u8; 32] = [
            0x54,0x6f,0x6b,0x65,0x6e,0x32,0x4b,0x4d,0x67,0x72,0x4e,0x6f,0x74,0x65,0x76,0x31,
            0x9a,0x3f,0xc1,0x05,0x7e,0x2b,0x44,0xd8,0x61,0x90,0xbc,0x3d,0x12,0xe7,0x58,0xaf,
        ];
        let b64 = base32_free_b64(&K);
        std::fs::write(&kf, b64).map_err(|e| e.to_string())?;
    }
    Ok(kf)
}
// standard base64 (the CLI expects base64 for -k)
fn base32_free_b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for ch in data.chunks(3) {
        let b = [ch[0], *ch.get(1).unwrap_or(&0), *ch.get(2).unwrap_or(&0)];
        out.push(T[(b[0] >> 2) as usize] as char);
        out.push(T[(((b[0] & 3) << 4) | (b[1] >> 4)) as usize] as char);
        out.push(if ch.len() > 1 { T[(((b[1] & 15) << 2) | (b[2] >> 6)) as usize] as char } else { '=' });
        out.push(if ch.len() > 2 { T[(b[2] & 63) as usize] as char } else { '=' });
    }
    out
}

/// Run the CLI with args, piping `pin` (if any) to stdin for the PIN prompt.
fn run_cli(args: &[&std::ffi::OsStr], pin: Option<&str>) -> R<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let cli = cli_path()?;
    let mut cmd = Command::new(&cli);
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)] { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); } // CREATE_NO_WINDOW
    let mut child = cmd.spawn().map_err(|e| format!("spawn CLI: {e}"))?;
    // Write the PIN and CLOSE stdin (take() -> dropped at end of block) so the CLI
    // sees EOF and stops waiting for input. Leaving stdin open deadlocks the child.
    {
        let mut si = child.stdin.take();
        if let (Some(si), Some(p)) = (si.as_mut(), pin) {
            let _ = si.write_all(p.as_bytes());
            let _ = si.write_all(b"\n");
            let _ = si.flush();
        }
        // si (and the stdin pipe) drops here -> EOF to the child
    }
    // Watchdog: don't let a stuck CLI hang the app. Drain output on threads and
    // poll for exit up to ~20s (a touch/PIN exchange is far faster); kill on timeout.
    use std::io::Read;
    let mut so = child.stdout.take();
    let mut se = child.stderr.take();
    let th_o = std::thread::spawn(move || { let mut v = Vec::new(); if let Some(s) = so.as_mut() { let _ = s.read_to_end(&mut v); } v });
    let th_e = std::thread::spawn(move || { let mut v = Vec::new(); if let Some(s) = se.as_mut() { let _ = s.read_to_end(&mut v); } v });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(st) => break st,
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("the key did not respond in time (is it plugged in / PIN correct?)".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    };
    let out_stdout = th_o.join().unwrap_or_default();
    let out_stderr = th_e.join().unwrap_or_default();
    struct Out { status: std::process::ExitStatus, stdout: Vec<u8>, stderr: Vec<u8> }
    let out = Out { status, stdout: out_stdout, stderr: out_stderr };
    let _ = &out.stdout;
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        // surface the CLI's error (e.g. FIDO_ERR_*) trimmed to the last line
        let line = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("CLI failed");
        return Err(line.to_string());
    }
    Ok(stderr)
}

/// Reader path (pcsc://slotN) for the current session, releasing+reopening the
/// libfido2 session around `f` so the CLI is the sole PC/SC owner.
fn with_cli_reader<T>(state: &State<FidoState>, f: impl FnOnce(&str) -> R<T>) -> R<T> {
    let path = {
        let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
        let d = s.dev.take().ok_or("No FIDO2 device connected")?;
        d.path.clone()
    };
    std::thread::sleep(std::time::Duration::from_millis(150));
    let result = f(&path);
    std::thread::sleep(std::time::Duration::from_millis(150));
    ensure_init();
    if let Ok(c) = CString::new(path.clone()) {
        let dev = unsafe { fido_dev_new() };
        if !dev.is_null() {
            if unsafe { fido_dev_open(dev, c.as_ptr()) } == FIDO_OK {
                if let Ok(mut s) = state.0.lock() { s.dev = Some(FidoDev { dev, path }); }
            } else { let mut d = dev; unsafe { fido_dev_free(&mut d) }; }
        }
    }
    result
}

#[tauri::command]
pub async fn fido_note_get(state: State<'_, FidoState>) -> R<Option<String>> {
    use std::ffi::OsStr;
    let kf = note_keyfile()?;
    let dir = std::env::temp_dir().join("token2-key-manager");
    let out = dir.join("note.read.bin");
    with_cli_reader(&state, |path| {
        let args: Vec<&OsStr> = vec![
            OsStr::new("-Gb"), OsStr::new("-k"), kf.as_os_str(),
            out.as_os_str(), OsStr::new(path),
        ];
        // read needs the PIN too (largeBlob get with a key is PIN-gated on some keys);
        // but -Gb per the trace did not prompt — try without, ignore errors -> None
        match run_cli(&args, None) {
            Ok(_) => {
                let bytes = std::fs::read(&out).unwrap_or_default();
                let _ = std::fs::remove_file(&out);
                Ok(note_unframe(&bytes))
            }
            Err(_) => Ok(None),
        }
    })
}

#[tauri::command]
pub async fn fido_note_set(state: State<'_, FidoState>, text: String, pin: String) -> R<()> {
    use std::ffi::OsStr;
    let kf = note_keyfile()?;
    let dir = std::env::temp_dir().join("token2-key-manager");
    let blob = dir.join("note.write.bin");
    std::fs::write(&blob, note_frame(&text)).map_err(|e| e.to_string())?;
    let res = with_cli_reader(&state, |path| {
        let args: Vec<&OsStr> = vec![
            OsStr::new("-Sb"), OsStr::new("-k"), kf.as_os_str(),
            blob.as_os_str(), OsStr::new(path),
        ];
        run_cli(&args, Some(&pin)).map(|_| ())
    });
    let _ = std::fs::remove_file(&blob);
    res
}

#[tauri::command]
pub async fn fido_note_clear(state: State<'_, FidoState>, pin: String) -> R<()> {
    use std::ffi::OsStr;
    let kf = note_keyfile()?;
    with_cli_reader(&state, |path| {
        let args: Vec<&OsStr> = vec![
            OsStr::new("-Db"), OsStr::new("-k"), kf.as_os_str(), OsStr::new(path),
        ];
        run_cli(&args, Some(&pin)).map(|_| ())
    })
}

// ---- passkeys (credential management) ----
#[derive(Serialize, Clone)]
pub struct Passkey {
    pub cred_id: String, // hex
    pub user_name: String,
    pub display_name: String,
    pub user_id: String, // hex
    pub protection: i32,
    pub key_type: String,
}
#[derive(Serialize, Clone)]
pub struct RelyingParty {
    pub id: String,
    pub name: String,
    pub credentials: Vec<Passkey>,
    pub creds_readable: bool, // false if the key refused per-RP enumeration
}
#[derive(Serialize, Clone)]
pub struct PasskeyInventory {
    pub existing: u64,
    pub remaining: u64,
    pub rps: Vec<RelyingParty>,
}

#[tauri::command]
pub async fn fido_list_passkeys(state: State<'_, FidoState>, pin: String) -> R<PasskeyInventory> {
    let p = pin_or_uv(&pin)?;
    let used_pin = !pin.is_empty();
    with_dev(&state, |d| unsafe { list_passkeys_ptr(d, &p, used_pin) })
}
/// Core passkey inventory read against an open device pointer (shared by the GUI
/// command and the fido2-manage CLI).
unsafe fn list_passkeys_ptr(d: *mut fido_dev_t, p: &Option<CString>, used_pin: bool) -> R<PasskeyInventory> {
        let md = fido_credman_metadata_new();
        let r = fido_credman_get_dev_metadata(d, md, pin_ptr(p));
        // 0x2E = FIDO_ERR_NO_CREDENTIALS. With a PIN this genuinely means the key
        // has no passkeys -> empty list. With UV (empty pin) it usually means the
        // UV auth token for credential management wasn't obtained; don't silently
        // report empty — tell the caller to use the PIN for passkey management.
        // 0x2E = NO_CREDENTIALS. Some 2.1 firmware returns 0xF7 (INTERNAL) instead
        // when the store is simply empty. With a PIN, treat both as "no passkeys".
        if r == 0x2E || (used_pin && (r as u32) == 0xF7) {
            let mut m = md; fido_credman_metadata_free(&mut m);
            if used_pin {
                return Ok(PasskeyInventory { existing: 0, remaining: 0, rps: Vec::new() });
            } else {
                return Err("This key needs the PIN (not a fingerprint) to list passkeys. Unlock with the PIN.".into());
            }
        }
        let (existing, remaining) = (fido_credman_rk_existing(md), fido_credman_rk_remaining(md));
        let mut m = md;
        fido_credman_metadata_free(&mut m);
        rc(r)?;

        let rp = fido_credman_rp_new();
        let r = fido_credman_get_dev_rp(d, rp, pin_ptr(p));
        if r == 0x2E || (used_pin && (r as u32) == 0xF7) {
            let mut x = rp; fido_credman_rp_free(&mut x);
            return Ok(PasskeyInventory { existing, remaining, rps: Vec::new() });
        }
        if r != FIDO_OK {
            let mut x = rp;
            fido_credman_rp_free(&mut x);
            rc(r)?;
        }
        let mut rps = Vec::new();
        for i in 0..fido_credman_rp_count(rp) {
            let id = cstr(fido_credman_rp_id(rp, i));
            let name = cstr(fido_credman_rp_name(rp, i));
            // libfido2 hashes this string (SHA256) to match stored credentials,
            // so it must be the real RP id. If the id came back empty, fall back
            // to the name (some keys populate only one of the two).
            let lookup = if id.is_empty() { name.clone() } else { id.clone() };
            let rk = fido_credman_rk_new();
            let cid = CString::new(lookup.as_str()).unwrap_or_default();
            let r = fido_credman_get_dev_rk(d, cid.as_ptr(), rk, pin_ptr(p));
            let mut creds = Vec::new();
            if std::env::var("T2_FIDO_DEBUG").is_ok() {
                eprintln!("rp[{i}] id={id:?} name={name:?} lookup={lookup:?} get_dev_rk={r} rk_count={}", if r == FIDO_OK { fido_credman_rk_count(rk) as i64 } else { -1i64 });
            }
            if r == FIDO_OK {
                for j in 0..fido_credman_rk_count(rk) {
                    let c = fido_credman_rk(rk, j);
                    let idp = fido_cred_id_ptr(c);
                    let idl = fido_cred_id_len(c);
                    let uidp = fido_cred_user_id_ptr(c);
                    let uidl = fido_cred_user_id_len(c);
                    creds.push(Passkey {
                        cred_id: if idp.is_null() { String::new() } else { hex::encode(std::slice::from_raw_parts(idp, idl)) },
                        user_name: cstr(fido_cred_user_name(c)),
                        display_name: cstr(fido_cred_display_name(c)),
                        user_id: if uidp.is_null() { String::new() } else { hex::encode(std::slice::from_raw_parts(uidp, uidl)) },
                        protection: fido_cred_prot(c),
                        key_type: match fido_cred_type(c) { -7 => "ES256", -8 => "EdDSA", -257 => "RS256", -47 => "ES256K", t => return Err(format!("unknown key type {t}")) }.into(),
                    });
                }
            }
            let mut x = rk;
            fido_credman_rk_free(&mut x);
            rps.push(RelyingParty { id, name, credentials: creds, creds_readable: r == FIDO_OK });
        }
        let mut x = rp;
        fido_credman_rp_free(&mut x);
        Ok(PasskeyInventory { existing, remaining, rps })
}

/// Update user name / display name of a passkey (CTAP 2.1 credential management).
#[tauri::command]
pub async fn fido_update_passkey(state: State<'_, FidoState>, cred_id: String, user_id: String, user_name: String, display_name: String, pin: String) -> R<()> {
    let id = hex::decode(cred_id).map_err(|e| e.to_string())?;
    let uid = hex::decode(user_id).map_err(|e| e.to_string())?;
    let p = pin_or_uv(&pin)?;
    let n = CString::new(user_name).map_err(|e| e.to_string())?;
    let d = CString::new(display_name).map_err(|e| e.to_string())?;
    with_dev(&state, |dev| unsafe {
        let c = fido_cred_new();
        if c.is_null() { return Err("out of memory".into()); }
        let r = fido_cred_set_id(c, id.as_ptr(), id.len());
        let r = if r == FIDO_OK { fido_cred_set_user(c, uid.as_ptr(), uid.len(), n.as_ptr(), d.as_ptr(), ptr::null()) } else { r };
        let r = if r == FIDO_OK { fido_credman_set_dev_rk(dev, c, pin_ptr(&p)) } else { r };
        let mut x = c;
        fido_cred_free(&mut x);
        rc(r)
    })
}

#[tauri::command]
pub async fn fido_delete_passkey(state: State<'_, FidoState>, cred_id: String, pin: String) -> R<()> {
    let id = hex::decode(cred_id).map_err(|e| e.to_string())?;
    let p = pin_or_uv(&pin)?;
    with_dev(&state, |d| rc(unsafe { fido_credman_del_dev_rk(d, id.as_ptr(), id.len(), pin_ptr(&p)) }))
}

// ---- biometrics ----
#[derive(Serialize, Clone)]
pub struct BioTemplate {
    pub id: String,
    pub name: String,
}
#[derive(Serialize, Clone)]
pub struct BioInfo {
    pub sensor_type: u8,
    pub max_samples: u8,
    pub templates: Vec<BioTemplate>,
}

#[tauri::command]
pub async fn fido_list_bio(state: State<'_, FidoState>, pin: String) -> R<BioInfo> {
    let p = pin_or_uv(&pin)?;
    with_dev(&state, |d| unsafe { list_bio_ptr(d, &p) })
}
/// Core fingerprint-template read against an open device pointer.
unsafe fn list_bio_ptr(d: *mut fido_dev_t, p: &Option<CString>) -> R<BioInfo> {
        let bi = fido_bio_info_new();
        let (mut sensor_type, mut max_samples) = (0u8, 0u8);
        if fido_bio_dev_get_info(d, bi) == FIDO_OK {
            sensor_type = fido_bio_info_type(bi);
            max_samples = fido_bio_info_max_samples(bi);
        }
        let mut x = bi;
        fido_bio_info_free(&mut x);
        let ta = fido_bio_template_array_new();
        let r = fido_bio_dev_get_template_array(d, ta, pin_ptr(p));
        let mut templates = Vec::new();
        if r == FIDO_OK {
            for i in 0..fido_bio_template_array_count(ta) {
                let t = fido_bio_template(ta, i);
                let ip = fido_bio_template_id_ptr(t);
                let il = fido_bio_template_id_len(t);
                templates.push(BioTemplate {
                    id: if ip.is_null() { String::new() } else { hex::encode(std::slice::from_raw_parts(ip, il)) },
                    name: cstr(fido_bio_template_name(t)),
                });
            }
        }
        let mut x = ta;
        fido_bio_template_array_free(&mut x);
        rc(r)?;
        Ok(BioInfo { sensor_type, max_samples, templates })
}

fn template_with_id(id_hex: &str, name: Option<&str>) -> R<*mut fido_bio_template_t> {
    let id = hex::decode(id_hex).map_err(|e| e.to_string())?;
    let t = unsafe { fido_bio_template_new() };
    if t.is_null() {
        return Err("out of memory".into());
    }
    unsafe {
        rc(fido_bio_template_set_id(t, id.as_ptr(), id.len()))?;
        if let Some(n) = name {
            let c = CString::new(n).map_err(|e| e.to_string())?;
            rc(fido_bio_template_set_name(t, c.as_ptr()))?;
        }
    }
    Ok(t)
}

#[tauri::command]
pub async fn fido_rename_bio(state: State<'_, FidoState>, id: String, name: String, pin: String) -> R<()> {
    let p = pin_or_uv(&pin)?;
    let t = template_with_id(&id, Some(&name))?;
    let res = with_dev(&state, |d| rc(unsafe { fido_bio_dev_set_template_name(d, t, pin_ptr(&p)) }));
    let mut x = t;
    unsafe { fido_bio_template_free(&mut x) };
    res
}

#[tauri::command]
pub async fn fido_delete_bio(state: State<'_, FidoState>, id: String, pin: String) -> R<()> {
    let p = pin_or_uv(&pin)?;
    let t = template_with_id(&id, None)?;
    let res = with_dev(&state, |d| rc(unsafe { fido_bio_dev_enroll_remove(d, t, pin_ptr(&p)) }));
    let mut x = t;
    unsafe { fido_bio_template_free(&mut x) };
    res
}

#[derive(Serialize, Clone)]
pub struct EnrollStep {
    pub remaining: u8,
    pub status: u8,
    pub status_text: String,
    pub done: bool,
    pub template_id: String,
}

fn enroll_status_text(s: u8) -> &'static str {
    match s {
        0x00 => "Good sample",
        0x01 => "Too high — move your finger down",
        0x02 => "Too low — move your finger up",
        0x03 => "Too far left",
        0x04 => "Too far right",
        0x05 => "Too fast",
        0x06 => "Too slow",
        0x07 => "Poor quality — try again",
        0x08 => "Too skewed",
        0x09 => "Too short",
        0x0a => "Could not merge sample",
        0x0b => "This fingerprint is already enrolled",
        0x0c => "Fingerprint storage is full",
        0x0d => "No finger detected",
        0x0e => "Lift your finger and touch again",
        _ => "",
    }
}

fn enroll_step(s: &FidoSession) -> R<EnrollStep> {
    let e = s.enrollment.as_ref().ok_or("no enrollment in progress")?;
    unsafe {
        let remaining = fido_bio_enroll_remaining_samples(e.enroll);
        let status = fido_bio_enroll_last_status(e.enroll);
        let ip = fido_bio_template_id_ptr(e.template);
        let il = fido_bio_template_id_len(e.template);
        Ok(EnrollStep {
            remaining,
            status,
            status_text: enroll_status_text(status).into(),
            done: remaining == 0,
            template_id: if ip.is_null() { String::new() } else { hex::encode(std::slice::from_raw_parts(ip, il)) },
        })
    }
}

/// Start fingerprint enrollment; captures the first sample (blocks until touch or timeout).
#[tauri::command]
pub async fn fido_enroll_begin(state: State<'_, FidoState>, name: String, pin: String) -> R<EnrollStep> {
    let p = pin_or_uv(&pin)?;
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let d = s.dev.as_ref().ok_or("No FIDO2 device connected")?.dev;
    let t = unsafe { fido_bio_template_new() };
    let e = unsafe { fido_bio_enroll_new() };
    if t.is_null() || e.is_null() {
        return Err("out of memory".into());
    }
    // The name can only be applied AFTER enrollment completes (the template id
    // is assigned by the device on completion). Stash it and set it then.
    s.enrollment = Some(Enrollment { template: t, enroll: e, name: name.clone(), pin: pin.clone() });
    let r = unsafe { fido_bio_dev_enroll_begin(d, t, e, 30000, pin_ptr(&p)) };
    if r != FIDO_OK {
        s.enrollment = None;
        rc(r)?;
    }
    enroll_step(&s)
}

/// Capture the next sample (blocks until touch or timeout).
#[tauri::command]
pub async fn fido_enroll_continue(state: State<'_, FidoState>) -> R<EnrollStep> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let d = s.dev.as_ref().ok_or("No FIDO2 device connected")?.dev;
    let (t, e) = {
        let en = s.enrollment.as_ref().ok_or("no enrollment in progress")?;
        (en.template, en.enroll)
    };
    let r = unsafe { fido_bio_dev_enroll_continue(d, t, e, 30000) };
    if r != FIDO_OK {
        rc(r)?;
    }
    let step = enroll_step(&s)?;
    if step.done {
        // enrollment finished: the template now has a real id, so name it.
        if let Some(en) = s.enrollment.as_ref() {
            if !en.name.is_empty() {
                if let Ok(c) = CString::new(en.name.clone()) {
                    unsafe { fido_bio_template_set_name(en.template, c.as_ptr()) };
                    let pin = pin_or_uv(&en.pin).ok().flatten();
                    unsafe { fido_bio_dev_set_template_name(d, en.template, pin_ptr(&pin)) };
                }
            }
        }
        s.enrollment = None;
    }
    Ok(step)
}

#[tauri::command]
pub async fn fido_enroll_cancel(state: State<'_, FidoState>) -> R<()> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    if let Some(d) = s.dev.as_ref() {
        unsafe { fido_bio_dev_enroll_cancel(d.dev) };
    }
    s.enrollment = None;
    Ok(())
}

/// Round-trip for the "touch your key" hint on NFC vs USB.
#[tauri::command]
pub fn fido_libfido2_version() -> String {
    "libfido2 (upstream, PC/SC backend)".into()
}

#[allow(dead_code)]
fn _unused(_: *mut c_void) {}

// ---- diagnostics: why a reader is or isn't listed ---------------------------
#[derive(Serialize, Clone)]
pub struct ReaderProbe {
    pub index: usize,
    pub reader: String,
    pub path: String,
    pub open_ok: bool,
    pub error: String,
    pub is_fido2: bool,
    pub versions: Vec<String>,
}

/// Probe every PC/SC reader the way libfido2's manifest does, but report the
/// outcome instead of silently dropping failures.
#[tauri::command]
pub async fn fido_diagnose(state: State<'_, FidoState>) -> R<Vec<ReaderProbe>> {
    ensure_init();
    // release our own handle so the probe isn't blocked by it
    {
        let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
        s.enrollment = None;
        s.dev = None;
    }
    let readers = crate::piv::Piv::new().and_then(|p| p.list_readers()).unwrap_or_default();
    let n = readers.len().max(1);
    let mut out = Vec::new();
    for i in 0..n {
        let path = format!("pcsc://slot{i}");
        let dev = unsafe { fido_dev_new() };
        let c = CString::new(path.as_str()).unwrap();
        let r = unsafe { fido_dev_open(dev, c.as_ptr()) };
        let mut probe = ReaderProbe {
            index: i,
            reader: readers.get(i).cloned().unwrap_or_else(|| "(no PC/SC reader at this index)".into()),
            path: path.clone(),
            open_ok: r == FIDO_OK,
            error: if r == FIDO_OK { String::new() } else { rc(r).err().unwrap_or_default() },
            is_fido2: false,
            versions: vec![],
        };
        if r == FIDO_OK {
            probe.is_fido2 = unsafe { fido_dev_is_fido2(dev) };
            if let Ok(info) = read_info(dev, &path) {
                probe.versions = info.versions;
            }
            unsafe { fido_dev_close(dev) };
        }
        let mut d = dev;
        unsafe { fido_dev_free(&mut d) };
        out.push(probe);
    }
    Ok(out)
}

// ---------------------------------------------- applet presence -------------
#[derive(Serialize, Clone, Default)]
pub struct KeyCapabilities {
    pub piv: bool,
    pub oath: bool,       // standard OATH applet
    pub token2_otp: bool, // Token2 OTP applet
    pub fido: bool,       // a FIDO2 device is present (CCID or HID)
    pub openpgp: bool,
    pub revision: String, // e.g. "R3.4" from the serial prefix, "" if unknown
    pub model: String,    // model name from the serial prefix, "" if unknown
    pub serial: String,   // full device serial (from the OTP applet), "" if unknown
}

/// Transmit an APDU and follow 61xx (more-data) via GET RESPONSE, returning the
/// response payload without the trailing SW. Applets on Token2 keys answer with
/// 61xx on T=0 contact readers.
fn transmit_apdu(card: &pcsc::Card, apdu: &[u8]) -> Result<Vec<u8>, String> {
    let mut cmd = apdu.to_vec();
    let mut data = Vec::new();
    let mut buf = vec![0u8; 4096];
    loop {
        let r = card.transmit(&cmd, &mut buf).map_err(|e| e.to_string())?;
        if r.len() < 2 { return Err("short response".into()); }
        let sw = ((r[r.len()-2] as u16) << 8) | r[r.len()-1] as u16;
        data.extend_from_slice(&r[..r.len()-2]);
        if sw >> 8 == 0x61 {
            cmd = vec![0x00, 0xc0, 0x00, 0x00, (sw & 0xff) as u8];
            continue;
        }
        return Ok(data);
    }
}

/// SELECT each applet on the first reader that has one, to learn what this key
/// carries. Also reports whether any FIDO2 device is visible at all.
/// `busy_reader`: a reader with a live session that must NOT be probed
/// (probing issues ResetCard, which would break that session). For a reader
/// that is busy we infer applet presence from the session that owns it instead.
#[tauri::command]
pub fn key_capabilities(busy_reader: Option<String>, busy_has_piv: Option<bool>, busy_has_otp: Option<bool>) -> R<KeyCapabilities> {
    use pcsc::{Context, Protocols, Scope, ShareMode};
    let mut caps = KeyCapabilities::default();
    // FIDO presence without opening anything:
    //  * a Token2 reader implies CCID FIDO (tunnel) — works elevated or not;
    //  * when elevated, a raw HID FIDO device also counts (standard HID path).
    // We avoid the libfido2 PC/SC manifest here (it would SELECT the OTP applet
    // and disturb a live PIV/OTP session).
    {
        use pcsc::{Context as C2, Scope as S2};
        let mut token2_reader = false;
        if let Ok(ctx) = C2::establish(S2::User) {
            let mut b = vec![0u8; 4096];
            if let Ok(rs) = ctx.list_readers(&mut b) {
                token2_reader = rs.filter_map(|r| r.to_str().ok()).any(|r| r.to_uppercase().contains("TOKEN2"));
            }
            if !token2_reader && has_token2_vid() {
                token2_reader = true;
            }
        }
        // caps.fido is decided below from the actual FIDO-applet probe plus the
        // Token2 OTP-tunnel and HID signals — not from the reader name alone, so
        // an NFC reader with no FIDO card doesn't falsely advertise FIDO.
        let _ = token2_reader;
    }
    // applets over PC/SC
    let ctx = match Context::establish(Scope::User) { Ok(c) => c, Err(_) => return Ok(caps) };
    let mut buf = vec![0u8; 4096];
    let readers: Vec<_> = match ctx.list_readers(&mut buf) { Ok(r) => r.map(|c| c.to_owned()).collect(), Err(_) => return Ok(caps) };
    let aids: [&[u8]; 5] = [
        &[0xa0,0x00,0x00,0x03,0x08],                    // PIV
        &[0xa0,0x00,0x00,0x05,0x27,0x21,0x01],          // OATH
        &[0xf0,0x00,0x00,0x01,0x4f,0x74,0x70,0x01],     // Token2 OTP
        &[0xd2,0x76,0x00,0x01,0x24,0x01],               // OpenPGP
        &[0xa0,0x00,0x00,0x06,0x47,0x2f,0x00,0x01],     // FIDO (U2F/CTAP applet)
    ];
    // SELECT each applet on ONE connection without resetting between probes: a
    // SELECT cleanly replaces the current applet, so no reset is needed, and
    // this yields the same result whether the process is elevated or not.
    let sel = |card: &pcsc::Card, aid: &[u8]| -> bool {
        let mut a = vec![0x00u8, 0xa4, 0x04, 0x00, aid.len() as u8];
        a.extend_from_slice(aid);
        let mut rb = vec![0u8; 258];
        card.transmit(&a, &mut rb).ok().and_then(|r| {
            if r.len() >= 2 { let sw = ((r[r.len()-2] as u16) << 8) | r[r.len()-1] as u16; Some(sw == 0x9000 || sw >> 8 == 0x61) } else { None }
        }).unwrap_or(false)
    };
    for reader in &readers {
        if busy_reader.as_deref() == Some(reader.to_str().unwrap_or("")) {
            if busy_has_piv == Some(true) { caps.piv = true; }
            if busy_has_otp == Some(true) { caps.token2_otp = true; }
            continue;
        }
        let Ok(mut card) = ctx.connect(reader, ShareMode::Shared, Protocols::ANY) else { continue };
        let probe = |card: &pcsc::Card| [sel(card, aids[0]), sel(card, aids[1]), sel(card, aids[2]), sel(card, aids[3]), sel(card, aids[4])];
        let mut flags = probe(&card);
        // On Linux, the first SELECT right after the key enumerates can fail
        // transiently, so PIV/OpenPGP look absent even though they are there.
        // If OTP (Token2) or FIDO is present but PIV AND OpenPGP are both absent,
        // reset the card to a clean state and probe once more.
        if (flags[2] || flags[4]) && !flags[0] && !flags[3] {
            if (&mut card).reconnect(ShareMode::Shared, Protocols::ANY, pcsc::Disposition::ResetCard).is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(60));
                let f2 = probe(&card);
                for i in 0..5 { flags[i] = flags[i] || f2[i]; }
            }
        }
        // If the Token2 OTP applet is present, re-SELECT it cleanly (the probe loop
        // left the card on OpenPGP, the last present applet) and read the full
        // serial to derive model/revision. transmit_apdu follows 61xx chaining,
        // which these applets use on T=0 readers.
        if flags[2] {
            let mut sel = vec![0x00u8, 0xa4, 0x04, 0x00, aids[2].len() as u8];
            sel.extend_from_slice(aids[2]);
            let _ = transmit_apdu(&card, &sel);
            let mut req = vec![0x80u8, 0x33, 0x00, 0x00, 18, 0xd1, 0x10];
            req.extend_from_slice(&[0u8; 16]);
            if let Ok(d) = transmit_apdu(&card, &req) {
                if d.len() >= 3 && d[0] == 0xd1 {
                    let len = d[1] as usize;
                    if 2 + len <= d.len() {
                        let serial = String::from_utf8_lossy(&d[2..2 + len]).into_owned();
                        caps.serial = serial.clone();
                        if let Some(m) = crate::t2model::from_serial(&serial) {
                            caps.revision = m.revision.clone();
                            caps.model = m.model.clone();
                        }
                    }
                }
            }
        }
        // Fallback for FIDO2-only PIN+ keys (no OTP applet): read the serial via the
        // vendor command 80 E3 EE 80 00 over the FIDO applet. The response is a TLV
        // where tag 82 holds the serial as ASCII digits.
        if caps.serial.is_empty() && flags[4] {
            let mut selc = vec![0x00u8, 0xa4, 0x04, 0x00, aids[4].len() as u8];
            selc.extend_from_slice(aids[4]);
            let _ = transmit_apdu(&card, &selc);
            if let Ok(d) = transmit_apdu(&card, &[0x80u8, 0xe3, 0xee, 0x80, 0x00]) {
                // d = 80 <len> <TLVs>; walk to tag 82
                if d.len() >= 2 && d[0] == 0x80 {
                    let body = &d[2..];
                    let mut j = 0usize;
                    while j + 2 <= body.len() {
                        let t = body[j]; let l = body[j+1] as usize;
                        if j + 2 + l > body.len() { break; }
                        if t == 0x82 {
                            let v = &body[j+2..j+2+l];
                            caps.serial = String::from_utf8_lossy(v).trim_matches(char::from(0)).to_string();
                            break;
                        }
                        j += 2 + l;
                    }
                }
            }
        }
        // one reset at the end so we leave the card clean for the next opener
        let _ = (&mut card).reconnect(ShareMode::Shared, Protocols::ANY, pcsc::Disposition::ResetCard);
        if flags.iter().any(|&x| x) {
            caps.piv = flags[0]; caps.oath = flags[1]; caps.token2_otp = flags[2]; caps.openpgp = flags[3];
            if flags[4] { caps.fido = true; } // real FIDO applet on this reader (NFC, or non-Token2)
            // Token2 USB key: FIDO rides the OTP-applet tunnel, not the FIDO applet.
            // Recognise it when the OTP applet is present on a contact (non-NFC)
            // reader. NFC/contactless readers usually carry "CL"/"contactless"/"NFC"
            // /"PICC" in the name; those must show the real FIDO applet instead.
            let rname = reader.to_str().unwrap_or("").to_uppercase();
            let contactless = rname.contains("CONTACTLESS") || rname.contains("NFC") || rname.contains("PICC") || rname.contains(" CL");
            if flags[2] && !contactless { caps.fido = true; }
            break;
        }
    }
    // elevated: raw HID FIDO devices (e.g. YubiKey over USB) count too. Do this
    // AFTER the PC/SC applet probe so the manifest's OTP-applet SELECT can't
    // corrupt it.
    if is_elevated() {
        if let Ok(devs) = fido_list_devices() {
            let hid: Vec<_> = devs.into_iter().filter(|d| d.transport == "hid").collect();
            if !hid.is_empty() { caps.fido = true; }
            // read the serial from a HID key (e.g. Feitian ePass / PIN+ firmware)
            // via the vendor APDU over CTAPHID, if we don't already have one.
            if caps.serial.is_empty() {
                for d in &hid {
                    let sn = hid_serial(&d.path);
                    if !sn.is_empty() { caps.serial = sn; break; }
                }
            }
        }
    }
    Ok(caps)
}

// ------------------------------------------------- device presence snapshot --
#[derive(Serialize, Clone, Default)]
pub struct Presence {
    pub readers: Vec<String>,     // PC/SC reader names (PIV/OTP live here)
    pub fido_devices: usize,      // count of FIDO devices (HID or CCID)
    pub token2_reader: Option<String>, // first reader whose name contains TOKEN2
    pub key_name: Option<String>, // a display name for the inserted key (reader label)
}

/// Token2 / Excelsecu USB vendor id. Shared with some Feitian OEM keys, so the
/// product string is used as a secondary signal where it matters.
const TOKEN2_VID: u16 = 0x349e;
fn has_token2_vid() -> bool {
    fido_list_devices().map(|v| v.iter().any(|d| d.vendor_id == TOKEN2_VID)).unwrap_or(false)
}

/// One cheap call the UI polls: what readers and FIDO devices exist right now.
/// Does not open or reset anything, so it is safe to call while sessions are live.
#[tauri::command]
pub fn presence() -> Presence {
    use pcsc::{Context, Scope};
    let mut p = Presence::default();
    if let Ok(ctx) = Context::establish(Scope::User) {
        let mut buf = vec![0u8; 4096];
        if let Ok(rs) = ctx.list_readers(&mut buf) {
            p.readers = rs.map(|c| c.to_string_lossy().into_owned()).collect();
            let by_vid = has_token2_vid();
            p.token2_reader = p.readers.iter().find(|r| r.to_uppercase().contains("TOKEN2")).cloned()
                .or_else(|| if by_vid { p.readers.first().cloned() } else { None });
        }
    }
    // FIDO presence: a Token2 reader implies CCID FIDO capability (the FIDO
    // applet is tunnelled through the OTP applet). We do NOT run the libfido2
    // PC/SC manifest here — with the tunnel it selects the OTP applet on every
    // reader each poll and would reset a live PIV/OTP session. Raw HID FIDO
    // keys (non-Token2) are counted separately below.
    if p.token2_reader.is_some() {
        p.fido_devices = 1; // Token2 CCID FIDO present
    } else {
        // count HID FIDO devices so a non-Token2 key still shows as detected
        // (management may need admin, but presence should not).
        let hid: Vec<_> = fido_list_devices().map(|v| v.into_iter().filter(|d| d.transport == "hid").collect()).unwrap_or_default();
        p.fido_devices = hid.len();
        if p.key_name.is_none() {
            p.key_name = hid.into_iter().next().map(|d| if d.product.is_empty() { "FIDO2 security key".into() } else { d.product });
        }
    }
    if p.key_name.is_none() {
        p.key_name = p.token2_reader.clone().or_else(|| p.readers.first().cloned());
    }
    p
}

/// How many smart cards are currently *inserted/present* across all readers.
/// Unlike presence() (which counts readers), this checks the card state so a
/// card lifted off an NFC reader is seen as removed even though the reader
/// itself stays. Used by the guided FIDO reset to detect NFC tap off/on.
/// Report whether the PC/SC smart-card service is reachable. On Linux this is the
/// pcscd daemon; when it is down, FIDO and PIV (which use pcsc://) silently fail,
/// so the UI uses this to show an actionable message instead of nothing.
#[tauri::command]
pub fn pcsc_health() -> PcscHealth {
    use pcsc::{Context, Scope};
    match Context::establish(Scope::User) {
        Ok(_) => PcscHealth { available: true, message: String::new() },
        Err(e) => {
            let hint = if cfg!(target_os = "linux") {
                "The PC/SC smart-card service (pcscd) is not available, so FIDO2 and PIV cannot be accessed. Install and start it:\n  sudo apt install pcscd libpcsclite1 libccid\n  sudo systemctl enable --now pcscd\nOTP still works over USB-HID.".to_string()
            } else if cfg!(target_os = "macos") {
                "The PC/SC smart-card service is not responding. Reconnect the key or restart the app.".to_string()
            } else {
                "The Windows Smart Card service is not running. Start it: in Services, set \"Smart Card\" to Automatic and Start it.".to_string()
            };
            PcscHealth { available: false, message: format!("{hint}\n\n(details: {e})") }
        }
    }
}

#[derive(Serialize, Clone)]
pub struct PcscHealth { pub available: bool, pub message: String }

#[tauri::command]
pub fn cards_present() -> usize {
    use pcsc::{Context, Scope, ReaderState, State as PcscState};
    let ctx = match Context::establish(Scope::User) { Ok(c) => c, Err(_) => return 0 };
    let mut buf = vec![0u8; 4096];
    // own the reader names as CStrings so the ReaderStates can borrow them
    let readers: Vec<CString> = match ctx.list_readers(&mut buf) {
        Ok(rs) => rs.map(|c| c.to_owned()).collect(),
        Err(_) => return 0,
    };
    if readers.is_empty() { return 0; }
    let mut states: Vec<ReaderState> = readers.iter()
        .map(|r| ReaderState::new(r.as_c_str(), PcscState::UNAWARE))
        .collect();
    // zero timeout: just read the current state once
    if ctx.get_status_change(std::time::Duration::from_secs(0), &mut states).is_err() {
        return 0;
    }
    states.iter()
        .filter(|st| st.event_state().contains(PcscState::PRESENT))
        .count()
}

/// Connect FIDO over CCID by reader index, without running the PC/SC manifest
/// (which would select the OTP applet on every reader). libfido2 opens
/// `pcsc://slotN` directly. Returns the FIDO info on success.
#[tauri::command]
pub async fn fido_connect_ccid(state: State<'_, FidoState>, slot: usize) -> R<FidoInfo> {
    ensure_init();
    // Try the given slot first, then scan 0..8. libfido2's pcsc slot index is
    // its own enumeration order, not the PC/SC reader index, so probe a range.
    let mut order: Vec<usize> = vec![slot];
    order.extend((0..8).filter(|&i| i != slot));
    let mut last = String::from("no pcsc:// FIDO slot responded");
    for i in order {
        let path = format!("pcsc://slot{i}");
        let dev = unsafe { fido_dev_new() };
        if dev.is_null() { continue; }
        let c = match CString::new(path.as_str()) { Ok(c) => c, Err(_) => { let mut d = dev; unsafe { fido_dev_free(&mut d) }; continue; } };
        let r = unsafe { fido_dev_open(dev, c.as_ptr()) };
        if r == FIDO_OK {
            match read_info(dev, &path) {
                Ok(info) => {
                    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
                    s.enrollment = None;
                    s.dev = Some(FidoDev { dev, path });
                    return Ok(info);
                }
                Err(e) => { last = e; let mut d = dev; unsafe { fido_dev_free(&mut d) }; }
            }
        } else {
            last = rc(r).err().unwrap_or_else(|| "open failed".into());
            let mut d = dev; unsafe { fido_dev_free(&mut d) };
        }
    }
    Err(format!("FIDO over CCID failed: {last}. If a PIV or OTP session is open on the same key, it must be closed first (the card is single-channel)."))
}

// ---------------------------------------------------- elevation / transport ---
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION::default();
        let mut ret = 0u32;
        let ok = GetTokenInformation(token, TokenElevation, Some(&mut elev as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut ret).is_ok();
        windows::Win32::Foundation::CloseHandle(token).ok();
        ok && elev.TokenIsElevated != 0
    }
}
#[cfg(not(windows))]
pub fn is_elevated() -> bool { true } // Linux/macOS: HID FIDO works without special elevation

#[tauri::command]
pub fn fido_environment() -> FidoEnvironment {
    FidoEnvironment {
        elevated: is_elevated(),
        os: std::env::consts::OS.to_string(),
        privileged: privileged(),
    }
}

/// True only when the process actually has elevated privileges:
/// Windows = elevated token; Linux/macOS = running as root (uid 0).
#[cfg(windows)]
fn privileged() -> bool { is_elevated() }
#[cfg(not(windows))]
fn privileged() -> bool { unsafe { libc_geteuid() == 0 } }
#[cfg(not(windows))]
extern "C" { #[link_name = "geteuid"] fn libc_geteuid() -> u32; }

#[derive(serde::Serialize, Clone)]
pub struct FidoEnvironment { pub elevated: bool, pub os: String, pub privileged: bool }

/// Diagnostic: probe each applet and report the SELECT status word + model, so
/// capability-detection issues can be seen directly (T2_FIDO_DEBUG or the UI).
#[tauri::command]
pub fn capabilities_debug() -> String {
    use pcsc::{Context, Protocols, Scope, ShareMode};
    let mut out = String::new();
    let ctx = match Context::establish(Scope::User) { Ok(c) => c, Err(e) => return format!("no pcsc: {e}") };
    let mut buf = vec![0u8; 4096];
    let readers: Vec<_> = match ctx.list_readers(&mut buf) { Ok(r) => r.map(|c| c.to_owned()).collect(), Err(e) => return format!("list_readers: {e}") };
    let aids: [(&str, &[u8]); 5] = [
        ("PIV",     &[0xa0,0x00,0x00,0x03,0x08]),
        ("OATH",    &[0xa0,0x00,0x00,0x05,0x27,0x21,0x01]),
        ("T2OTP",   &[0xf0,0x00,0x00,0x01,0x4f,0x74,0x70,0x01]),
        ("OpenPGP", &[0xd2,0x76,0x00,0x01,0x24,0x01]),
        ("FIDO",    &[0xa0,0x00,0x00,0x06,0x47,0x2f,0x00,0x01]),
    ];
    for reader in &readers {
        out.push_str(&format!("reader: {}\n", reader.to_string_lossy()));
        let Ok(mut card) = ctx.connect(reader, ShareMode::Shared, Protocols::ANY) else { out.push_str("  connect failed\n"); continue };
        for (name, aid) in &aids {
            let mut a = vec![0x00u8, 0xa4, 0x04, 0x00, aid.len() as u8];
            a.extend_from_slice(aid);
            let mut rb = vec![0u8; 258];
            match card.transmit(&a, &mut rb) {
                Ok(r) if r.len() >= 2 => {
                    let sw = ((r[r.len()-2] as u16) << 8) | r[r.len()-1] as u16;
                    out.push_str(&format!("  {name:8} SELECT -> {sw:04X}{}\n", if sw == 0x9000 || sw >> 8 == 0x61 { " (present)" } else { "" }));
                }
                Ok(_) => out.push_str(&format!("  {name:8} short response\n")),
                Err(e) => out.push_str(&format!("  {name:8} transmit err: {e}\n")),
            }
        }
        // read + show the OTP serial and derived model, to debug revision detection
        {
            let mut sel = vec![0x00u8, 0xa4, 0x04, 0x00, 8, 0xf0,0x00,0x00,0x01,0x4f,0x74,0x70,0x01];
            let _ = transmit_apdu(&card, &sel);
            sel.clear();
            let mut req = vec![0x80u8, 0x33, 0x00, 0x00, 18, 0xd1, 0x10];
            req.extend_from_slice(&[0u8; 16]);
            match transmit_apdu(&card, &req) {
                Ok(d) => {
                    out.push_str(&format!("  OTP serial raw: {}\n", d.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")));
                    if d.len() >= 3 && d[0] == 0xd1 {
                        let len = d[1] as usize;
                        if 2 + len <= d.len() {
                            let serial = String::from_utf8_lossy(&d[2..2+len]).into_owned();
                            out.push_str(&format!("  serial: {serial}\n"));
                            match crate::t2model::from_serial(&serial) {
                                Some(m) => out.push_str(&format!("  model: {} rev {}\n", m.model, m.revision)),
                                None => out.push_str("  model: UNKNOWN (serial prefix not in table)\n"),
                            }
                        }
                    }
                }
                Err(e) => out.push_str(&format!("  OTP serial read err: {e}\n")),
            }
        }
        // FIDO-only serial probe: SELECT FIDO applet, send 80 E3 EE 80 00, dump raw.
        {
            let selc: &[u8] = &[0x00,0xa4,0x04,0x00,0x08,0xa0,0x00,0x00,0x06,0x47,0x2f,0x00,0x01];
            match transmit_apdu(&card, selc) {
                Ok(_) => {
                    match transmit_apdu(&card, &[0x80u8,0xe3,0xee,0x80,0x00]) {
                        Ok(d) => {
                            out.push_str(&format!("  FIDO serial cmd raw: {}\n", d.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")));
                            // walk TLV for tag 82
                            if d.len() >= 2 && d[0] == 0x80 {
                                let body = &d[2..];
                                let mut j = 0usize; let mut found = false;
                                while j + 2 <= body.len() {
                                    let t = body[j]; let l = body[j+1] as usize;
                                    if j + 2 + l > body.len() { break; }
                                    if t == 0x82 { out.push_str(&format!("  FIDO serial (tag 82): {}\n", String::from_utf8_lossy(&body[j+2..j+2+l]))); found = true; break; }
                                    j += 2 + l;
                                }
                                if !found { out.push_str("  FIDO serial: tag 82 not found in TLV\n"); }
                            }
                        }
                        Err(e) => out.push_str(&format!("  FIDO serial cmd err: {e}\n")),
                    }
                }
                Err(e) => out.push_str(&format!("  FIDO applet SELECT err: {e}\n")),
            }
        }
        let _ = card.reconnect(ShareMode::Shared, Protocols::ANY, pcsc::Disposition::ResetCard);
    }
    out.push_str(&format!("elevated: {}\n", is_elevated()));
    out
}

/// PC/SC service health: on Linux the pcscd daemon (and libpcsclite) must be
/// running or no smart-card key is ever seen. Returns Ok(true) if the service is
/// reachable, Ok(false) if it is installed but not running, and an explanatory
/// string the UI can show. On Windows/macOS the service is built in and normally
/// always available.
#[tauri::command]
pub fn pcsc_status() -> PcscStatus {
    use pcsc::{Context, Scope};
    match Context::establish(Scope::User) {
        Ok(ctx) => {
            // establish succeeded — the service is up. list_readers just confirms.
            let mut buf = vec![0u8; 4096];
            let readers = ctx.list_readers(&mut buf).map(|r| r.count()).unwrap_or(0);
            PcscStatus { ok: true, readers, message: String::new() }
        }
        Err(e) => {
            let msg = if cfg!(target_os = "linux") {
                format!("The PC/SC service isn't available ({e}). Install and start it:\n  sudo apt install pcscd libpcsclite1\n  sudo systemctl enable --now pcscd\nThen reinsert your key.")
            } else if cfg!(target_os = "macos") {
                format!("The PC/SC service isn't responding ({e}). Try reinserting the key or rebooting.")
            } else {
                format!("The Smart Card service isn't running ({e}). Open services.msc and start 'Smart Card' (SCardSvr), then reinsert your key.")
            };
            PcscStatus { ok: false, readers: 0, message: msg }
        }
    }
}

#[derive(Serialize, Clone)]
pub struct PcscStatus { pub ok: bool, pub readers: usize, pub message: String }

// ============================================================================
// Session-based cores for the fido2-manage CLI. These reuse the exact same FFI
// and module helpers as the Tauri commands above, but operate on a plain
// &FidoSession so the CLI binary can call them without Tauri's State.
// ============================================================================
impl FidoSession {
    /// Open a device by libfido2 path (e.g. "pcsc://slot0" or a hidraw path).
    pub fn open(path: &str) -> R<(Self, FidoInfo)> {
        ensure_init();
        let dev = unsafe { fido_dev_new() };
        if dev.is_null() { return Err("out of memory".into()); }
        let c = CString::new(path).map_err(|e| e.to_string())?;
        let r = unsafe { fido_dev_open(dev, c.as_ptr()) };
        if r != FIDO_OK {
            let mut d = dev; unsafe { fido_dev_free(&mut d) };
            return rc(r).map(|_| unreachable!());
        }
        let info = read_info(dev, path)?;
        let mut s = FidoSession::default();
        s.dev = Some(FidoDev { dev, path: path.to_string() });
        Ok((s, info))
    }
    fn d(&self) -> R<*mut fido_dev_t> { Ok(self.dev.as_ref().ok_or("no device")?.dev) }

    pub fn info(&self) -> R<FidoInfo> {
        read_info(self.d()?, &self.dev.as_ref().unwrap().path)
    }
    pub fn set_pin(&self, new_pin: &str, old_pin: Option<&str>) -> R<()> {
        let n = CString::new(new_pin).map_err(|e| e.to_string())?;
        let o = old_pin.filter(|p| !p.is_empty()).and_then(|p| CString::new(p).ok());
        rc(unsafe { fido_dev_set_pin(self.d()?, n.as_ptr(), pin_ptr(&o)) })
    }
    pub fn set_min_pin(&self, len: usize, pin: &str) -> R<()> {
        let p = opt_pin(&Some(pin.to_string()));
        rc(unsafe { fido_dev_set_pin_minlen(self.d()?, len, pin_ptr(&p)) })
    }
    pub fn force_pin_change(&self, pin: &str) -> R<()> {
        let p = opt_pin(&Some(pin.to_string()));
        rc(unsafe { fido_dev_force_pin_change(self.d()?, pin_ptr(&p)) })
    }
    /// -uvs / -uvd. The device call is a toggle; we only toggle when the current
    /// state differs from the desired one, mirroring fido2-manage semantics.
    pub fn set_always_uv(&self, enable: bool, pin: &str) -> R<()> {
        let d = self.d()?;
        let info = self.info()?;
        if info.always_uv == enable { return Ok(()); } // already in the wanted state
        if !unsafe { fido_dev_has_pin(d) } && !unsafe { fido_dev_has_uv(d) } {
            return Err("set a FIDO PIN (or enroll a fingerprint) before changing always-UV".into());
        }
        let p = pin_or_uv(pin)?;
        rc(unsafe { fido_dev_toggle_always_uv(d, pin_ptr(&p)) })
    }
    pub fn reset(&self) -> R<()> { rc(unsafe { fido_dev_reset(self.d()?) }) }

    pub fn list_passkeys(&self, pin: &str) -> R<PasskeyInventory> {
        let p = pin_or_uv(pin)?;
        let used_pin = !pin.is_empty();
        let d = self.d()?;
        unsafe { list_passkeys_ptr(d, &p, used_pin) }
    }
    pub fn delete_passkey(&self, cred_id_b64: &str, pin: &str) -> R<()> {
        let id = b64_decode(cred_id_b64)?;
        let p = opt_pin(&Some(pin.to_string()));
        rc(unsafe { fido_credman_del_dev_rk(self.d()?, id.as_ptr(), id.len(), pin_ptr(&p)) })
    }
    pub fn list_bio(&self, pin: &str) -> R<BioInfo> {
        let p = pin_or_uv(pin)?;
        let d = self.d()?;
        unsafe { list_bio_ptr(d, &p) }
    }
    pub fn rename_bio(&self, id_hex: &str, name: &str, pin: &str) -> R<()> {
        let p = pin_or_uv(pin)?;
        let t = template_with_id(id_hex, Some(name))?;
        let res = rc(unsafe { fido_bio_dev_set_template_name(self.d()?, t, pin_ptr(&p)) });
        let mut x = t; unsafe { fido_bio_template_free(&mut x) }; res
    }
    pub fn delete_bio(&self, id_hex: &str, pin: &str) -> R<()> {
        let p = pin_or_uv(pin)?;
        let t = template_with_id(id_hex, None)?;
        let res = rc(unsafe { fido_bio_dev_enroll_remove(self.d()?, t, pin_ptr(&p)) });
        let mut x = t; unsafe { fido_bio_template_free(&mut x) }; res
    }
    /// Blocking fingerprint enrollment; `progress(remaining, status)` after each sample.
    pub fn enroll_fingerprint(&self, name: &str, pin: &str, mut progress: impl FnMut(u8, u8)) -> R<()> {
        let d = self.d()?;
        let p = pin_or_uv(pin)?;
        let t = unsafe { fido_bio_template_new() };
        let e = unsafe { fido_bio_enroll_new() };
        if t.is_null() || e.is_null() { return Err("out of memory".into()); }
        let free = |mut t: *mut fido_bio_template_t, mut e: *mut fido_bio_enroll_t| unsafe {
            fido_bio_template_free(&mut t); fido_bio_enroll_free(&mut e);
        };
        if let Err(err) = rc(unsafe { fido_bio_dev_enroll_begin(d, t, e, 30000, pin_ptr(&p)) }) {
            unsafe { fido_bio_dev_enroll_cancel(d) }; free(t, e); return Err(err);
        }
        loop {
            let remaining = unsafe { fido_bio_enroll_remaining_samples(e) } as u8;
            let status = unsafe { fido_bio_enroll_last_status(e) };
            progress(remaining, status);
            if remaining == 0 { break; }
            if let Err(err) = rc(unsafe { fido_bio_dev_enroll_continue(d, t, e, 30000) }) {
                unsafe { fido_bio_dev_enroll_cancel(d) }; free(t, e); return Err(err);
            }
        }
        if !name.is_empty() {
            if let Ok(c) = CString::new(name) {
                unsafe { fido_bio_template_set_name(t, c.as_ptr()); }
                unsafe { fido_bio_dev_set_template_name(d, t, pin_ptr(&p)); }
            }
        }
        free(t, e); Ok(())
    }
}

/// base64 (standard, with '=' padding) decode for credential ids, matching
/// fido2-manage's -credential argument format. Small self-contained decoder so
/// the CLI doesn't pull an extra dependency.
fn b64_decode(s: &str) -> R<Vec<u8>> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut inv = [255u8; 256];
    for (i, &c) in T.iter().enumerate() { inv[c as usize] = i as u8; }
    let mut out = Vec::new();
    let mut buf = 0u32; let mut bits = 0;
    for &c in s.trim().as_bytes() {
        if c == b'=' { break; }
        let v = inv[c as usize];
        if v == 255 { return Err("invalid base64 in credential id".into()); }
        buf = (buf << 6) | v as u32; bits += 6;
        if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); }
    }
    Ok(out)
}
