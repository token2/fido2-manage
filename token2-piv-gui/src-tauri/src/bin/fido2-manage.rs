// fido2-manage — command-line FIDO2 token manager, drop-in compatible with the
// classic fido2-manage.exe flag set, built on the same libfido2 core as the
// Token2 Key Manager GUI. Part of the same workspace; shares code via the lib.
//
// Usage mirrors https://www.token2.com/.../fido2-manage-exe :
//   -list
//   -info            -device N
//   -storage         -device N
//   -residentKeys    -device N [-domain D]
//   -delete          -device N -credential <b64>
//   -setPIN          -device N [-pin P]
//   -changePIN       -device N [-pin P -newpin NP]  (interactive if omitted)
//   -reset           -device N
//   -forcePINchange  -device N [-pin P]
//   -setMinimumPIN M -device N [-pin P]
//   -uvs | -uvd      -device N [-pin P]
//   -fingerprint     -device N [-pin P] [-fingerprintname NAME]
//   -fingerprintlist -device N [-pin P]
//   -deletefingerprint ID -device N [-pin P]
//   -renamefingerprint ID -fingerprintname NAME -device N [-pin P]

use std::io::Write;
use token2_keymanager::fido::{self, FidoSession};

type R<T> = Result<T, String>;

fn main() {
    // If launched by double-click (Windows Explorer), the process owns a brand-new
    // console that closes instantly on exit, so the user sees a flash and nothing
    // else. Detect that case and show a "this is a command-line tool" notice, then
    // wait for a key so the message stays on screen.
    #[cfg(windows)]
    if launched_by_double_click() {
        print_cli_notice();
        pause();
        std::process::exit(0);
    }
    std::process::exit(match run() {
        Ok(()) => 0,
        Err(e) => { eprintln!("Error: {e}"); 1 }
    });
}

#[cfg(windows)]
fn launched_by_double_click() -> bool {
    // No command-line arguments AND this process is the only one attached to its
    // console (parent is Explorer, not a shell) -> almost certainly a double-click.
    if std::env::args().count() > 1 { return false; }
    extern "system" { fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32; }
    let mut buf = [0u32; 4];
    let n = unsafe { GetConsoleProcessList(buf.as_mut_ptr(), buf.len() as u32) };
    n <= 1
}

#[cfg(windows)]
fn print_cli_notice() {
    println!();
    println!("  fido2-manage is a COMMAND-LINE tool, not a window application.");
    println!("  It has no graphical interface — run it from a terminal (cmd or PowerShell).");
    println!();
    println!("  For the graphical app, run  token2-key-manager.exe  instead.");
    println!();
    println!("  Examples (open PowerShell in this folder):");
    println!("     .\\fido2-manage.exe -list");
    println!("     .\\fido2-manage.exe -info -device 1");
    println!("     .\\fido2-manage.exe -setPIN -device 1");
    println!();
    println!("  Most read operations on a Token2 key over USB work without administrator");
    println!("  rights. Raw-HID keys (e.g. YubiKey over USB) need an elevated terminal.");
    println!();
}

#[cfg(windows)]
fn pause() {
    use std::io::{Read, Write};
    print!("  Press Enter to close this window… ");
    std::io::stdout().flush().ok();
    let mut b = [0u8; 1];
    let _ = std::io::stdin().read(&mut b);
}

// ---- tiny arg parser (flags, and "-key value" pairs) ----
struct Args { flags: Vec<String>, vals: std::collections::HashMap<String, String> }
impl Args {
    fn parse() -> Self {
        let raw: Vec<String> = std::env::args().skip(1).collect();
        let mut flags = Vec::new();
        let mut vals = std::collections::HashMap::new();
        let takes_val = ["-device","-domain","-credential","-pin","-newpin",
                         "-setMinimumPIN","-setminimumPIN","-deletefingerprint",
                         "-renamefingerprint","-fingerprintname"];
        let mut i = 0;
        while i < raw.len() {
            let a = &raw[i];
            if takes_val.iter().any(|t| t.eq_ignore_ascii_case(a)) {
                let key = a.to_lowercase();
                let v = raw.get(i + 1).cloned().unwrap_or_default();
                vals.insert(key, v);
                i += 2;
            } else {
                flags.push(a.to_lowercase());
                i += 1;
            }
        }
        Args { flags, vals }
    }
    fn has(&self, f: &str) -> bool { self.flags.iter().any(|x| x == &f.to_lowercase()) }
    fn val(&self, k: &str) -> Option<String> { self.vals.get(&k.to_lowercase()).cloned() }
}

