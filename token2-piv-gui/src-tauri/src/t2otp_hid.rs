//! OTP applet over a HID-only Token2 key (PIN+ Release2), via the CTAPHID tunnel.
//!
//! HID-only keys expose no PC/SC reader, so the normal PC/SC OTP path can't reach
//! the OTP applet. But the applet IS reachable by tunnelling OTP APDUs inside
//! CTAPHID, using vendor command 0x41 carrying a `80 C5 02`-framed payload
//! (the OTP sibling of the FIDO `80 C5 03` CBOR tunnel). Verified from a USBPcap
//! of the Companion app:
//!   INIT (0x86) on ffffffff + 8-byte nonce -> reply assigns a channel
//!   then: <channel> 0x41 <bcnt> 80 C5 02 00 <Lc> <OTP-APDU>
//!   reply: <channel> 0x41 <bcnt> 00 <OTP-response> <SW>
//!
//! This module is self-contained (hidapi only) — it does NOT touch libfido2.

use hidapi::{HidApi, HidDevice};

const REPORT_LEN: usize = 64;
const CMD_INIT: u8 = 0x86;
const CMD_OTP_TUNNEL: u8 = 0x41; // vendor command carrying the 80 C5 02 OTP tunnel
const BROADCAST: u32 = 0xffff_ffff;

pub struct OtpHid {
    dev: HidDevice,
    channel: u32,
}

impl OtpHid {
    /// Open the HID device at `path` and perform the CTAPHID INIT handshake.
    pub fn open(path: &str) -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| format!("hidapi: {e}"))?;
        let cpath = std::ffi::CString::new(path).map_err(|_| "bad device path")?;
        let dev = api.open_path(&cpath).map_err(|e| format!("open HID device: {e}"))?;
        let channel = ctaphid_init(&dev)?;
        Ok(Self { dev, channel })
    }

    /// Tunnel one OTP APDU and return (response_data_without_sw, sw).
    pub fn transmit(&self, apdu: &[u8]) -> Result<(Vec<u8>, u16), String> {
        // OTP APDUs (native 80 C5 CLA) go over CTAPHID cmd 0x41 directly (no wrapper;
        // the applet is pre-selected on this tunnel). For a case-2 read APDU
        // (5 bytes: hdr + Le), the HID transport expects Le trailing zero bytes
        // (from the capture: 80 C5 02 00 0A followed by 10 zeros).
        let mut frame = apdu.to_vec();
        if frame.len() == 5 {
            let le = frame[4] as usize;
            frame.extend(std::iter::repeat(0u8).take(le));
        }
        let resp = self.send_recv(CMD_OTP_TUNNEL, &frame)?;
        if resp.len() < 2 { return Err("short OTP response".into()); }
        // The 80 C5 command family returns <status 00><data><SW>; the 80 33 serial
        // returns <data><SW> with no status byte. Strip a single leading 0x00 only
        // when present (80 33 responses start with 0xD1, never 0x00).
        let body: &[u8] = if resp[0] == 0x00 { &resp[1..] } else { &resp[..] };
        if body.len() < 2 { return Err("short OTP response".into()); }
        let sw = ((body[body.len()-2] as u16) << 8) | body[body.len()-1] as u16;
        Ok((body[..body.len()-2].to_vec(), sw))
    }

    fn send_recv(&self, cmd: u8, data: &[u8]) -> Result<Vec<u8>, String> {
        // drain any stale/buffered input so we don't read a prior command's reply
        let mut junk = [0u8; REPORT_LEN];
        while self.dev.read_timeout(&mut junk, 0).unwrap_or(0) > 0 {}
        write_frames(&self.dev, self.channel, cmd, data)?;
        read_frames(&self.dev, self.channel, cmd)
    }
}

