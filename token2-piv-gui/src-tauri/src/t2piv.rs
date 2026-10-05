//! Raw FFI bindings to libt2piv (the PIV library shipped with token2-piv-tool).
//! Only the subset the GUI needs is declared here.
#![allow(non_camel_case_types, dead_code)]

use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
pub struct t2piv_state {
    _private: [u8; 0],
}

pub type t2piv_rc = c_int;
pub const T2PIV_OK: t2piv_rc = 0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct t2piv_metadata {
    pub algorithm: u8,
    pub pin_policy: u8,
    pub touch_policy: u8,
    pub origin: u8,
    pub pubkey_len: usize,
    pub pubkey: [u8; 1024],
}

impl Default for t2piv_metadata {
    fn default() -> Self {
        Self { algorithm: 0, pin_policy: 0, touch_policy: 0, origin: 0, pubkey_len: 0, pubkey: [0; 1024] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct t2piv_config {
    pub puk_blocked: u8,
    pub puk_noblock_on_upgrade: u8,
    pub pin_last_changed: u32,
    pub mgm_type: c_int, // -1 invalid, 0 manual, 1 derived, 2 protected
    pub mgm_len: usize,
    pub mgm_key: [u8; 32],
}

// algorithms
pub const ALGO_3DES: u8 = 0x03;
pub const ALGO_AES128: u8 = 0x08;
pub const ALGO_AES192: u8 = 0x0a;
pub const ALGO_AES256: u8 = 0x0c;
pub const ALGO_RSA1024: u8 = 0x06;
pub const ALGO_RSA2048: u8 = 0x07;
pub const ALGO_RSA3072: u8 = 0x05;
pub const ALGO_RSA4096: u8 = 0x16;
pub const ALGO_ECCP256: u8 = 0x11;
pub const ALGO_ECCP384: u8 = 0x14;
pub const ALGO_ED25519: u8 = 0xE0;
pub const ALGO_X25519: u8 = 0xE1;

// slots
pub const SLOT_AUTH: u8 = 0x9a;
pub const SLOT_SIGN: u8 = 0x9c;
pub const SLOT_KEYMGM: u8 = 0x9d;
pub const SLOT_CARDAUTH: u8 = 0x9e;
pub const SLOT_ATTESTATION: u8 = 0xf9;

// policies
pub const PINPOLICY_DEFAULT: u8 = 0;
pub const PINPOLICY_NEVER: u8 = 1;
pub const PINPOLICY_ONCE: u8 = 2;
pub const PINPOLICY_ALWAYS: u8 = 3;
pub const TOUCHPOLICY_DEFAULT: u8 = 0;
pub const TOUCHPOLICY_NEVER: u8 = 1;
pub const TOUCHPOLICY_ALWAYS: u8 = 2;
pub const TOUCHPOLICY_CACHED: u8 = 3;

pub const ORIGIN_GENERATED: u8 = 1;
pub const ORIGIN_IMPORTED: u8 = 2;

extern "C" {
    pub fn t2piv_strerror(err: t2piv_rc) -> *const c_char;
    pub fn t2piv_init(state: *mut *mut t2piv_state, verbose: c_int) -> t2piv_rc;
    pub fn t2piv_done(state: *mut t2piv_state) -> t2piv_rc;
    pub fn t2piv_connect(state: *mut t2piv_state, wanted: *const c_char) -> t2piv_rc;
    pub fn t2piv_disconnect(state: *mut t2piv_state) -> t2piv_rc;
    pub fn t2piv_list_readers(state: *mut t2piv_state, readers: *mut c_char, len: *mut usize) -> t2piv_rc;
    pub fn t2piv_get_version(state: *mut t2piv_state, version: *mut c_char, len: usize) -> t2piv_rc;
    pub fn t2piv_get_serial(state: *mut t2piv_state, serial: *mut u32) -> t2piv_rc;
    pub fn t2piv_transfer_data(
        state: *mut t2piv_state,
        templ: *const u8,
        in_data: *const u8,
        in_len: std::os::raw::c_long,
        out_data: *mut u8,
        out_len: *mut std::os::raw::c_ulong,
        sw: *mut c_int,
    ) -> t2piv_rc;
    pub fn t2piv_util_import_key_blob(
        state: *mut t2piv_state,
        slot: u8,
        data: *const u8,
        data_len: usize,
        password: *const c_char,
        pin_policy: u8,
        touch_policy: u8,
        write_cert: c_int,
        algorithm_out: *mut u8,
        cert_written: *mut c_int,
    ) -> t2piv_rc;
    pub fn t2piv_util_probe_key_blob(data: *const u8, data_len: usize, password: *const c_char, algorithm_out: *mut u8, has_cert: *mut c_int) -> t2piv_rc;
    pub fn t2piv_util_import_last_error() -> *const c_char;
    pub fn t2piv_get_serial_str(state: *mut t2piv_state, buf: *mut c_char, len: usize) -> t2piv_rc;
    pub fn t2piv_verify(state: *mut t2piv_state, pin: *const c_char, tries: *mut c_int) -> t2piv_rc;
    pub fn t2piv_get_pin_retries(state: *mut t2piv_state, tries: *mut c_int) -> t2piv_rc;
    pub fn t2piv_set_pin_retries(state: *mut t2piv_state, pin_tries: c_int, puk_tries: c_int) -> t2piv_rc;
    pub fn t2piv_change_pin(
        state: *mut t2piv_state,
        current: *const c_char,
        current_len: usize,
        new: *const c_char,
        new_len: usize,
        tries: *mut c_int,
    ) -> t2piv_rc;
    pub fn t2piv_change_puk(
        state: *mut t2piv_state,
        current: *const c_char,
        current_len: usize,
        new: *const c_char,
        new_len: usize,
        tries: *mut c_int,
    ) -> t2piv_rc;
    pub fn t2piv_unblock_pin(
        state: *mut t2piv_state,
        puk: *const c_char,
        puk_len: usize,
        new_pin: *const c_char,
        new_pin_len: usize,
        tries: *mut c_int,
    ) -> t2piv_rc;
    pub fn t2piv_authenticate2(state: *mut t2piv_state, key: *const u8, len: usize) -> t2piv_rc;
    pub fn t2piv_set_mgmkey3(state: *mut t2piv_state, new_key: *const u8, len: usize, algorithm: u8, touch: u8) -> t2piv_rc;
    pub fn t2piv_import_private_key(
        state: *mut t2piv_state,
        key: u8,
        algorithm: u8,
        p: *const u8, p_len: usize,
        q: *const u8, q_len: usize,
        dp: *const u8, dp_len: usize,
        dq: *const u8, dq_len: usize,
        qinv: *const u8, qinv_len: usize,
        ec_data: *const u8, ec_data_len: u8,
        pin_policy: u8,
        touch_policy: u8,
    ) -> t2piv_rc;
    pub fn t2piv_attest(state: *mut t2piv_state, key: u8, data: *mut u8, data_len: *mut usize) -> t2piv_rc;
    pub fn t2piv_get_metadata(state: *mut t2piv_state, key: u8, data: *mut u8, data_len: *mut usize) -> t2piv_rc;
    pub fn t2piv_util_parse_metadata(data: *mut u8, data_len: usize, metadata: *mut t2piv_metadata) -> t2piv_rc;
    pub fn t2piv_util_read_cert(state: *mut t2piv_state, slot: u8, data: *mut *mut u8, data_len: *mut usize) -> t2piv_rc;
    pub fn t2piv_util_write_cert(state: *mut t2piv_state, slot: u8, data: *mut u8, data_len: usize, certinfo: u8) -> t2piv_rc;
    pub fn t2piv_util_delete_cert(state: *mut t2piv_state, slot: u8) -> t2piv_rc;
    pub fn t2piv_move_key(state: *mut t2piv_state, from_slot: u8, to_slot: u8) -> t2piv_rc;
    pub fn t2piv_util_generate_key(
        state: *mut t2piv_state,
        slot: u8,
        algorithm: u8,
        pin_policy: u8,
        touch_policy: u8,
        modulus: *mut *mut u8,
        modulus_len: *mut usize,
        exp: *mut *mut u8,
        exp_len: *mut usize,
        point: *mut *mut u8,
        point_len: *mut usize,
    ) -> t2piv_rc;
    pub fn t2piv_util_free(state: *mut t2piv_state, data: *mut c_void) -> t2piv_rc;
    pub fn t2piv_util_reset(state: *mut t2piv_state) -> t2piv_rc;
    pub fn t2piv_util_get_config(state: *mut t2piv_state, config: *mut t2piv_config) -> t2piv_rc;
    pub fn t2piv_sign_data(
        state: *mut t2piv_state,
        sign_in: *const u8,
        in_len: usize,
        sign_out: *mut u8,
        out_len: *mut usize,
        algorithm: u8,
        key: u8,
    ) -> t2piv_rc;
}