fn prompt(label: &str, hidden: bool) -> R<String> {
    eprint!("{label}: ");
    std::io::stderr().flush().ok();
    if !hidden {
        let mut s = String::new();
        std::io::stdin().read_line(&mut s).map_err(|e| e.to_string())?;
        return Ok(s.trim_end_matches(['\r', '\n']).to_string());
    }
    // masked input: echo an asterisk per character, support backspace.
    let v = read_masked()?;
    eprintln!();
    Ok(v)
}

#[cfg(windows)]
fn read_masked() -> R<String> {
    // Read raw console input without echo, printing '*' per char ourselves.
    use std::io::Read;
    extern "system" {
        fn GetStdHandle(n: u32) -> isize;
        fn GetConsoleMode(h: isize, m: *mut u32) -> i32;
        fn SetConsoleMode(h: isize, m: u32) -> i32;
    }
    const STD_INPUT_HANDLE: u32 = 0xFFFF_FFF6; // (DWORD)-10
    const ENABLE_ECHO_INPUT: u32 = 0x0004;
    const ENABLE_LINE_INPUT: u32 = 0x0002;
    let h = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut orig: u32 = 0;
    let have_mode = unsafe { GetConsoleMode(h, &mut orig) } != 0;
    if have_mode {
        unsafe { SetConsoleMode(h, orig & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT)); }
    }
    let mut out = String::new();
    let mut byte = [0u8; 1];
    let mut stdin = std::io::stdin();
    loop {
        if stdin.read(&mut byte).map_err(|e| e.to_string())? == 0 { break; }
        match byte[0] {
            b'\r' | b'\n' => break,
            8 | 127 => { if out.pop().is_some() { eprint!("\u{8} \u{8}"); std::io::stderr().flush().ok(); } }
            3 => { if have_mode { unsafe { SetConsoleMode(h, orig); } } return Err("cancelled".into()); }
            c => { out.push(c as char); eprint!("*"); std::io::stderr().flush().ok(); }
        }
    }
    if have_mode { unsafe { SetConsoleMode(h, orig); } }
    Ok(out)
}

#[cfg(unix)]
fn read_masked() -> R<String> {
    use std::io::Read;
    use std::os::unix::io::AsRawFd;
    // termios raw-ish: disable ECHO and canonical mode on the tty.
    #[repr(C)]
    #[derive(Clone)]
    struct Termios { c_iflag: u32, c_oflag: u32, c_cflag: u32, c_lflag: u32, c_line: u8, c_cc: [u8; 32], c_ispeed: u32, c_ospeed: u32 }
    extern "C" {
        fn tcgetattr(fd: i32, t: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, actions: i32, t: *const Termios) -> i32;
    }
    const ECHO: u32 = 0o0000010;
    const ICANON: u32 = 0o0000002;
    const TCSANOW: i32 = 0;
    let fd = std::io::stdin().as_raw_fd();
    let mut orig: Termios = unsafe { std::mem::zeroed() };
    let have = unsafe { tcgetattr(fd, &mut orig) } == 0;
    if have {
        let mut raw = orig.clone();
        raw.c_lflag &= !(ECHO | ICANON);
        unsafe { tcsetattr(fd, TCSANOW, &raw); }
    }
    let mut out = String::new();
    let mut byte = [0u8; 1];
    let mut stdin = std::io::stdin();
    loop {
        if stdin.read(&mut byte).map_err(|e| e.to_string())? == 0 { break; }
        match byte[0] {
            b'\r' | b'\n' => break,
            8 | 127 => { if out.pop().is_some() { eprint!("\u{8} \u{8}"); std::io::stderr().flush().ok(); } }
            3 => { if have { unsafe { tcsetattr(fd, TCSANOW, &orig); } } return Err("cancelled".into()); }
            c => { out.push(c as char); eprint!("*"); std::io::stderr().flush().ok(); }
        }
    }
    if have { unsafe { tcsetattr(fd, TCSANOW, &orig); } }
    Ok(out)
}

// device list, numbered from 1 like fido2-manage
fn devices() -> R<Vec<fido::FidoDevice>> { fido::fido_list_devices() }

fn device_name(args: &Args) -> Option<String> {
    let list = devices().ok()?;
    let n: usize = args.val("-device").and_then(|v| v.parse().ok()).unwrap_or(1);
    list.get(n.saturating_sub(1)).map(|d| if d.product.is_empty() { d.manufacturer.clone() } else { d.product.clone() })
}