/// CTAPHID_INIT: get a fresh channel id from the broadcast channel.
fn ctaphid_init(dev: &HidDevice) -> Result<u32, String> {
    let nonce: [u8; 8] = rand8();
    write_frames(dev, BROADCAST, CMD_INIT, &nonce)?;
    let reply = read_frames(dev, BROADCAST, CMD_INIT)?;
    // reply: <8-byte nonce echo><4-byte new channel><...>
    if reply.len() < 12 { return Err("short CTAPHID INIT reply".into()); }
    if reply[..8] != nonce { return Err("CTAPHID INIT nonce mismatch".into()); }
    Ok(u32::from_be_bytes([reply[8], reply[9], reply[10], reply[11]]))
}

/// Write a CTAPHID message as an init frame plus continuation frames.
fn write_frames(dev: &HidDevice, channel: u32, cmd: u8, data: &[u8]) -> Result<(), String> {
    let ch = channel.to_be_bytes();
    let mut frame = [0u8; REPORT_LEN + 1]; // +1 for the report id (0x00) hidapi expects
    // ---- init frame: [ch0..3][0x80|cmd][bcnt_hi][bcnt_lo][data up to 57] ----
    frame[0] = 0x00; // report id
    frame[1..5].copy_from_slice(&ch);
    frame[5] = 0x80 | cmd;
    frame[6] = (data.len() >> 8) as u8;
    frame[7] = (data.len() & 0xff) as u8;
    let first = data.len().min(REPORT_LEN - 7);
    frame[8..8 + first].copy_from_slice(&data[..first]);
    dev.write(&frame).map_err(|e| format!("HID write: {e}"))?;
    // ---- continuation frames: [ch0..3][seq][data up to 59] ----
    let mut off = first;
    let mut seq = 0u8;
    while off < data.len() {
        let mut cont = [0u8; REPORT_LEN + 1];
        cont[0] = 0x00;
        cont[1..5].copy_from_slice(&ch);
        cont[5] = seq & 0x7f;
        let n = (data.len() - off).min(REPORT_LEN - 5);
        cont[6..6 + n].copy_from_slice(&data[off..off + n]);
        dev.write(&cont).map_err(|e| format!("HID write cont: {e}"))?;
        off += n;
        seq = seq.wrapping_add(1);
    }
    Ok(())
}

/// Read a CTAPHID message (init frame + continuations) for the given channel/cmd.
fn read_frames(dev: &HidDevice, channel: u32, cmd: u8) -> Result<Vec<u8>, String> {
    let ch = channel.to_be_bytes();
    let mut buf = [0u8; REPORT_LEN];
    // read init frame (skip keepalive 0xBB and frames for other channels)
    let mut tries = 0;
    let (bcnt, mut out) = loop {
        tries += 1;
        if tries > 30 { return Err("HID read timed out".into()); }
        let n = dev.read_timeout(&mut buf, 1000).map_err(|e| format!("HID read: {e}"))?;
        if n < 7 { continue; }
        if buf[0..4] != ch { continue; }
        let fcmd = buf[4];
        if fcmd == (0x80 | 0xbb) { continue; } // CTAPHID_KEEPALIVE
        if fcmd != (0x80 | cmd) {
            // error frame (0xBF) or mismatched cmd
            if fcmd == (0x80 | 0xbf) {
                let code = buf.get(7).copied().unwrap_or(0);
                return Err(format!("CTAPHID error 0x{code:02x}"));
            }
            continue;
        }
        let bcnt = ((buf[5] as usize) << 8) | buf[6] as usize;
        let first = bcnt.min(REPORT_LEN - 7);
        break (bcnt, buf[7..7 + first].to_vec());
    };
    // continuation frames
    let mut ctries = 0;
    while out.len() < bcnt {
        ctries += 1;
        if ctries > 60 { return Err("HID read (continuation) timed out".into()); }
        let n = dev.read_timeout(&mut buf, 1000).map_err(|e| format!("HID read cont: {e}"))?;
        if n < 5 { continue; }
        if buf[0..4] != ch { continue; }
        let take = (bcnt - out.len()).min(REPORT_LEN - 5);
        out.extend_from_slice(&buf[5..5 + take]);
    }
    Ok(out)
}