fn open_device(args: &Args) -> R<(FidoSession, fido::FidoInfo)> {
    let list = devices()?;
    if list.is_empty() { return Err("no FIDO2 devices found".into()); }
    let n: usize = args.val("-device").and_then(|v| v.parse().ok()).unwrap_or(1);
    if n == 0 || n > list.len() {
        return Err(format!("device {n} out of range (1..{})", list.len()));
    }
    let path = list[n - 1].path.clone();
    FidoSession::open(&path)
}

fn pin_arg(args: &Args, confirm_new: bool) -> R<String> {
    if let Some(p) = args.val("-pin") { return Ok(p); }
    let p = prompt("Enter PIN", true)?;
    if confirm_new {
        let p2 = prompt("Confirm PIN", true)?;
        if p != p2 { return Err("PINs do not match".into()); }
    }
    Ok(p)
}

fn run() -> R<()> {
    let args = Args::parse();

    if args.has("-list") || (args.flags.is_empty() && args.vals.is_empty()) {
        let list = devices()?;
        if list.is_empty() { println!("No FIDO2 devices found."); return Ok(()); }
        for (i, d) in list.iter().enumerate() {
            let name = if !d.product.is_empty() { &d.product } else { &d.manufacturer };
            println!("{}: {}  [{}]  {:04x}:{:04x}  {}",
                     i + 1, name, d.transport, d.vendor_id, d.product_id, d.path);
        }
        return Ok(());
    }

    if args.has("-info") {
        let prod = device_name(&args).unwrap_or_default();
        let (s, info) = open_device(&args)?;
        print_info(&prod, &info);
        drop(s);
        return Ok(());
    }

    if args.has("-storage") {
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        let inv = s.list_passkeys(&pin)?;
        println!("resident credentials: {} stored, {} free", inv.existing, inv.remaining);
        return Ok(());
    }

    if args.has("-residentkeys") {
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        let inv = s.list_passkeys(&pin)?;
        let domain = args.val("-domain");
        for rp in &inv.rps {
            if let Some(ref d) = domain {
                if !rp.id.eq_ignore_ascii_case(d) && !rp.name.eq_ignore_ascii_case(d) { continue; }
            }
            println!("RP: {} ({})", rp.id, rp.name);
            if domain.is_some() {
                for c in &rp.credentials {
                    println!("   credential: {}  user: {} ({})  [{}]",
                             to_b64(&hex_to_bytes(&c.cred_id)), c.user_name, c.display_name, c.key_type);
                }
            }
        }
        return Ok(());
    }

    if args.has("-delete") {
        let (s, _) = open_device(&args)?;
        let cred = args.val("-credential").ok_or("missing -credential <base64>")?;
        let pin = pin_arg(&args, false)?;
        eprint!("Delete credential {cred}? This cannot be undone. Type YES to confirm: ");
        std::io::stderr().flush().ok();
        let mut c = String::new();
        std::io::stdin().read_line(&mut c).ok();
        if c.trim() != "YES" { return Err("cancelled".into()); }
        s.delete_passkey(&cred, &pin)?;
        println!("Credential deleted.");
        return Ok(());
    }

    if args.has("-setpin") {
        let (s, _) = open_device(&args)?;
        let new = match args.val("-pin") {
            Some(p) => p,
            None => {
                let p = prompt("Enter new PIN", true)?;
                let p2 = prompt("Confirm new PIN", true)?;
                if p != p2 { return Err("PINs do not match".into()); }
                p
            }
        };
        s.set_pin(&new, None)?;
        println!("PIN set.");
        return Ok(());
    }

    if args.has("-changepin") {
        let (s, _) = open_device(&args)?;
        let old = match args.val("-pin") { Some(p) => p, None => prompt("Enter current PIN", true)? };
        let new = match args.val("-newpin") {
            Some(p) => p,
            None => {
                let p = prompt("Enter new PIN", true)?;
                let p2 = prompt("Confirm new PIN", true)?;
                if p != p2 { return Err("PINs do not match".into()); }
                p
            }
        };
        s.set_pin(&new, Some(&old))?;
        println!("PIN changed.");
        return Ok(());
    }

    if args.has("-reset") {
        let (s, _) = open_device(&args)?;
        eprintln!("Factory reset must be done within ~10 s of plugging the key in.");
        eprint!("Touch the key when it blinks… ");
        std::io::stderr().flush().ok();
        s.reset()?;
        println!("\nFIDO applet reset.");
        return Ok(());
    }

    if args.has("-forcepinchange") {
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        s.force_pin_change(&pin)?;
        println!("The key will require a PIN change at next use.");
        return Ok(());
    }

    if let Some(m) = args.val("-setminimumpin") {
        let len: usize = m.parse().map_err(|_| "setMinimumPIN needs a number")?;
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        s.set_min_pin(len, &pin)?;
        println!("Minimum PIN length set to {len}.");
        return Ok(());
    }

    if args.has("-uvs") || args.has("-uvd") {
        let enable = args.has("-uvs");
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        s.set_always_uv(enable, &pin)?;
        println!("Always-UV {}.", if enable { "enabled" } else { "disabled" });
        return Ok(());
    }

    if args.has("-fingerprintlist") {
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        let bio = s.list_bio(&pin)?;
        if bio.templates.is_empty() { println!("No fingerprints enrolled."); }
        for (i, t) in bio.templates.iter().enumerate() {
            println!("{}: {}  (id {})", i + 1,
                     if t.name.is_empty() { "(unnamed)" } else { &t.name }, t.id);
        }
        return Ok(());
    }

    if let Some(id) = args.val("-deletefingerprint") {
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        let hex_id = resolve_bio_id(&s, &pin, &id)?;
        s.delete_bio(&hex_id, &pin)?;
        println!("Fingerprint deleted.");
        return Ok(());
    }

    if let Some(id) = args.val("-renamefingerprint") {
        let name = args.val("-fingerprintname").ok_or("missing -fingerprintname NAME")?;
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        let hex_id = resolve_bio_id(&s, &pin, &id)?;
        s.rename_bio(&hex_id, &name, &pin)?;
        println!("Fingerprint renamed to {name}.");
        return Ok(());
    }

    if args.has("-fingerprint") {
        let (s, _) = open_device(&args)?;
        let pin = pin_arg(&args, false)?;
        let name = args.val("-fingerprintname").unwrap_or_default();
        println!("Enrolling a fingerprint — touch the sensor repeatedly until complete.");
        s.enroll_fingerprint(&name, &pin, |remaining, status| {
            eprintln!("  sample captured (status {status}), {remaining} more needed");
        })?;
        println!("Fingerprint enrolled{}.", if name.is_empty() { String::new() } else { format!(" as \"{name}\"") });
        return Ok(());
    }

    Err("no recognised command — run with -list, -info, -setPIN, -fingerprint, … (see -help on the website)".into())
}

fn print_info(product: &str, info: &fido::FidoInfo) {
    println!("product:    {}", product);
    println!("transport:  {}", info.transport);
    println!("firmware:   {}", info.firmware);
    println!("versions:   {}", info.versions.join(", "));
    println!("aaguid:     {}", info.aaguid);
    println!("PIN set:    {}", if info.has_pin { "yes" } else { "no" });
    println!("min PIN:    {}", info.min_pin_len);
    println!("always-UV:  {}", if info.always_uv { "on" } else { "off" });
    if info.supports_bio { println!("biometric:  yes"); }
}

// -deletefingerprint / -renamefingerprint accept the slot NUMBER shown by
// -fingerprintlist; resolve that to the template's hex id the API needs.
fn resolve_bio_id(s: &FidoSession, pin: &str, id: &str) -> R<String> {
    if let Ok(n) = id.parse::<usize>() {
        let bio = s.list_bio(pin)?;
        let t = bio.templates.get(n.saturating_sub(1))
            .ok_or_else(|| format!("fingerprint {n} not found (see -fingerprintlist)"))?;
        return Ok(t.id.clone());
    }
    Ok(id.to_string()) // already a hex id
}

// small hex/base64 helpers for credential display (match fido2-manage's b64 ids)
fn hex_to_bytes(h: &str) -> Vec<u8> {
    (0..h.len()).step_by(2).filter_map(|i| u8::from_str_radix(h.get(i..i+2)?, 16).ok()).collect()
}
fn to_b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        out.push(T[(b[0] >> 2) as usize] as char);
        out.push(T[(((b[0] & 3) << 4) | (b[1] >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 { T[(((b[1] & 15) << 2) | (b[2] >> 6)) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(b[2] & 63) as usize] as char } else { '=' });
    }
    out
}