fn rand8() -> [u8; 8] {
    // a nonce; does not need to be cryptographically strong
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut n = [0u8; 8];
    n.copy_from_slice(&(t as u64).to_be_bytes());
    n[0] ^= 0x11; n[7] ^= 0x88; // avoid all-zero
    n
}

/// Enumerate HID FIDO devices (VID 0x349e) that may support the OTP tunnel.
/// Returns (path, product) pairs. Used for autodetection.
pub fn list_otp_hid_devices() -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Ok(api) = HidApi::new() {
        for d in api.device_list() {
            if d.vendor_id() != 0x349e { continue; }
            // The FIDO applet lives on the FIDO/CTAP HID interface, whose usage page
            // is 0xF1D0. Skip the keyboard interface (usage page 0x01 / paths with
            // "KBD"), which returns "access denied" and isn't the CTAPHID endpoint.
            if d.usage_page() != 0xF1D0 { continue; }
            if let Some(p) = d.path().to_str().ok() {
                let prod = d.product_string().unwrap_or("Token2 key").to_string();
                out.push((p.to_string(), prod));
            }
        }
    }
    out
}

/// Probe whether a HID device answers the OTP tunnel (SELECT the OTP applet).
/// Returns the serial if reachable, else None. Used to auto-detect OTP-HID keys.
pub fn probe_otp_hid(path: &str) -> Option<String> {
    let hid = OtpHid::open(path).ok()?;
    // No SELECT over HID (the applet is pre-selected; SELECT returns 6D00). The
    // serial read alone confirms the OTP applet responds over the tunnel.
    // read serial: 80 33 00 00 12 D1 10 <16 zeros>
    let mut ser = vec![0x80u8,0x33,0x00,0x00,0x12,0xd1,0x10];
    ser.extend_from_slice(&[0u8;16]);
    let (d, sw) = hid.transmit(&ser).ok()?;
    if sw != 0x9000 { return None; }
    // response: D1 0E <14 ascii digits>
    if d.len() >= 3 && d[0] == 0xd1 {
        let l = d[1] as usize;
        if d.len() >= 2 + l {
            return Some(String::from_utf8_lossy(&d[2..2+l]).trim_matches(char::from(0)).to_string());
        }
    }
    Some(String::new())
}

/// Diagnostic: report each step of the OTP-HID probe as text (for troubleshooting).
pub fn probe_otp_hid_debug() -> String {
    let mut out = String::new();
    let devs = list_otp_hid_devices();
    out.push_str(&format!("HID VID 0x349e devices: {}\n", devs.len()));
    for (path, product) in &devs {
        out.push_str(&format!("- {product}  [{path}]\n"));
        match OtpHid::open(path) {
            Err(e) => { out.push_str(&format!("    open: ERR {e}\n")); continue; }
            Ok(hid) => {
                out.push_str("    open: ok, INIT ok\n");
                let sel = [0x00u8,0xa4,0x04,0x00,0x08,0xf0,0x00,0x00,0x01,0x4f,0x74,0x70,0x01,0x00];
                match hid.transmit(&sel) {
                    Err(e) => out.push_str(&format!("    SELECT OTP: ERR {e}\n")),
                    Ok((d, sw)) => {
                        out.push_str(&format!("    SELECT OTP: SW={sw:04X} data={}\n", d.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("")));
                        let mut ser = vec![0x80u8,0x33,0x00,0x00,0x12,0xd1,0x10];
                        ser.extend_from_slice(&[0u8;16]);
                        match hid.transmit(&ser) {
                            Err(e) => out.push_str(&format!("    serial: ERR {e}\n")),
                            Ok((d, sw)) => out.push_str(&format!("    serial: SW={sw:04X} data={}\n", d.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(""))),
                        }
                    }
                }
            }
        }
    }
    out
}
