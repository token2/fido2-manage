const { invoke } = window.__TAURI__.core;
const $ = (s, r = document) => {
  const el = r.querySelector(s);
  if (!el) console.warn("missing element", s);
  return el || { addEventListener() {}, textContent: "", innerHTML: "", value: "", hidden: true, classList: { toggle() {}, add() {}, remove() {} }, style: {}, append() {}, dataset: {}, close() {}, showModal() {}, querySelector: () => null };
};
const $$ = (s, r = document) => [...r.querySelectorAll(s)];

// ---- inline SVG icons on buttons and headings (added once at load) ----
var ICONS = {
  fingerprint: '<path d="M12 1C16.9706 1 21 5.02944 21 10V14C21 18.9706 16.9706 23 12 23C10.9137 23 9.8724 22.8076 8.90826 22.4549C9.03638 22.2782 9.15938 22.0977 9.27703 21.9134L9.44782 21.633C10.388 20.0636 10.9461 18.2391 10.9963 16.2884L11 16V9H13V16C13 17.7724 12.6453 19.4619 12.0031 21.0015C12.7954 21 13.5599 20.8673 14.2724 20.6229C14.7147 19.2616 14.966 17.8148 14.9968 16.3138L15 16L14.9998 12.999H16.9998L17 16C17 17.0885 16.8977 18.1531 16.7022 19.1847C18.0583 17.9552 18.9297 16.2 18.9959 14.2407L19 14V10C19 6.13401 15.866 3 12 3C10.4277 3 8.97638 3.51841 7.8078 4.39364L6.38282 2.96769C7.92242 1.73631 9.87522 1 12 1ZM7 10C7 7.23858 9.23858 5 12 5C14.7614 5 17 7.23858 17 10V11H15V10C15 8.34315 13.6569 7 12 7C10.4023 7 9.09634 8.24892 9.00509 9.82373L9 10V16C9 17.5669 8.5996 19.0402 7.89554 20.3233L7.87214 20.3627C7.64284 20.7771 7.38087 21.1711 7.09037 21.5417C6.6495 21.2545 6.23541 20.9297 5.85264 20.5719L5.5445 20.2711C3.96956 18.65 3 16.4382 3 14V10C3 7.87522 3.73631 5.92242 4.96769 4.38282L6.39364 5.8078C5.56325 6.91652 5.05405 8.27971 5.00406 9.75935L5 10V14C5 15.6748 5.58816 17.2122 6.56918 18.4169C6.82239 17.7351 6.97017 17.0034 6.99594 16.2407L7 16V10Z"/>',
  key: '<path d="M10.7577 11.8281L18.6066 3.97919L20.0208 5.3934L18.6066 6.80761L21.0815 9.28249L19.6673 10.6967L17.1924 8.22183L15.7782 9.63604L17.8995 11.7574L16.4853 13.1716L14.364 11.0503L12.1719 13.2423C13.4581 15.1837 13.246 17.8251 11.5355 19.5355C9.58291 21.4882 6.41709 21.4882 4.46447 19.5355C2.51184 17.5829 2.51184 14.4171 4.46447 12.4645C6.17493 10.754 8.81633 10.5419 10.7577 11.8281ZM10.1213 18.1213C11.2929 16.9497 11.2929 15.0503 10.1213 13.8787C8.94975 12.7071 7.05025 12.7071 5.87868 13.8787C4.70711 15.0503 4.70711 16.9497 5.87868 18.1213C7.05025 19.2929 8.94975 19.2929 10.1213 18.1213Z"/>',
  shield: '<path d="M12 1L20.2169 2.82598C20.6745 2.92766 21 3.33347 21 3.80217V13.7889C21 15.795 19.9974 17.6684 18.3282 18.7812L12 23L5.6718 18.7812C4.00261 17.6684 3 15.795 3 13.7889V3.80217C3 3.33347 3.32553 2.92766 3.78307 2.82598L12 1ZM12 3.04879L5 4.60434V13.7889C5 15.1263 5.6684 16.3752 6.7812 17.1171L12 20.5963L17.2188 17.1171C18.3316 16.3752 19 15.1263 19 13.7889V4.60434L12 3.04879ZM12 7C13.1046 7 14 7.89543 14 9C14 9.73984 13.5983 10.3858 13.0011 10.7318L13 15H11L10.9999 10.7324C10.4022 10.3866 10 9.74025 10 9C10 7.89543 10.8954 7 12 7Z"/>',
  pin: '<path d="M18 8H20C20.5523 8 21 8.44772 21 9V21C21 21.5523 20.5523 22 20 22H4C3.44772 22 3 21.5523 3 21V9C3 8.44772 3.44772 8 4 8H6V7C6 3.68629 8.68629 1 12 1C15.3137 1 18 3.68629 18 7V8ZM5 10V20H19V10H5ZM11 14H13V16H11V14ZM7 14H9V16H7V14ZM15 14H17V16H15V14ZM16 8V7C16 4.79086 14.2091 3 12 3C9.79086 3 8 4.79086 8 7V8H16Z"/>',
  plus: '<path d="M11 11V5H13V11H19V13H13V19H11V13H5V11H11Z"/>',
  trash: '<path d="M17 6H22V8H20V21C20 21.5523 19.5523 22 19 22H5C4.44772 22 4 21.5523 4 21V8H2V6H7V3C7 2.44772 7.44772 2 8 2H16C16.5523 2 17 2.44772 17 3V6ZM18 8H6V20H18V8ZM9 11H11V17H9V11ZM13 11H15V17H13V11ZM9 4V6H15V4H9Z"/>',
  reset: '<path d="M18.5374 19.5674C16.7844 21.0831 14.4993 22 12 22C6.47715 22 2 17.5228 2 12C2 6.47715 6.47715 2 12 2C17.5228 2 22 6.47715 22 12C22 14.1361 21.3302 16.1158 20.1892 17.7406L17 12H20C20 7.58172 16.4183 4 12 4C7.58172 4 4 7.58172 4 12C4 16.4183 7.58172 20 12 20C14.1502 20 16.1022 19.1517 17.5398 17.7716L18.5374 19.5674Z"/>',
  download: '<path d="M13 10H18L12 16L6 10H11V3H13V10ZM4 19H20V12H22V20C22 20.5523 21.5523 21 21 21H3C2.44772 21 2 20.5523 2 20V12H4V19Z"/>',
  upload: '<path d="M4 19H20V12H22V20C22 20.5523 21.5523 21 21 21H3C2.44772 21 2 20.5523 2 20V12H4V19ZM13 9V16H11V9H6L12 3L18 9H13Z"/>',
  refresh: '<path d="M5.46257 4.43262C7.21556 2.91688 9.5007 2 12 2C17.5228 2 22 6.47715 22 12C22 14.1361 21.3302 16.1158 20.1892 17.7406L17 12H20C20 7.58172 16.4183 4 12 4C9.84982 4 7.89777 4.84827 6.46023 6.22842L5.46257 4.43262ZM18.5374 19.5674C16.7844 21.0831 14.4993 22 12 22C6.47715 22 2 17.5228 2 12C2 9.86386 2.66979 7.88416 3.8108 6.25944L7 12H4C4 16.4183 7.58172 20 12 20C14.1502 20 16.1022 19.1517 17.5398 17.7716L18.5374 19.5674Z"/>',
  cert: '<path d="M17 15.2454V22.1169C17 22.393 16.7761 22.617 16.5 22.617C16.4094 22.617 16.3205 22.5923 16.2428 22.5457L12 20L7.75725 22.5457C7.52046 22.6877 7.21333 22.6109 7.07125 22.3742C7.02463 22.2964 7 22.2075 7 22.1169V15.2454C5.17107 13.7793 4 11.5264 4 9C4 4.58172 7.58172 1 12 1C16.4183 1 20 4.58172 20 9C20 11.5264 18.8289 13.7793 17 15.2454ZM9 16.4185V19.4676L12 17.6676L15 19.4676V16.4185C14.0736 16.7935 13.0609 17 12 17C10.9391 17 9.92643 16.7935 9 16.4185ZM12 15C15.3137 15 18 12.3137 18 9C18 5.68629 15.3137 3 12 3C8.68629 3 6 5.68629 6 9C6 12.3137 8.68629 15 12 15Z"/>',
  lock: '<path d="M19 10H20C20.5523 10 21 10.4477 21 11V21C21 21.5523 20.5523 22 20 22H4C3.44772 22 3 21.5523 3 21V11C3 10.4477 3.44772 10 4 10H5V9C5 5.13401 8.13401 2 12 2C15.866 2 19 5.13401 19 9V10ZM5 12V20H19V12H5ZM11 14H13V18H11V14ZM17 10V9C17 6.23858 14.7614 4 12 4C9.23858 4 7 6.23858 7 9V10H17Z"/>',
  usb: '<path d="M12 1L15 6H13V13.381L16 11.882L15.999 11H15V7H19V11H17.999L18 13.118L13 15.618L13.0009 17.171C14.1656 17.5831 15 18.6941 15 20C15 21.6569 13.6569 23 12 23C10.3431 23 9 21.6569 9 20C9 18.813 9.68934 17.7871 10.6895 17.3006L6 14L5.99892 11.7318C5.40172 11.3858 5 10.7398 5 10C5 8.89543 5.89543 8 7 8C8.10457 8 9 8.89543 9 10C9 10.7403 8.59783 11.3866 8.00007 11.7324L8 13L11 15.086V6H9L12 1ZM12 19C11.4477 19 11 19.4477 11 20C11 20.5523 11.4477 21 12 21C12.5523 21 13 20.5523 13 20C13 19.4477 12.5523 19 12 19Z"/>',
  info: '<path d="M12 22C6.47715 22 2 17.5228 2 12C2 6.47715 6.47715 2 12 2C17.5228 2 22 6.47715 22 12C22 17.5228 17.5228 22 12 22ZM12 20C16.4183 20 20 16.4183 20 12C20 7.58172 16.4183 4 12 4C7.58172 4 4 7.58172 4 12C4 16.4183 7.58172 20 12 20ZM11 7H13V9H11V7ZM11 11H13V17H11V11Z"/>',
  otp: '<path d="M17.6177 5.9681L19.0711 4.51472L20.4853 5.92893L19.0319 7.38231C20.2635 8.92199 21 10.875 21 13C21 17.9706 16.9706 22 12 22C7.02944 22 3 17.9706 3 13C3 8.02944 7.02944 4 12 4C14.125 4 16.078 4.73647 17.6177 5.9681ZM12 20C15.866 20 19 16.866 19 13C19 9.13401 15.866 6 12 6C8.13401 6 5 9.13401 5 13C5 16.866 8.13401 20 12 20ZM11 8H13V14H11V8ZM8 1H16V3H8V1Z"/>',
  settings: '<path d="M3.33946 17.0002C2.90721 16.2515 2.58277 15.4702 2.36133 14.6741C3.3338 14.1779 3.99972 13.1668 3.99972 12.0002C3.99972 10.8345 3.3348 9.824 2.36353 9.32741C2.81025 7.71651 3.65857 6.21627 4.86474 4.99001C5.7807 5.58416 6.98935 5.65534 7.99972 5.072C9.01009 4.48866 9.55277 3.40635 9.4962 2.31604C11.1613 1.8846 12.8847 1.90004 14.5031 2.31862C14.4475 3.40806 14.9901 4.48912 15.9997 5.072C17.0101 5.65532 18.2187 5.58416 19.1346 4.99007C19.7133 5.57986 20.2277 6.25151 20.66 7.00021C21.0922 7.7489 21.4167 8.53025 21.6381 9.32628C20.6656 9.82247 19.9997 10.8336 19.9997 12.0002C19.9997 13.166 20.6646 14.1764 21.6359 14.673C21.1892 16.2839 20.3409 17.7841 19.1347 19.0104C18.2187 18.4163 17.0101 18.3451 15.9997 18.9284C14.9893 19.5117 14.4467 20.5941 14.5032 21.6844C12.8382 22.1158 11.1148 22.1004 9.49633 21.6818C9.55191 20.5923 9.00929 19.5113 7.99972 18.9284C6.98938 18.3451 5.78079 18.4162 4.86484 19.0103C4.28617 18.4205 3.77172 17.7489 3.33946 17.0002ZM8.99972 17.1964C10.0911 17.8265 10.8749 18.8227 11.2503 19.9659C11.7486 20.0133 12.2502 20.014 12.7486 19.9675C13.1238 18.8237 13.9078 17.8268 14.9997 17.1964C16.0916 16.5659 17.347 16.3855 18.5252 16.6324C18.8146 16.224 19.0648 15.7892 19.2729 15.334C18.4706 14.4373 17.9997 13.2604 17.9997 12.0002C17.9997 10.74 18.4706 9.5632 19.2729 8.6665C19.1688 8.4405 19.0538 8.21822 18.9279 8.00021C18.802 7.78219 18.667 7.57148 18.5233 7.36842C17.3457 7.61476 16.0911 7.43414 14.9997 6.80405C13.9083 6.17395 13.1246 5.17768 12.7491 4.03455C12.2509 3.98714 11.7492 3.98646 11.2509 4.03292C10.8756 5.17671 10.0916 6.17364 8.99972 6.80405C7.9078 7.43447 6.65245 7.61494 5.47428 7.36803C5.18485 7.77641 4.93463 8.21117 4.72656 8.66637C5.52881 9.56311 5.99972 10.74 5.99972 12.0002C5.99972 13.2604 5.52883 14.4372 4.72656 15.3339C4.83067 15.5599 4.94564 15.7822 5.07152 16.0002C5.19739 16.2182 5.3324 16.4289 5.47612 16.632C6.65377 16.3857 7.90838 16.5663 8.99972 17.1964ZM11.9997 15.0002C10.3429 15.0002 8.99972 13.6571 8.99972 12.0002C8.99972 10.3434 10.3429 9.00021 11.9997 9.00021C13.6566 9.00021 14.9997 10.3434 14.9997 12.0002C14.9997 13.6571 13.6566 15.0002 11.9997 15.0002ZM11.9997 13.0002C12.552 13.0002 12.9997 12.5525 12.9997 12.0002C12.9997 11.4479 12.552 11.0002 11.9997 11.0002C11.4474 11.0002 10.9997 11.4479 10.9997 12.0002C10.9997 12.5525 11.4474 13.0002 11.9997 13.0002Z"/>',
  passkey: '<path d="M12.917 13C12.441 15.8377 9.973 18 7 18C3.68629 18 1 15.3137 1 12C1 8.68629 3.68629 6 7 6C9.973 6 12.441 8.16229 12.917 11H23V13H21V17H19V13H17V17H15V13H12.917ZM7 16C9.20914 16 11 14.2091 11 12C11 9.79086 9.20914 8 7 8C4.79086 8 3 9.79086 3 12C3 14.2091 4.79086 16 7 16Z"/>',
};
function svg(name) { const i = ICONS[name]; return i ? `<svg class="ico" viewBox="0 0 24 24">${i}</svg>` : ""; }
function iconize() {
  const map = [
    ["btn-t2-fp","fingerprint"],["btn-t2-unlock-fp","fingerprint"],["btn-fido-unlock-uv","fingerprint"],["op-fp","fingerprint"],
    ["btn-fido-enroll","fingerprint"],["btn-t2-setpin","pin"],["btn-oath-setpw","pin"],["btn-oath-lock","lock"],["btn-t2-lock","lock"],
    ["btn-t2-add","plus"],["btn-oath-add","plus"],["btn-t2-erase","trash"],["btn-oath-reset","reset"],["btn-reset","reset"],
    ["btn-fido-reset","reset"],["fs-reset","reset"],["op-lock","lock"],["op-removepin","trash"],
    ["btn-t2-btnhotp","usb"],["btn-t2-ifaces","usb"],
  ];
  for (const [id, name] of map) { const b = document.getElementById(id); if (b && !b.querySelector(".ico")) b.insertAdjacentHTML("afterbegin", svg(name)); }
  // menu items by data-mi
  const mi = { "piv-certs":"cert","piv-pin":"pin","piv-admin":"key","piv-puk":"key","piv-unblock":"key","piv-retries":"pin","piv-attest":"shield","piv-reset":"reset",
    "fido-pin":"pin","fido-passkeys":"key","fido-fingerprints":"fingerprint","fido-settings":"settings","fido-reset":"reset",
    "otp-accounts":"otp","otp-hidhotp":"usb","otp-protection":"shield","otp-reset":"reset",
    "device-settings":"settings","connect-all":"usb","rescan":"refresh","disconnect-all":"lock","exit":"info","about":"info" };
  for (const [k, name] of Object.entries(mi)) { const b = document.querySelector(`[data-mi="${k}"]`); if (b && !b.querySelector(".ico")) b.insertAdjacentHTML("afterbegin", svg(name)); }
  // per-slot action buttons render dynamically; icons added in their render via data-icon (below)
}
iconize();

let info = null;
let slots = [];
let algos = [];

// ---- helpers ---------------------------------------------------------------
function toast(msg, bad = false) {
  const t = $("#toast");
  t.textContent = msg;
  t.className = bad ? "bad" : "";
  t.hidden = false;
  clearTimeout(t._t);
  t._t = setTimeout(() => (t.hidden = true), bad ? 6000 : 3000);
}
let _opInFlight = 0;
async function busy(fn) {
  document.body.classList.add("busy");
  _opInFlight++;
  try { return await fn(); }
  catch (e) { toast(String(e), true); throw e; }
  finally { _opInFlight--; document.body.classList.remove("busy"); }
}

// Indeterminate progress for operations that take seconds to minutes on the
// device (key generation, on-card signing). The card gives no progress
// feedback, so we show the stage and elapsed time.
const progress = {
  t0: 0, timer: null,
  show(text) {
    if (!this.timer) {
      this.t0 = Date.now();
      this.timer = setInterval(() => ($("#progress-elapsed").textContent = `${Math.round((Date.now() - this.t0) / 1000)} s`), 500);
      $("#progress-elapsed").textContent = "0 s";
    }
    $("#progress-text").textContent = text;
    $("#progress").hidden = false;
  },
  hide() { clearInterval(this.timer); this.timer = null; $("#progress").hidden = true; },
};
const ALGO_HINT = (a) => /RSA4096/.test(a) ? " This takes 1–3 minutes on the device." : /RSA3072/.test(a) ? " This takes up to a minute." : /RSA/.test(a) ? " This takes a few seconds." : "";
function fmtDate(ts) {
  return new Date(ts * 1000).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}
function family(algo) {
  if (!algo) return "";
  if (algo.startsWith("RSA")) return "rsa";
  if (algo.startsWith("ECC")) return "ecc";
  return "edx";
}
const PIN_POLICY = { 0: "default", 1: "no PIN", 2: "PIN once", 3: "PIN always", 4: "match once", 5: "match always" };
const TOUCH_POLICY = { 0: "", 1: "no touch", 2: "touch always", 3: "touch cached" };
function policies(m) {
  const p = [PIN_POLICY[m.pin_policy], TOUCH_POLICY[m.touch_policy], m.origin === 2 ? "imported" : "generated"];
  return p.filter(Boolean).join(" · ").replace(/ · /g, ", ");
}
function showText(title, text, filename) {
  $("#text-title").textContent = title;
  $("#text-body").value = text;
  const dlg = $("#dlg-text");
  dlg.onclose = async () => {
    if (dlg.returnValue === "save") {
      await busy(async () => { if (await invoke("save_text", { suggestedName: filename, text })) toast("Saved"); });
    }
  };
  dlg.showModal();
}
function confirmDialog(title, body, { mgm = false, pin = false, okLabel = "Confirm", defaults = false } = {}) {
  return new Promise((resolve) => {
    $("#confirm-title").textContent = title;
    $("#confirm-body").textContent = body;
    $("#confirm-mgm-wrap").hidden = !mgm;
    $("#confirm-mgm").value = "";
    $("#confirm-pin-wrap").hidden = !pin;
    $("#confirm-pin").value = "";
    // default-PIN chips only for PIV (mgm) contexts; never for FIDO/OTP PINs
    const chips = $("#confirm-pin-wrap .defaults");
    if (chips) chips.style.display = defaults ? "" : "none";
    const mchips = $("#confirm-mgm-wrap .defaults");
    if (mchips) mchips.style.display = (defaults || mgm) ? "" : "none";
    $("#confirm-ok").textContent = okLabel;
    const dlg = $("#dlg-confirm");
    dlg.onclose = () => resolve(dlg.returnValue === "ok" ? { ok: true, mgm: $("#confirm-mgm").value, pin: $("#confirm-pin").value } : { ok: false });
    dlg.showModal();
  });
}

function showAttestation(label, id, rep) {
  $("#attest-title").textContent = `Attestation for slot ${label}`;
  const v = $("#attest-verdict");
  v.textContent = !rep.valid ? "Attestation could not be verified."
    : rep.warnings ? "Key was generated on this Token2 device — chain verifies against TOKEN2 PIV CA (with a provisioning warning)."
    : "Key was generated on this Token2 device — chain verifies against TOKEN2 PIV CA.";
  v.className = "verdict " + (!rep.valid ? "bad" : rep.warnings ? "warn" : "ok");
  const ul = $("#attest-checks");
  ul.innerHTML = "";
  for (const [name, level, detail] of rep.checks) {
    const li = document.createElement("li");
    li.className = level;
    li.textContent = name;
    if (detail) li.append(document.createTextNode(" — "), Object.assign(document.createElement("small"), { textContent: detail }));
    ul.append(li);
  }
  const props = [
    ["Attested key", rep.leaf ? rep.leaf.key_algorithm : ""],
    ["Certificate subject", rep.leaf ? rep.leaf.subject : ""],
    ["Firmware version", rep.firmware_version],
    ["Device serial in certificate", rep.device_serial],
    ["PIN / touch policy", [rep.pin_policy, rep.touch_policy].filter(Boolean).join(" / ")],
    ["Form factor", rep.form_factor],
    ["CA fingerprint (SHA-256)", rep.root_fingerprint],
  ].filter(([, val]) => val);
  const box = $("#attest-props");
  box.innerHTML = "";
  for (const [k, val] of props) {
    const d = document.createElement("div");
    d.innerHTML = `<span>${esc(k)}</span><b>${esc(val)}</b>`;
    box.append(d);
  }
  const chain = rep.leaf_pem + rep.intermediate_pem;
  $("#attest-pem").value = chain;
  const dlg = $("#dlg-attest");
  dlg.onclose = async () => {
    if (dlg.returnValue === "save") {
      await busy(async () => { if (await invoke("save_text", { suggestedName: `slot-${id}-attestation-chain.pem`, text: chain })) toast("Saved"); });
    }
  };
  dlg.showModal();
}

// ---- readers / connection --------------------------------------------------
async function loadReaders() {
  // manual reader picker is a fallback; auto-detect drives normal use
  const sel = $("#reader");
  sel.innerHTML = "";
  try {
    const readers = await invoke("list_readers");
    for (const r of readers) {
      const o = document.createElement("option");
      o.value = o.textContent = r;
      if (/token2/i.test(r)) o.selected = true;
      sel.append(o);
    }
    $("#connect-msg").textContent = readers.length ? "" : "No smart card readers found. Plug in the key and rescan.";
  } catch (e) {
    $("#connect-msg").textContent = String(e);
  }
}

async function connect(readerArg) {
  let reader = readerArg || $("#reader").value || null;
  if (reader && typeof reader !== "string") reader = String(reader.name || reader.reader || reader.value || "");
  if (reader && $("#reader")) {
    if (![...$("#reader").options].some((o) => o.value === reader)) { const o = document.createElement("option"); o.value = o.textContent = reader; $("#reader").append(o); }
    $("#reader").value = reader;
  }
  await busy(async () => {
    info = await invoke("connect", { reader });
    refreshCaps();
    renderInfo();
    try { algos = await invoke("supported_algorithms"); } catch (_) { algos = []; }
    fillAlgorithms();
    await refreshSlots();
    try { pivMoveKeySupported = await invoke("piv_supports_key_delete"); } catch (_) { pivMoveKeySupported = false; }
    if (page === "piv") setMode("piv");
    if (page === "home") renderHome(); renderIdent();
  });
}
async function disconnect() {
  await invoke("disconnect");
  info = null; slots = [];
  setMode("piv");
  loadReaders();
}
async function refreshInfo() {
  info = await invoke("get_info");
  renderInfo();
}
function renderInfo() {
  $("#k-serial").textContent = info.serial_full || (info.serial ? String(info.serial) : "—");
  $("#k-model-row").hidden = !info.model;
  if (info.model) $("#k-model").textContent = `${info.model.revision} · ${info.model.model}${info.model.branding !== "Token2" ? " · " + info.model.branding : ""}`;
  $("#k-version").textContent = info.version;
  $("#k-mgm").textContent = [info.mgm_algo, info.mgm_type === "protected" ? "PIN-protected" : info.mgm_type === "derived" ? "PIN-derived" : ""].filter(Boolean).join(", ") || "—";
  const n = info.pin_retries;
  const pips = $("#k-pin-pips");
  pips.innerHTML = "";
  const max = Math.max(n, 3);
  for (let i = 0; i < Math.min(max, 10); i++) pips.append(Object.assign(document.createElement("i"), { className: i < n ? "on" : "" }));
  pips.className = "pips" + (n === 0 ? " none" : n === 1 ? " low" : "");
  $("#k-pin-retries").textContent = n < 0 ? "?" : n;
  $("#k-puk").textContent = info.puk_blocked ? "PUK is blocked — unblock PIN is not available." : "";
}

function fillAlgorithms() {
  const sel = $("#gen-algorithm");
  sel.innerHTML = "";
  const list = algos.length ? algos : [
    { name: "RSA2048", label: "RSA 2048", can_sign: true }, { name: "RSA3072", label: "RSA 3072", can_sign: true },
    { name: "RSA4096", label: "RSA 4096", can_sign: true }, { name: "ECCP256", label: "ECC P-256 (secp256r1)", can_sign: true },
    { name: "ECCP384", label: "ECC P-384 (secp384r1)", can_sign: true }, { name: "ED25519", label: "Ed25519", can_sign: true },
    { name: "X25519", label: "X25519 (key agreement only)", can_sign: false },
  ];
  for (const a of list) {
    const o = document.createElement("option");
    o.value = a.name; o.textContent = a.label; o.dataset.canSign = a.can_sign ? "1" : "";
    if (a.name === "RSA2048") o.selected = true;
    sel.append(o);
  }
}

// ---- slots -----------------------------------------------------------------
let pivMoveKeySupported = false; // move-key (private-key delete) needs applet 5.7.0+
function setupUnblockToggle() {
  const dlg = document.getElementById("dlg-unblock"); if (!dlg) return;
  const apply = () => {
    const admin = dlg.querySelector('input[name=method]:checked').value === "admin";
    document.getElementById("unblock-puk-row").hidden = admin;
    document.getElementById("unblock-admin-row").hidden = !admin;
    document.getElementById("unblock-hint").textContent = admin ? "Uses the Admin PIN (management key) to reset the PIN, then sets the new PIN." : "Uses the PUK to set a new PIN.";
    dlg.querySelector('input[name=puk]').required = !admin;
    dlg.querySelector('input[name=admin]').required = admin;
  };
  dlg.querySelectorAll('input[name=method]').forEach((r) => r.addEventListener("change", apply));
  apply();
}
async function refreshSlots() {
  slots = await invoke("list_slots");
  renderSlots();
}
function certBlock(c) {
  if (!c) return "";
  const now = Date.now() / 1000;
  const cls = c.not_after < now ? "expired" : c.not_after - now < 30 * 86400 ? "soon" : "";
  const exp = c.not_after < now ? "expired " : "expires ";
  return `<div class="cert">
    <span class="subj">${esc(c.subject.replace(/^CN=/, ""))}</span>
    <span class="hint">${c.self_signed ? "self-signed" : "issued by " + esc(c.issuer.replace(/^CN=/, ""))}</span>
    <span class="exp ${cls}">${exp}${fmtDate(c.not_after)}</span>
  </div>`;
}
function esc(s) { return String(s).replace(/[&<>"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[ch])); }
function buttonsFor(s) {
  const b = [];
  b.push(`<button data-act="generate">Generate key</button>`);
  b.push(`<button data-act="import">Import certificate</button>`);
  b.push(`<button data-act="pfx">Import PFX</button>`);
  if (s.meta) {
    b.push(`<button data-act="pubkey">Public key</button>`);
    if (s.meta.algorithm !== 0xe1) b.push(`<button data-act="selfsign">Self-sign</button>`);
    b.push(`<button data-act="attest">Attest</button>`);
  }
  if (s.cert) {
    b.push(`<button data-act="export">Export</button>`);
    b.push(`<button data-act="delete" class="danger">Delete certificate</button>`);
  }
  return b.join("");
}
async function openPivFpDialog() {
  await loadPivFpCard();
  const card = $("#piv-fp-card");
  if (!card || card.hidden) { toast("This key does not expose fingerprint-for-PIV login (needs a Bio key with the biometric PIV reference).", true); return; }
  card.scrollIntoView({ behavior: "smooth", block: "center" });
  card.classList.add("flash"); setTimeout(() => card.classList.remove("flash"), 1200);
}
async function loadPivFpCard() {
  const card = $("#piv-fp-card"); if (!card) return;
  if (!info) { card.hidden = true; return; }
  let st = { supported: false, enabled: false };
  try { st = await invoke("piv_fp_binding_status"); } catch (_) {}
  card.hidden = !st.supported;               // show only when the key supports it
  const t = $("#piv-fp-toggle");
  if (t) t.checked = !!st.enabled;           // reflect the REAL on/off state
}
function renderSlots() {
  const prim = $("#primary-slots");
  prim.innerHTML = "";
  for (const s of slots.filter((x) => x.primary)) {
    const algo = s.meta ? s.meta.algorithm_name : (s.cert ? s.cert.key_algorithm : "");
    const el = document.createElement("article");
    el.className = "tile " + family(algo);
    el.dataset.slot = s.slot;
    el.innerHTML = `
      <header><b>${s.name}</b><code>${s.id}</code></header>
      ${s.meta || s.cert
        ? `<div class="algo">${esc(algo)}</div><div class="policies">${s.meta ? esc(policies(s.meta)) : "certificate only"}</div>`
        : `<div class="algo empty">No key</div>`}
      ${certBlock(s.cert)}
      <div class="buttons">${buttonsFor(s)}</div>`;
    prim.append(el);
  }
  const ret = $("#retired-slots");
  ret.innerHTML = "";
  const used = slots.filter((x) => !x.primary && (x.meta || x.cert)).length;
  $("#retired-count").textContent = used ? `(${used} in use)` : "(empty)";
  for (const s of slots.filter((x) => !x.primary)) {
    const el = document.createElement("div");
    el.className = "slotrow";
    el.dataset.slot = s.slot;
    const algo = s.meta ? s.meta.algorithm_name : (s.cert ? s.cert.key_algorithm : "");
    el.innerHTML = `
      <code>${s.id}</code>
      <span class="${algo ? "" : "empty"}">${algo || "empty"}</span>
      <span class="hint">${s.cert ? esc(s.cert.subject.replace(/^CN=/, "")) + " · " + fmtDate(s.cert.not_after) : ""}</span>
      <div class="buttons">${buttonsFor(s)}</div>`;
    ret.append(el);
  }
}

// slot actions
$("#board").addEventListener("click", async (e) => {
  const btn = e.target.closest("button[data-act]");
  if (!btn) return;
  const slot = Number(btn.closest("[data-slot]").dataset.slot);
  const s = slots.find((x) => x.slot === slot);
  const label = `${s.id} (${s.name})`;
  switch (btn.dataset.act) {
    case "generate":
      openDialog("dlg-generate", { slot, label });
      break;
    case "selfsign":
      openDialog("dlg-selfsign", { slot, label });
      break;
    case "pfx":
      openDialog("dlg-pfx", { slot, label });
      { const f = $("#dlg-pfx form"); f.file.value = ""; f.file.dataset.path = ""; $("#pfx-status").textContent = ""; $("#pfx-import").disabled = true; }
      break;
    case "pubkey":
      await busy(async () => showText(`Public key in slot ${label}`, await invoke("public_key_pem", { slot }), `slot-${s.id}-public.pem`));
      break;
    case "attest":
      await busy(async () => showAttestation(label, s.id, await invoke("verify_attestation", { slot })));
      break;
    case "import": {
      const r = await confirmDialog(`Import certificate into slot ${label}`, "Choose a PEM or DER certificate file.", { pin: true, okLabel: "Choose file…" });
      if (!r.ok) return;
      await busy(async () => {
        const c = await invoke("import_cert", { slot, pin: r.pin });
        if (c) { toast("Certificate imported"); await refreshSlots(); }
      });
      break;
    }
    case "export":
      await busy(async () => { if (await invoke("export_cert", { slot })) toast("Certificate exported"); });
      break;
    case "delete": {
      openDialog("dlg-piv-delete");
      $("#del-slot-label").textContent = label;
      $("#dlg-piv-delete form").cert.checked = true;
      $("#dlg-piv-delete form").key.checked = false;
      const keyRow = $("#del-key-row"); if (keyRow) keyRow.hidden = false; // Token2 vendor delete (PIN-only)
      break;
    }
  }
});

// ---- factory defaults ------------------------------------------------------
const DEFAULTS = {
  pin: [["865362", "default PIN"], ["88653622", "Octo default PIN"]],
  puk: [["86536286", "default PUK"]],
  mgm: [["865362865362865362865362865362865362865362865362", "default Admin PIN"]],
};
// Management key (Admin PIN) input helper: accept ASCII or hex, return 48 hex
// chars (24 bytes). If the user types hex (exactly 48 hex digits), use it as-is.
// Otherwise treat the input as ASCII, hex-encode it, and zero-pad to 24 bytes.
// Returns { hex, warn } where warn is set when the ASCII was shorter than 24.
function toMgmHex(input) {
  const v = (input || "").trim();
  if (!/^[0-9a-fA-F]*$/.test(v)) return { hex: "", warn: "Admin PIN may only contain hex digits 0-9 and a-f." };
  if (v.length % 2) return { hex: "", warn: "Admin PIN must have an even number of hex digits." };
  if (v.length > 48) return { hex: "", warn: "Admin PIN is too long (max 48 hex digits / 24 bytes)." };
  let warn = "";
  if (v.length && v.length < 48) warn = `Admin PIN is ${v.length} digits; padded with zeros to 48.`;
  return { hex: (v + "0".repeat(48 - v.length)).toLowerCase(), warn };
}
const FIELD_KIND = { current: null, pin: "pin", pin_verify: "pin", new_pin: null, puk: "puk", mgm_key: "mgm", password: null };
function addDefaultButtons(form) {
  if (/^(fido_|oath_|t2otp_)/.test(form.dataset.cmd || "")) return;
  for (const inp of $$("input", form)) {
    if (inp.type === "number" || inp.type === "checkbox") continue;
    let kind = FIELD_KIND[inp.name];
    if (inp.name === "current") kind = form.dataset.cmd === "change_puk" ? "puk" : "pin";
    if (!kind || inp.parentElement.querySelector(".defaults")) continue;
    const box = document.createElement("span");
    box.className = "defaults";
    for (const [val, label] of DEFAULTS[kind]) {
      const b = document.createElement("button");
      b.type = "button"; b.textContent = label; b.title = val;
      b.addEventListener("click", () => {
        inp.value = val;
        // the factory Admin PIN is hex; switch that field's format select to Hex.
        inp.dispatchEvent(new Event("input", { bubbles: true })); inp.focus();
      });
      box.append(b);
    }
    inp.after(box);
  }
}
$$("dialog form").forEach(addDefaultButtons);
$("#confirm-mgm-wrap").append(Object.assign(document.createElement("span"), { className: "defaults" }));
{
  const box = Object.assign(document.createElement("span"), { className: "defaults" });
  for (const [val, label] of DEFAULTS.pin) {
    const b = document.createElement("button"); b.type = "button"; b.textContent = label;
    b.addEventListener("click", () => { $("#confirm-pin").value = val; });
    box.append(b);
  }
  $("#confirm-pin-wrap").append(box);
}
{
  const b = document.createElement("button");
  b.type = "button"; b.textContent = "default Admin PIN";
  b.addEventListener("click", () => { $("#confirm-mgm").value = DEFAULTS.mgm[0][0]; });
  $("#confirm-mgm-wrap .defaults").append(b);
}

// ---- dialogs ---------------------------------------------------------------
let dialogCtx = {};
function openDialog(id, ctx = {}) {
  dialogCtx = ctx;
  const dlg = $("#" + id);
  const form = $("form", dlg);
  form.reset();
  $$(".err", form).forEach((x) => x.remove());
  $$("[data-slot-label]", dlg).forEach((x) => (x.textContent = ctx.label || ""));
  dlg.showModal();
}
$$("[data-dialog]").forEach((b) => b.addEventListener("click", () => openDialog(b.dataset.dialog)));

// repeat-field matching
document.addEventListener("input", (e) => {
  const inp = e.target;
  if (!inp.dataset?.match) return;
  const other = inp.form.elements[inp.dataset.match];
  inp.setCustomValidity(inp.value === other.value ? "" : "Does not match");
});
$("#dlg-generate form").elements.selfsign.addEventListener("change", (e) => {
  $("#selfsign-fields").hidden = !e.target.checked;
});

$$("dialog form[data-cmd]").forEach((form) => {
  if (form.dataset.cmd.startsWith("fido_")) return;
  form.addEventListener("submit", async (e) => {
    const submitter = e.submitter;
    if (!submitter || submitter.value !== "ok") return; // cancel
    e.preventDefault();
    const f = Object.fromEntries(new FormData(form).entries());
    const cmd = form.dataset.cmd;
    try {
      await busy(async () => {
        switch (cmd) {
          case "change_pin":
            await invoke("change_pin", { current: f.current, new: f.new });
            toast("PIN changed"); break;
          case "change_puk":
            await invoke("change_puk", { current: f.current, new: f.new });
            toast("PUK changed"); break;
          case "unblock_pin": {
            const method = form.method ? form.method.value : "puk";
            if (method === "admin") {
              if (!f.admin) return toast("Enter the Admin PIN", true);
              const ah = toMgmHex(f.admin);
              if (!ah.hex) return toast(ah.warn || "Invalid Admin PIN", true);
              if (ah.warn) toast(ah.warn);
              await invoke("unblock_pin_with_admin", { adminHex: ah.hex, newPin: f.new_pin, pinRetries: 3, pukRetries: 3 });
              form.closest("dialog").close("ok"); toast("PIN unblocked with the Admin PIN"); await refreshInfo(); break;
            }
            if (!f.puk) return toast("Enter the PUK", true);
            await invoke("unblock_pin", { puk: f.puk, newPin: f.new_pin });
            toast("PIN unblocked"); break;
          }
          case "set_mgm_key": {
            const cur = toMgmHex(f.mgm_key), nk = toMgmHex(f.new_key);
            if (!cur.hex) return toast(cur.warn || "Invalid current Admin PIN", true);
            if (!nk.hex) return toast(nk.warn || "Invalid new Admin PIN", true);
            if (nk.warn) toast(nk.warn);
            await invoke("authenticate", { mgmKey: cur.hex });
            await invoke("set_mgm_key", { newKey: nk.hex, algorithm: f.algorithm, touch: !!f.touch });
            toast("Admin PIN set"); break;
          }
          case "set_retries": {
            const mk = toMgmHex(f.mgm_key);
            if (!mk.hex) return toast(mk.warn || "Invalid Admin PIN", true);
            if (mk.warn) toast(mk.warn);
            await invoke("authenticate", { mgmKey: mk.hex });
            await invoke("verify_pin", { pin: f.pin_verify });
            await invoke("set_retries", { pin: Number(f.pin_tries), puk: Number(f.puk_tries) });
            toast("Retry limits set; PIN and PUK are back to defaults"); break;
          }
          case "generate_key": {
            const opt = $("#gen-algorithm").selectedOptions[0];
            form.closest("dialog").close("ok");
            progress.show(`Generating ${opt ? opt.textContent : f.algorithm} key in slot ${dialogCtx.label}…${ALGO_HINT(f.algorithm)}`);
            let g;
            try {
              g = await invoke("generate_key", {
                slot: dialogCtx.slot, algorithm: f.algorithm, pinPolicy: f.pin_policy, touchPolicy: f.touch_policy, pin: f.pin,
              });
              if (f.selfsign && opt && opt.dataset.canSign) {
                progress.show("Creating self-signed certificate (signing on the device)…");
                await invoke("self_sign", { slot: dialogCtx.slot, commonName: f.cn, days: Number(f.days), pin: f.pin });
              }
            } finally { progress.hide(); }
            if (f.selfsign && opt && opt.dataset.canSign) {
              toast(`${g.algorithm} key and certificate created`);
            } else {
              toast(`${g.algorithm} key generated`);
              showText("Public key", g.public_key_pem, `slot-${dialogCtx.slot.toString(16)}-public.pem`);
            }
            break;
          }
          case "delete_slot": {
            const cert = !!f.cert, key = !!f.key;
            if (!cert && !key) return toast("Choose what to remove", true);
            const label = $("#del-slot-label").textContent;
            const slot = parseInt(label, 16) || dialogCtx.slot;
            form.closest("dialog").close("ok");
            await busy(async () => { await invoke("delete_slot", { slot, pin: f.pin, cert, key }); });
            const what = [cert && "certificate", key && "private key"].filter(Boolean).join(" and ");
            toast(`Deleted the ${what} from slot ${label}`);
            await refreshSlots();
            break;
          }
          case "import_pfx": {
            const path = form.file.dataset.path;
            if (!path) return toast("Choose a file first", true);
            form.closest("dialog").close("ok");
            progress.show("Importing key into the device…");
            let r;
            try {
              r = await invoke("import_pfx", { slot: dialogCtx.slot, path, pin: f.pin, password: f.password || "", pinPolicy: f.pin_policy, touchPolicy: f.touch_policy });
            } finally { progress.hide(); }
            if (r) toast(`${r.algorithm} key${r.cert_written ? " and certificate" : ""} imported from ${r.file}`);
            await refreshSlots();
            break;
          }
          case "self_sign": {
            form.closest("dialog").close("ok");
            progress.show(fpUse ? "Touch the fingerprint sensor to authorise signing…" : "Creating self-signed certificate (signing on the device)…");
            try {
              await invoke("self_sign", { slot: dialogCtx.slot, commonName: f.common_name, days: Number(f.days), pin: f.pin });
            } finally { progress.hide(); }
            toast("Certificate created"); break;
          }
        }
      });
      if (form.closest("dialog").open) form.closest("dialog").close("ok");
      await refreshInfo();
      await refreshSlots();
    } catch (err) {
      if (!form.closest("dialog").open) { toast(String(err), true); refreshInfo().catch(() => {}); refreshSlots().catch(() => {}); return; }
      let p = $(".err", form);
      if (!p) { p = document.createElement("p"); p.className = "err"; form.insertBefore(p, $("menu", form)); }
      p.textContent = String(err);
      refreshInfo().catch(() => {});
    }
  });
});

// ---- key-level buttons -----------------------------------------------------
$("#btn-refresh").addEventListener("click", loadReaders);
$("#btn-connect").addEventListener("click", connect);
$("#btn-attest-chain").addEventListener("click", () =>
  busy(async () => showText("Attestation certificate (slot f9)", await invoke("attestation_chain"), "attestation-f9.pem")));
$("#btn-reset").addEventListener("click", async () => {
  const r = await confirmDialog("Reset the PIV applet?", "All keys and certificates in the PIV applet are erased and PIN, PUK and Admin PIN return to factory defaults. Other applets (FIDO2, OATH, OpenPGP) are not affected. The reset only works once the PIN and PUK are both blocked, which is done automatically.", { okLabel: "Erase everything" });
  if (!r.ok) return;
  await busy(async () => {
    progress.show("Blocking PIN and PUK, then resetting the PIV applet…");
    try {
    // A PIV reset requires PIN and PUK to be blocked first.
    for (let i = 0; i < 20; i++) { try { await invoke("verify_pin", { pin: "00000000" }); } catch (_) {} if ((await invoke("get_info")).pin_retries === 0) break; }
    for (let i = 0; i < 20; i++) { try { await invoke("unblock_pin", { puk: "00000000", newPin: "00000000" }); } catch (e) { if (/locked|blocked/i.test(String(e))) break; } }
    await invoke("reset");
    } finally { progress.hide(); }
    toast("PIV applet reset");
    await refreshInfo();
    await refreshSlots();
  });
});

loadReaders();

// =============================== FIDO2 mode ==================================
let mode = "fido";
let caps = { piv: true, oath: true, token2_otp: true, fido: true, openpgp: true }; let capsProbed = false;
let t2Only = true; // show only Token2 devices by default
let isElevated = false;
let envOs = "", envPrivileged = false;
invoke("fido_environment").then((e) => { isElevated = !!e.elevated; envOs = e.os || ""; envPrivileged = !!e.privileged; }).catch(() => {});
function isToken2(d) { return /token2/i.test((d.product || "") + (d.manufacturer || "")) || d.vendor_id === 0x349e; }
let oathInfo = null;
let t2Info = null;   // Token2 OTP applet session
let fidoInfo = null;
let fidoPin = "";   // session-only; "" = fingerprint (UV) for passkey operations
let fidoBioPin = ""; // PIN for fingerprint management (libfido2 bio API needs a PIN)
let fidoDevices = [];

function renderIdent() {
  const sec = $("#key-ident"); if (!sec) return;
  const detected = capsProbed && (caps.piv || caps.fido || caps.token2_otp || caps.oath || !!info || !!fidoInfo || !!t2Info || !!oathInfo);
  if (!detected) { sec.hidden = true; return; }
  sec.hidden = false;

  // fresh reads from whatever applet is connected
  let usbNameFresh = "";
  if (fidoInfo) { const d = fidoDevices.find((x) => x.path === fidoInfo.path); usbNameFresh = (d && (d.product || d.manufacturer)) || ""; }
  usbNameFresh = usbNameFresh || _keyName || "";
  const modelFresh = (info && info.model && info.model.model) || caps.model || "";
  const revFresh   = caps.revision || (info && info.model && info.model.revision) || (t2Info && t2Info.model && t2Info.model.revision) || "";
  const serialFresh= caps.serial || (info && (info.serial_full || info.serial)) || (t2Info && t2Info.serial) || "";

  // The cache exists only to survive a transient empty read on the SAME key. If the
  // key identity changed (different serial or product name), wipe the cache so a new
  // key never inherits the previous key's illustration / fields.
  const idNow = (serialFresh || "") + "|" + (usbNameFresh || "") + "|" + (modelFresh || "");
  const idWas = (_identCache.serial || "") + "|" + (_identCache.usbName || "") + "|" + (_identCache.modelName || "");
  if (serialFresh && idNow !== idWas && (_identCache.serial || _identCache.usbName)) {
    // a different key: reset the cache to the fresh values only
    _identCache = { usbName: "", modelName: "", revision: "", serial: "", appsStr: "" }; _noteCache = {}; { const nr = $("#di-note-row"); if (nr) nr.hidden = true; }
  }

  // now resolve each field, falling back to cache only to bridge a transient empty read
  const usbName   = usbNameFresh || _identCache.usbName || "";
  const modelName = modelFresh   || _identCache.modelName || "";
  const revision  = revFresh     || _identCache.revision || "";
  const serial    = serialFresh  || _identCache.serial || "";
  const apps = [];
  if (caps.piv) apps.push("PIV");
  if (caps.token2_otp) apps.push("OTP"); else if (caps.oath) apps.push("OATH");
  if (caps.fido) apps.push("FIDO2");
  if (caps.openpgp) apps.push("OpenPGP");
  const appsStr = apps.join(", ") || _identCache.appsStr || "";
  maybeLoadNoteForHome((caps.serial || (t2Info && t2Info.serial) || "")).catch(()=>{});
  _identCache = { usbName: usbName || _identCache.usbName, modelName: modelName || _identCache.modelName,
                  revision: revision || _identCache.revision, serial: serial || _identCache.serial,
                  appsStr: appsStr || _identCache.appsStr };

  const name = prettyKeyName(usbName) || (modelName || "Security key");
  const nm = $("#ident-name"); if (nm) nm.textContent = name;
  const ib = $("#ident-illus"); if (ib) ib.innerHTML = keyIllustration((modelName + " " + usbName).trim(), serial);

  const setField = (id, val) => {
    const dd = $("#" + id); if (!dd) return; const row = dd.closest("div");
    if (val) { dd.textContent = val; if (row) row.hidden = false; } else if (row) row.hidden = true;
  };
  setField("di-model", modelName);
  setField("di-rev", revision);
  setField("di-serial", serial);
  setField("di-apps", appsStr);
  if (envOs === "windows") setField("di-access", isElevated ? "Administrator" : "Standard user");
  else if (envPrivileged) setField("di-access", "root");
  else setField("di-access", "");
}
function resetKeyPanels() {
  // hide only the per-page DETAIL boxes; the persistent identity header stays.
  for (const id of ["home-info", "key-box", "fido-dev-box"]) {
    const el = $("#" + id); if (el) el.hidden = true;
  }
}
function setMode(m) {
  mode = m;
  $$(".modes .mode").forEach((b) => b.classList.toggle("active", b.dataset.mode === m));
  const piv = m === "piv", fido = m === "fido", oath = m === "oath";
  // sidebar (manual connect UI removed; auto-detect drives connections)
  $("#connect-box").hidden = true;
  $("#fido-connect-box").hidden = true;
  resetKeyPanels();                                   // hide detail boxes; ident persists
  renderIdent();
  $("#key-box").hidden = !piv || !info;               // then show only the active applet's box
  $("#fido-dev-box").hidden = !fido || !fidoInfo;
  // board
  $("#empty").hidden = !piv || !!info;
  $("#slots").hidden = !piv || !info;
  if (piv && info) { renderInfo(); loadPivFpCard(); } else { const c=$("#piv-fp-card"); if (c) c.hidden = true; }
  $("#fido-empty").hidden = !fido || !!fidoInfo;
  $("#fido-board").hidden = !fido || !fidoInfo;
  if (fido && fidoInfo) { renderFidoInfo(); loadFidoDevices().then(() => renderFidoInfo()).catch(()=>{}); showFidoSub(); }
  const anyOtp = !!oathInfo || !!t2Info;
  $("#oath-connect-box").hidden = !oath || anyOtp;
  $("#oath-dev-box").hidden = !oath || !oathInfo;
  $("#t2otp-dev-box").hidden = !oath || !t2Info;
  if (oath && t2Info) renderT2Info(); else if (oath && oathInfo) renderOathInfo();
  $("#oath-empty").hidden = !oath || anyOtp;
  $("#oath-board").hidden = !oath || !anyOtp;
  if (page !== "home") $("#home-board").hidden = true;
  if (fido && !fidoInfo) loadFidoDevices();
  if (oath && !oathInfo) loadOathReaders();
  if (Array.isArray(PAGE_IDS) && ["piv", "fido", "oath"].includes(page)) PAGE_IDS.forEach((id) => { const el = document.getElementById(id); if (el) el.hidden = true; });
  applyCaps();
}
$$(".modes .mode").forEach((b) => b.addEventListener("click", () => setMode(b.dataset.mode)));

async function loadFidoDevices() {
  const sel = $("#fido-device");
  sel.innerHTML = "";
  try {
    let all = await invoke("fido_list_devices");
    // Deduplicate a Token2 key that enumerates as BOTH hid and ccid: keep the ccid
    // entry (works non-admin, tunnels FIDO). Key by vendor+product id.
    const seen = new Map();
    for (const d of all) {
      const k = `${d.vendor_id}:${d.product_id}`;
      const prev = seen.get(k);
      if (!prev) { seen.set(k, d); }
      else if (prev.transport === "hid" && d.transport === "ccid") { seen.set(k, d); } // prefer ccid
    }
    fidoDevices = [...seen.values()];
    for (const d of fidoDevices) {
      const o = document.createElement("option");
      o.value = d.path;
      o.textContent = `${d.product || d.manufacturer || "FIDO2 device"} — ${d.transport === "ccid" ? "smart card (USB/NFC)" : "USB HID"}`;
      o.selected = isToken2(d) || (d.transport === "ccid" && !fidoDevices.some((x) => isToken2(x)));
      sel.append(o);
    }
    $("#fido-connect-msg").textContent = fidoDevices.length ? "" : "No FIDO2 devices found. Plug in a key or place it on the reader, then rescan.";
  } catch (e) {
    $("#fido-connect-msg").textContent = String(e);
  }
}

async function fidoConnect() {
  await busy(async () => {
    let path = $("#fido-device").value;
    try { fidoInfo = await invoke("fido_connect", { path }); }
    catch (e) {
      // try the other listed devices before giving up (a busy/again-HID one may free up)
      const others = fidoDevices.map((d) => d.path).filter((p) => p !== path);
      let ok = false;
      for (const p of others) { try { fidoInfo = await invoke("fido_connect", { path: p }); $("#fido-device").value = p; ok = true; break; } catch (_) {} }
      if (!ok) {
        const hid = fidoDevices.find((d) => d.path === path && d.transport !== "ccid");
        if (hid && /elevat|admin|denied|access/i.test(String(e))) {
          throw new Error("This is a USB-HID FIDO key. On Windows it can only be opened by an administrator — right-click the app and 'Run as administrator', or use a key that presents a smart-card (CCID) interface.");
        }
        throw e;
      }
    }
    fidoPin = ""; fidoBioPin = "";
    refreshCaps();
    checkPcscHealth();
    renderFidoInfo();
    $("#fido-passkeys").hidden = true;
    $("#fido-bio").hidden = true;
    $("#fido-unlock").hidden = false;
    $("#fido-pin").value = "";
    if (page === "fido") setMode("fido");
    if (page === "home") renderHome(); renderIdent();
  });
}
async function fidoDisconnect() {
  await invoke("fido_disconnect");
  fidoInfo = null; fidoPin = ""; fidoBioPin = ""; fidoConnect._warned = false;
  setMode("fido");
}
async function refreshFidoInfo() {
  fidoInfo = await invoke("fido_info");
  renderFidoInfo();
}
function pips(el, n, cls) {
  el.innerHTML = "";
  const max = Math.max(n, 3);
  for (let i = 0; i < Math.min(max, 10); i++) el.append(Object.assign(document.createElement("i"), { className: i < n ? "on" : "" }));
  el.className = "pips" + (n === 0 ? " none" : n === 1 ? " low" : "");
}
let renderFidoInfo = function () {
  const d = fidoDevices.find((x) => x.path === fidoInfo.path) || {};
  $("#f-product").textContent = d.product || d.manufacturer || "—";
  $("#f-transport").innerHTML = fidoInfo.transport === "ccid" ? '<span class="badge nfc">Smart card (USB/NFC)</span>' : '<span class="badge">HID</span>';
  $("#f-firmware").textContent = fidoInfo.firmware || "—";
  $("#f-versions").textContent = fidoInfo.versions.join(", ") || (fidoInfo.is_fido2 ? "FIDO2" : "U2F only");
  $("#f-aaguid").textContent = fidoInfo.aaguid || "—";
  $("#f-pin").textContent = fidoInfo.has_pin ? (fidoInfo.new_pin_required ? "set — change required" : "set") : (fidoInfo.supports_pin ? "not set" : "not supported");
  if (fidoInfo.new_pin_required && !fidoConnect._warned) { fidoConnect._warned = true; toast("This key requires a PIN change before other FIDO operations (FIDO → PIN)", true); }
  $("#f-rk").textContent = fidoInfo.rk_remaining >= 0 ? String(fidoInfo.rk_remaining) : "—";
  $("#f-minpin").textContent = fidoInfo.min_pin_len ? String(fidoInfo.min_pin_len) : "—";
  $("#f-alwaysuv").textContent = (fidoInfo.supports_config && fidoInfo.options.some(([k]) => k === "alwaysUv")) ? (fidoInfo.always_uv ? "on" : "off") : (fidoInfo.always_uv ? "on (read-only)" : "not supported");
  pips($("#f-pin-pips"), Math.max(fidoInfo.pin_retries, 0));
  $("#f-pin-retries").textContent = fidoInfo.pin_retries < 0 ? "?" : fidoInfo.pin_retries;
  $("#f-uv-row").hidden = !fidoInfo.supports_bio;
  pips($("#f-uv-pips"), Math.max(fidoInfo.uv_retries, 0));
  $("#f-uv-retries").textContent = fidoInfo.uv_retries < 0 ? "?" : fidoInfo.uv_retries;
  $("#fido-pin-title").textContent = fidoInfo.has_pin ? "Change PIN" : "Set PIN";
  $("#fido-oldpin-wrap").hidden = !fidoInfo.has_pin;
  { // dynamic minimum from the key (not a hardcoded 4), + PIN+ complexity note
    const minLen = fidoInfo.min_pin_len && fidoInfo.min_pin_len > 4 ? fidoInfo.min_pin_len : 4;
    const lbl = $("#fido-pin-len-label"); const inp = lbl && lbl.querySelector("input");
    if (inp) inp.setAttribute("minlength", String(minLen));
    if (lbl) lbl.childNodes[0].nodeValue = `New PIN (${minLen}\u2013${inp ? inp.maxLength : 63} characters) `;
    // Token2 PIN+ series enforce PIN complexity — show the note + link for Token2 keys
    const d = fidoDevices.find((x) => x.path === fidoInfo.path);
    const nm = ((d && (d.product || d.manufacturer)) || caps.model || "").toLowerCase();
    const isT2 = /token2|pin\+|epass/.test(nm) || caps.token2_otp;
    const cx = $("#fido-pin-complexity"); if (cx) cx.hidden = !isT2;
  }
  $("#btn-fido-pin").textContent = fidoInfo.has_pin ? "Change PIN" : "Set PIN";
  $("#btn-fido-alwaysuv").hidden = !fidoInfo.options.some(([k]) => k === "alwaysUv");
  $("#btn-fido-forcepin").hidden = !fidoInfo.has_pin;
  const canUv = fidoInfo.has_uv || fidoInfo.supports_bio || fidoInfo.options.some(([k,v]) => k === "uv" && v);
  $("#btn-fido-unlock-uv").hidden = true; // fingerprint cannot manage passkeys (credMgmt needs the PIN)
  $$(".uv-hint").forEach((h) => (h.hidden = true)); // UV-instead-of-PIN not supported
}

let fidoSub = "passkeys"; // which FIDO sub-page is active
const fidoUnlocked = () => fidoPin !== "" || (fidoInfo && !fidoInfo.has_pin);
function showFidoSub(which) {
  const changedSub = which && which !== fidoSub;
  if (which) fidoSub = which;
  const unlocked = fidoUnlocked();
  if (fidoInfo) { const b=$("#fido-dev-box"); if (b) b.hidden = false; try { renderFidoInfo(); } catch (_) {} }
  // the unlock gate is shown until a session PIN is established
  $("#fido-unlock").hidden = unlocked;
  { const br = $("#fido-unlock-back"); if (br) br.hidden = unlocked; }
  // sub-sections only show once unlocked
  $("#fido-passkeys").hidden = !(unlocked && fidoSub === "passkeys");
  $("#fido-bio").hidden = !(unlocked && fidoSub === "fingerprints");
  // load the pane's content when navigating to it while already unlocked
  if (unlocked && changedSub) {
    if (fidoSub === "fingerprints" && fidoInfo && fidoInfo.supports_bio) loadBio().catch(() => {});
    else if (fidoSub === "passkeys") loadPasskeys().catch(() => {});
  }
}

async function fidoUnlock(useUv = false) {
  const pin = useUv ? "" : $("#fido-pin").value;
  if (!useUv && !pin) return toast("Enter the PIN", true);
  // Passkey management (credMgmt) requires the PIN-derived token when a PIN is set;
  // a fingerprint (UV) token isn't accepted for listing on these keys and would
  // just hang waiting. Only the fingerprint-enrollment page uses UV directly.
  const wantsPasskeys = !(fidoSub === "fingerprints" && fidoInfo.supports_bio);
  if (useUv && wantsPasskeys && fidoInfo.has_pin) {
    return toast("Managing passkeys needs the PIN on this key. Enter the PIN to list passkeys; the fingerprint is used when you sign in.", true);
  }
  if (useUv) progress.show("Touch the fingerprint sensor…");
  try {
    await busy(async () => {
      fidoPin = pin;
      if (fidoSub === "fingerprints" && fidoInfo.supports_bio) await loadBio();
      else await loadPasskeys(pin);
      showFidoSub();
    });
  } finally { progress.hide(); }
}
// wrap fingerprint-authorised calls with a touch prompt
async function uvCall(fn) {
  if (fidoPin === "" && fidoInfo && fidoInfo.has_uv) progress.show("Touch the fingerprint sensor…");
  try { return await fn(); } finally { progress.hide(); }
}
async function loadPasskeys(pin = fidoPin) {
  // credMgmt is the real gate, but some 2.1 keys under-report supports_credman;
  // if the key advertises FIDO_2_1 we try anyway and only show the unsupported
  // note if the attempt genuinely fails as unsupported.
  const likely21 = (fidoInfo.versions || []).some((v) => /2_1/.test(v)) || fidoInfo.options.some(([k,v]) => (k === "credMgmt" || k === "credentialMgmtPreview") && v);
  if (!fidoInfo.supports_credman && !likely21) {
    $("#fido-passkeys").hidden = false;
    $("#fido-rk-count").textContent = "";
    $("#fido-rps").innerHTML = `<p class="hint">This device does not support passkey management (FIDO 2.0 without credMgmt).</p>`;
    return;
  }
  let inv;
  try { inv = await invoke("fido_list_passkeys", { pin }); }
  catch (e) {
    const es = String(e);
    // PIN missing / invalid / token expired -> go back to the unlock gate right away
    if (/pin is required|pin required|pin invalid|0x31|0x33|0x36|not verified|unlock with the pin|needs the pin/i.test(es)) {
      fidoPin = "";
      $("#fido-passkeys").hidden = true; $("#fido-bio").hidden = true;
      $("#fido-unlock").hidden = false;
      { const br = $("#fido-unlock-back"); if (br) br.hidden = false; }
      toast("Enter the PIN to view passkeys", true);
      return;
    }
    // a genuine "not supported" from the key -> show the note; otherwise re-throw
    if (/not supported|credMgmt|unsupported|0x2d|0x2e/i.test(es)) {
      $("#fido-passkeys").hidden = false;
      $("#fido-rk-count").textContent = "";
      $("#fido-rps").innerHTML = `<p class="hint">This device does not support listing passkeys from a manager.</p>`;
      return;
    }
    throw e;
  }
  $("#fido-passkeys").hidden = false;
  $("#fido-rk-count").textContent = `${inv.existing} stored · ${inv.remaining} free`;
  const box = $("#fido-rps");
  box.innerHTML = "";
  if (!inv.rps.length) box.innerHTML = `<p class="hint">No passkeys on this key.</p>`;
  const noneReadable = inv.rps.length && inv.rps.every((r) => !r.creds_readable);
  if (noneReadable) {
    const banner = document.createElement("p");
    banner.className = "hint";
    banner.textContent = `${inv.existing} passkey(s) stored across ${inv.rps.length} site(s). This key's firmware does not support listing or deleting individual passkeys from a manager (per-credential enumeration is refused). The relying parties are shown below; remove passkeys from each website, or use FIDO → Reset to clear them all.`;
    box.prepend(banner);
  }
  for (const rp of inv.rps) {
    const el = document.createElement("article");
    el.className = "rp";
    const rows = rp.credentials.map((c) => `<div class="cred" data-cred="${esc(c.cred_id)}" data-uid="${esc(c.user_id)}" data-uname="${esc(c.user_name)}" data-dname="${esc(c.display_name)}">
        <div class="who"><span>${esc(c.display_name || c.user_name || "(no user info)")}</span><small>${esc(c.user_name && c.user_name !== c.display_name ? c.user_name + " · " : "")}${esc(c.key_type)}${c.cred_id ? " · " + esc(c.cred_id.slice(0, 16)) + "…" : ""}</small></div>
        <span class="prot">${["", "UV optional", "UV optional w/ list", "UV required"][c.protection] || ""}</span>
        <span class="buttons"><button data-act="edit-cred">Edit</button><button data-act="del-cred" class="danger">Delete</button></span>
      </div>`).join("");
    const noteRow = `<div class="cred"><div class="who"><small>This key does not allow listing individual passkeys for this site (firmware limitation). The site is registered; manage or remove the passkey from the website, or reset the FIDO applet to clear all.</small></div></div>`;
    el.innerHTML = `<header><b>${esc(rp.name || rp.id)}</b><code>${esc(rp.id)}</code></header>` +
      (rp.creds_readable ? (rows || `<div class="cred"><div class="who"><small>(no credentials)</small></div></div>`) : noteRow);
    box.append(el);
  }
}
async function loadBio() {
  const pin = fidoPin || fidoBioPin;
  $("#fido-bio").hidden = false;
  if (fidoInfo && !fidoInfo.has_pin) {
    $("#fido-bio-unlock").hidden = true; $("#btn-fido-enroll").hidden = true; $("#fido-bio-count").textContent = "";
    $("#fido-templates").innerHTML = `<p class="hint">Set a FIDO PIN first (FIDO → PIN). Fingerprints can be enrolled once the key has a PIN.</p>`;
    return;
  }
  if (!pin) { $("#fido-bio-unlock").hidden = false; $("#fido-templates").innerHTML = ""; $("#fido-bio-count").textContent = ""; $("#btn-fido-enroll").hidden = true; return; }
  const b = await invoke("fido_list_bio", { pin });
  $("#fido-bio-unlock").hidden = true; $("#btn-fido-enroll").hidden = false;
  $("#fido-bio-count").textContent = `${b.templates.length} enrolled`;
  const box = $("#fido-templates");
  box.innerHTML = "";
  for (const t of b.templates) {
    const el = document.createElement("div");
    el.className = "slotrow";
    el.dataset.tid = t.id;
    el.innerHTML = `<span>${esc(t.name || "(unnamed)")}</span><div class="buttons"><button data-act="rename-bio">Rename</button><button data-act="del-bio" class="danger">Delete</button></div>`;
    box.append(el);
  }
}

$("#fido-board").addEventListener("click", async (e) => {
  const btn = e.target.closest("button[data-act]");
  if (!btn) return;
  const act = btn.dataset.act;
  if (act === "del-cred") {
    const id = btn.closest("[data-cred]").dataset.cred;
    const r = await confirmDialog("Delete this passkey?", "The site will no longer recognise this key for that account. This cannot be undone.", { okLabel: "Delete passkey" });
    if (!r.ok) return;
    await uvCall(() => busy(async () => { await invoke("fido_delete_passkey", { credId: id, pin: fidoPin }); toast("Passkey deleted"); await loadPasskeys(); await refreshFidoInfo(); }));
  } else if (act === "edit-cred") {
    const el = btn.closest("[data-cred]");
    openDialog("dlg-fido-passkey", { cred: el.dataset.cred, uid: el.dataset.uid });
    $("#dlg-fido-passkey input[name=user_name]").value = el.dataset.uname;
    $("#dlg-fido-passkey input[name=display_name]").value = el.dataset.dname;
  } else if (act === "del-bio") {
    const id = btn.closest("[data-tid]").dataset.tid;
    const r = await confirmDialog("Delete this fingerprint?", "", { okLabel: "Delete" });
    if (!r.ok) return;
    await uvCall(() => busy(async () => { await invoke("fido_delete_bio", { id, pin: fidoPin }); toast("Fingerprint deleted"); await loadBio(); }));
  } else if (act === "rename-bio") {
    openDialog("dlg-fido-rename", { tid: btn.closest("[data-tid]").dataset.tid });
    $("#dlg-fido-rename input[name=pin]").value = fidoPin || fidoBioPin;
  }
});

$("#btn-fido-refresh").addEventListener("click", loadFidoDevices);
$("#btn-passkeys-reload") && $("#btn-passkeys-reload").addEventListener("click", () => busy(async () => {
  // reconnect so a passkey added elsewhere since we opened is seen (metadata is cached per session)
  try { if (fidoInfo && fidoInfo.path) { await invoke("fido_disconnect"); fidoInfo = await invoke("fido_connect", { path: fidoInfo.path }); } } catch (_) {}
  await loadPasskeys();
}).catch((e) => toast(String(e), true)));
$("#btn-bio-reload") && $("#btn-bio-reload").addEventListener("click", () => busy(() => loadBio()).catch((e) => toast(String(e), true)));
$("#btn-oath-reload") && $("#btn-oath-reload").addEventListener("click", () => busy(() => (t2Info ? loadT2() : loadOath())).catch((e) => toast(String(e), true)));

$("#btn-fido-diag").addEventListener("click", async () => {
  await busy(async () => {
    const probes = await invoke("fido_diagnose");
    const lines = probes.map((p) => `slot${p.index}  ${p.reader}\n    ${p.open_ok ? "OK — FIDO applet answers" + (p.versions.length ? " (" + p.versions.join(", ") + ")" : "") : "FAILED: " + p.error}`);
    showText("PC/SC reader probe (what libfido2 sees)", lines.join("\n\n") + "\n\nlibfido2 debug log: start the app from a console with T2_FIDO_DEBUG=1 set, or run token2-fido2-token.exe -L -d", "fido-probe.txt");
  });
});
$("#btn-fido-connect").addEventListener("click", fidoConnect);
$("#btn-fido-detect") && $("#btn-fido-detect").addEventListener("click", rescanAll);
$("#btn-fido-unlock").addEventListener("click", () => fidoUnlock(false));
$("#btn-fido-unlock-uv").addEventListener("click", () => fidoUnlock(true));
$("#fido-pin").addEventListener("keydown", (e) => { if (e.key === "Enter") fidoUnlock(); });
$("#btn-fido-enroll").addEventListener("click", () => { openDialog("dlg-fido-enroll"); $("#dlg-fido-enroll input[name=pin]").value = fidoPin || fidoBioPin; });
$("#btn-fido-bio-unlock").addEventListener("click", async () => {
  const pin = $("#fido-bio-pin").value;
  if (!pin) return toast("Enter the PIN", true);
  await busy(async () => { fidoBioPin = pin; try { await loadBio(); } catch (e) { fidoBioPin = ""; throw e; } $("#fido-bio-pin").value = ""; });
});
$("#btn-fido-forcepin").addEventListener("click", async () => {
  if (!fidoInfo || !fidoInfo.supports_config) return toast("This key does not support forcing a PIN change", true);
  const r = await confirmDialog("Force PIN change?", "The user will have to choose a new PIN the next time the key is used." + (fidoInfo.has_uv ? " Leave the PIN empty to confirm with a fingerprint." : ""), { pin: true, okLabel: "Force change" });
  if (!r.ok) return;
  await uvCall(() => busy(async () => { await invoke("fido_force_pin_change", { pin: r.pin }); toast("PIN change will be required at next use"); await refreshFidoInfo(); }));
});
$("#btn-fido-alwaysuv").addEventListener("click", async () => {
  if (!fidoInfo || !fidoInfo.supports_config || !fidoInfo.options.some(([k]) => k === "alwaysUv")) return toast("This key does not support changing always-UV", true);
  const r = await confirmDialog("Toggle always-UV?", `Currently ${fidoInfo.always_uv ? "on" : "off"}. When on, every use requires the PIN or a fingerprint.`, { pin: true, okLabel: "Toggle" });
  if (!r.ok) return;
  await uvCall(() => busy(async () => { await invoke("fido_toggle_always_uv", { pin: r.pin }); toast("Always-UV toggled"); await refreshFidoInfo(); }));
});
$("#btn-fido-reset").addEventListener("click", () => guidedFidoReset());

// Guided factory reset: the authenticator only accepts a reset shortly after
// power-up, so we walk the user through unplug -> replug -> immediate reset. On
// replug we RE-ENUMERATE to get a fresh device path (its handle/hidraw address
// changes on replug, especially on macOS) rather than reusing the stale one.
let _resetBusy = false;
async function guidedFidoReset() {
  if (_resetBusy) return;
  _resetBusy = true;
  const dlg = $("#dlg-fido-reset");
  const step = (n) => { for (let i = 1; i <= 4; i++) { const li = $("#rst-" + i); if (li) { li.classList.toggle("active", i === n); li.classList.toggle("done", i < n); } } };
  const msg = (t, bad) => { const m = $("#reset-msg"); if (m) { m.textContent = t || ""; m.classList.toggle("bad", !!bad); } };

  let cancelled = false;
  const cancel = () => { cancelled = true; };
  $("#btn-reset-cancel").onclick = cancel;
  dlg.oncancel = (e) => { e.preventDefault(); cancel(); };   // Esc -> cancel the flow, not just close
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const guard = () => { if (cancelled) throw new Error("__cancelled__"); };

  const vid = 0x349e;
  const isT2 = (d) => (d.vendor_id === vid) || /token2|epass/i.test((d.product || "") + " " + (d.manufacturer || ""));
  // Count how many keys are physically present. Use the lightweight presence()
  // probe (the same signal the main poller uses for plug/unplug on USB): readers
  // = PC/SC smart-card channels, fido_devices = HID+CCID FIDO interfaces. The
  // total moves down on unplug and up on replug regardless of the exact path.
  const presentCount = async () => {
    let n = 0;
    try { const p = await invoke("presence"); n += (p.readers ? p.readers.length : 0) + (p.fido_devices || 0); } catch (_) {}
    // add physically-present cards so NFC tap-off/tap-on is detected even though
    // the reader itself stays listed (USB is already covered by presence()).
    try { n += await invoke("cards_present"); } catch (_) {}
    return n;
  };
  const listT2 = async () => { try { return (await invoke("fido_list_devices")).filter(isT2); } catch (_) { return []; } };

  step(1); msg(""); dlg.showModal();

  try {
    _opInFlight++;   // pause the background poller for the whole flow
    try { if (fidoInfo) await invoke("fido_disconnect"); } catch (_) {}
    fidoInfo = null;
    await sleep(400);

    const startCount = await presentCount();

    // 1) wait for UNPLUG: the present-device count drops below the start count
    //    (and settles). We require a real decrease, so step 1 can't go green
    //    while the key is still inserted.
    step(1);
    let removed = false;
    for (let i = 0; i < 300 && !removed; i++) {
      guard(); await sleep(400);
      if ((await presentCount()) < startCount) {
        // confirm it stays down for two reads (debounce enumeration jitter)
        await sleep(300); guard();
        if ((await presentCount()) < startCount) removed = true;
      }
    }
    guard();
    if (!removed) throw new Error("Didn't detect the key being removed — unplug it and try again.");
    const lowCount = await presentCount();   // level while unplugged (usually 0)

    // 2) wait for REPLUG: the present-device count rises above the unplugged level.
    step(2); msg("");
    let p = null;
    for (let i = 0; i < 400; i++) {
      guard(); await sleep(350);
      const c = await presentCount();
      if (c > lowCount) {
        await sleep(300); guard();
        if ((await presentCount()) > lowCount) { p = await invoke("presence").catch(() => null); if (p) break; }
      }
    }
    guard();
    if (!p) throw new Error("Didn't detect the key being plugged back in — try again.");

    // 3) connect using the SAME path the auto-poller uses (works for USB CCID/HID),
    //    then reset immediately — all within the key's power-up window.
    step(3); msg("Touch the key when it blinks…");
    // small settle so the reader/interface finishes enumerating
    await sleep(500); guard();
    await connectFido(p);                 // sets fidoInfo via fido_connect_ccid / HID
    guard();
    if (!fidoInfo) throw new Error("Key detected but the FIDO applet could not be opened — try again.");
    await invoke("fido_reset");

    // 4) done — reconnect so the fresh post-reset state is read (the cached session
    //    still reports the old PIN/bio state, which would wrongly show "Change PIN").
    step(4); msg("");
    fidoPin = ""; fidoBioPin = "";
    try {
      const path = fidoInfo && fidoInfo.path;
      try { await invoke("fido_disconnect"); } catch (_) {}
      fidoInfo = path ? await invoke("fido_connect", { path }) : null;
    } catch (_) { try { await refreshFidoInfo(); } catch (_) {} }
    try { renderFidoInfo(); } catch (_) {}
    await sleep(500);
    dlg.close();
    toast("FIDO2 applet reset");
    // return to FIDO Settings so the (now empty) PIN state shows correctly
    showPage("fido-settings");
    renderFidoSettings();
  } catch (e) {
    if (String(e).includes("__cancelled__")) { try { dlg.close(); } catch (_) {} }
    else { msg(String(e).replace(/^Error:\s*/, ""), true); step(2); }
  } finally {
    dlg.oncancel = null;
    _opInFlight = Math.max(0, _opInFlight - 1);
    _resetBusy = false;
    // the flow disconnected FIDO; refresh the left pane so no stale detail panel
    // ("More info" of the just-reset/again-absent key) lingers.
    try { resetKeyPanels(); } catch (_) {}
    try { if (!fidoInfo && page === "fido") showFidoSub(); } catch (_) {}
    try { renderIdent(); } catch (_) {}
  }
}

// FIDO dialog submissions share the generic form handler; add their cases
document.addEventListener("submit", async (e) => {
  const form = e.target;
  const cmd = form.dataset?.cmd || "";
  if (!(cmd.startsWith("fido_") || cmd.startsWith("oath_") || cmd.startsWith("t2otp_")) || !e.submitter || e.submitter.value !== "ok") return;
  e.preventDefault();
  const f = Object.fromEntries(new FormData(form).entries());
  try {
    await busy(async () => {
      switch (cmd) {
        case "fido_set_pin":
          await invoke("fido_set_pin", { newPin: f.new_pin, oldPin: f.old_pin || null });
          fidoPin = f.new_pin; toast("PIN saved"); break;
        case "fido_set_min_pin_len":
          await uvCall(() => invoke("fido_set_min_pin_len", { len: Number(f.len), pin: f.pin || "" }));
          toast("Minimum PIN length set"); break;
        case "fido_rename_bio":
          await uvCall(() => invoke("fido_rename_bio", { id: dialogCtx.tid, name: f.name, pin: f.pin || "" }));
          toast("Fingerprint renamed"); await loadBio(); break;
        case "fido_update_passkey":
          await uvCall(() => invoke("fido_update_passkey", { credId: dialogCtx.cred, userId: dialogCtx.uid, userName: f.user_name, displayName: f.display_name, pin: fidoPin }));
          toast("Passkey updated"); await loadPasskeys(); break;
        case "fido_enroll":
          form.closest("dialog").close("ok");
          await runEnrollment(f.name || "", f.pin || "");
          return;
        case "oath_put":
          await invoke("oath_put", { issuer: f.issuer.trim(), account: f.account.trim(), secretBase32: f.secret, kind: f.kind, algorithm: f.algorithm,
            digits: Number(f.digits), period: Number(f.period) || 30, counter: Number(f.counter) || 0, touch: !!f.touch });
          toast("Account added"); await loadOath(); break;
        case "oath_set_password":
          oathInfo = await invoke("oath_set_password", { password: f.password });
          toast("Password set"); renderOathInfo(); break;
        case "t2otp_write":
          await invoke("t2otp_write", { app: f.app.trim(), account: f.account.trim(), secretBase32: f.secret, kind: f.kind, algorithm: f.algorithm,
            digits: Number(f.digits) || 6, timestep: Number(f.timestep) || 30, button: !!f.button });
          toast("Account added"); await loadT2(); break;
        case "t2otp_pin": {
          if (t2Info.pin.set) t2Info.pin = await invoke("t2otp_change_pin", { current: f.current, newPin: f.pin });
          else t2Info.pin = await invoke("t2otp_set_pin", { pin: f.pin });
          toast(t2Info.pin.set ? "OTP PIN saved" : "OTP PIN removed"); renderT2Info();
          if (t2Info.pin.set && !t2Info.pin.verified) { $("#oath-unlock").hidden = false; $("#oath-accounts").hidden = true; }
          break;
        }
        case "t2otp_set_keyboard_interface": {
          const on = !!f.keyboard;
          const r = await confirmDialog(on ? "Enable the keyboard interface?" : "Disable the keyboard interface?", "The key will re-enumerate on USB. Reconnect it in this app afterwards.", { okLabel: "Apply" });
          if (!r.ok) return;
          await invoke("t2otp_set_keyboard_interface", { keyboardEnabled: on });
          toast(on ? "Keyboard interface enabled — reconnect the key" : "Keyboard interface disabled — reconnect the key");
          form.closest("dialog").close("ok");
          if (info) await disconnect().catch(() => {});
          if (fidoInfo) await fidoDisconnect().catch(() => {});
          await oathDisconnect().catch(() => {});
          return;
        }
        case "t2otp_set_button_hotp":
          if (f.secret.trim()) await invoke("t2otp_set_button_hotp", { secretBase32: f.secret, digits: Number(f.digits), noEnter: !!f.no_enter, longPress: !!f.long_press, numpad: !!f.numpad });
          else await invoke("t2otp_set_button_options", { noEnter: !!f.no_enter, longPress: !!f.long_press, numpad: !!f.numpad });
          toast("Button HOTP saved"); await refreshT2Info(); break;
      }
    });
    form.closest("dialog").close("ok");
    if (cmd.startsWith("fido_")) await refreshFidoInfo();
  } catch (err) {
    let p = $(".err", form);
    if (!p) { p = document.createElement("p"); p.className = "err"; form.insertBefore(p, $("menu", form)); }
    p.textContent = String(err);
  }
}, true);

async function runEnrollment(name, pin) {
  const dlg = $("#dlg-enroll-progress");
  let cancelled = false;
  dlg.onclose = () => { if (dlg.returnValue === "cancel") { cancelled = true; invoke("fido_enroll_cancel").catch(() => {}); } };
  const show = (step, total) => {
    $("#enroll-text").textContent = step.done ? "Fingerprint enrolled." : `Touch the sensor — ${step.remaining} more sample${step.remaining === 1 ? "" : "s"}`;
    $("#enroll-status").textContent = step.status_text;
    $("#enroll-bar").style.width = total ? `${Math.round(100 * (total - step.remaining) / total)}%` : "0%";
  };
  $("#enroll-text").textContent = "Touch the sensor to start…";
  $("#enroll-status").textContent = "";
  $("#enroll-bar").style.width = "0%";
  dlg.showModal();
  try {
    let step = await invoke("fido_enroll_begin", { name, pin });
    const total = step.remaining + 1;
    show(step, total);
    while (!step.done && !cancelled) {
      step = await invoke("fido_enroll_continue");
      show(step, total);
    }
    if (step.done) { toast("Fingerprint enrolled"); await loadBio(); await refreshFidoInfo(); }
  } catch (err) {
    if (!cancelled) toast(String(err), true);
  } finally {
    if (dlg.open) dlg.close("done");
  }
}

// FIDO dialog fields should not get PIV default-PIN buttons
$$("#dlg-fido-pin input, #dlg-fido-minpin input, #dlg-fido-enroll input, #dlg-fido-rename input, #dlg-fido-passkey input").forEach((i) => i.parentElement.querySelector(".defaults")?.remove());

// =============================== OTP (OATH) mode ============================
let oathAccounts = [];
let oathTimer = null;

async function loadOathReaders() {
  const sel = $("#oath-reader");
  sel.innerHTML = "";
  try {
    const readers = (await invoke("oath_list_readers")).map((r) => typeof r === "string" ? r : String(r && (r.name || r.reader) || ""));
    for (const r of readers) {
      if (!r) continue;
      const o = document.createElement("option");
      o.value = o.textContent = r;
      if (/token2/i.test(r)) o.selected = true;
      sel.append(o);
    }
    $("#oath-connect-msg").textContent = readers.length ? "" : "No smart card readers found.";
  } catch (e) { $("#oath-connect-msg").textContent = String(e); }
}
async function oathConnect(readerArg) {
  let reader = readerArg || $("#oath-reader").value;
  if (reader && typeof reader !== "string") reader = String(reader.name || reader.reader || reader.value || "");
  if (!reader) { await oathConnectHid(); return; }
  // clear any stale unlock/accounts UI from a previous key
  oathInfo = null; t2Info = null; oathAccounts = [];
  $("#oath-unlock").hidden = true; $("#oath-accounts").hidden = true;
  $("#oath-pw") && ($("#oath-pw").value = "");
  $("#btn-t2-unlock-fp") && ($("#btn-t2-unlock-fp").hidden = true);
  // keep the dropdown consistent even if it wasn't populated
  if ($("#oath-reader") && $("#oath-reader").value !== reader) {
    if (![...$("#oath-reader").options].some((o) => o.value === reader)) { const o = document.createElement("option"); o.value = o.textContent = reader; $("#oath-reader").append(o); }
    $("#oath-reader").value = reader;
  }
  // Token2 keys carry their own OTP applet; fall back to the standard OATH applet
  try {
    t2Info = await invoke("t2otp_connect", { reader });
    await refreshCaps();
    renderT2Info();
    const locked = t2Info.pin.set && !t2Info.pin.verified;
    if (page === "oath") setMode("oath");
    $("#oath-unlock").hidden = !locked;
    $("#oath-unlock-label").textContent = "OTP PIN";
    $("#oath-unlock-hint").textContent = "This key's OTP accounts are PIN-protected.";
    if (page === "oath") { $("#oath-accounts").hidden = locked; if (!locked) await loadT2(); }
    else if (!locked) { oathAccounts = await invoke("t2otp_list").catch(() => []); }
    if (page === "home") renderHome(); renderIdent();
    return;
  } catch (e) {
    if (!/no Token2 OTP applet/.test(String(e))) return; // real error already toasted
  }
  await busy(async () => {
    oathInfo = await invoke("oath_connect", { reader });
    renderOathInfo();
    await refreshCaps();
    if (page === "oath") setMode("oath");
    $("#oath-unlock-label").textContent = "Applet password";
    $("#oath-unlock-hint").textContent = "This key's OATH applet is password-protected.";
    $("#oath-unlock").hidden = oathInfo.unlocked;
    $("#oath-accounts").hidden = !oathInfo.unlocked;
    if (oathInfo.unlocked) await loadOath();
  });
}
// Connect OATH over the HID CTAPHID tunnel for a HID-only Token2 key (autodetect).
async function oathConnectHid() {
  let devs = [];
  try { devs = await invoke("otp_hid_detect"); } catch (_) { return false; }
  if (!devs || !devs.length) return false;
  const dev = devs[0];
  oathInfo = null; t2Info = null; oathAccounts = [];
  $("#oath-unlock").hidden = true; $("#oath-accounts").hidden = true;
  try {
    t2Info = await busy(() => invoke("t2otp_connect_hid", { path: dev.path }));
    _otpHidActive = true;
    await refreshCaps();
    renderT2Info();
    const locked = t2Info.pin.set && !t2Info.pin.verified;
    if (page === "oath") setMode("oath");
    $("#oath-unlock").hidden = !locked;
    $("#oath-unlock-label") && ($("#oath-unlock-label").textContent = "OTP PIN");
    $("#oath-unlock-hint") && ($("#oath-unlock-hint").textContent = "This key's OTP accounts are PIN-protected.");
    if (page === "oath") { $("#oath-accounts").hidden = locked; if (!locked) await loadT2(); }
    else if (!locked) { oathAccounts = await invoke("t2otp_list").catch(() => []); }
    if (page === "home") renderHome(); renderIdent();
    return true;
  } catch (e) {
    toast("OTP over USB-HID failed: " + e, true);
    return false;
  }
}
async function oathDisconnect() {
  clearInterval(oathTimer); oathTimer = null;
  if (t2Info) await invoke("t2otp_disconnect").catch(() => {});
  if (oathInfo) await invoke("oath_disconnect").catch(() => {});
  oathInfo = null; t2Info = null; oathAccounts = [];
  setMode("oath");
}
function renderOathInfo() {
  $("#o-version").textContent = oathInfo.version || "—";
  $("#o-devid").textContent = oathInfo.device_id || "—";
  $("#o-password").textContent = oathInfo.password_set ? (oathInfo.unlocked ? "set (unlocked)" : "set") : "not set";
  $("#btn-oath-setpw").textContent = oathInfo.password_set ? "Change password" : "Set password";
  $("#btn-oath-removepw").hidden = !oathInfo.password_set;
}
async function oathUnlock() {
  if (t2Info) {
    await busy(async () => {
      t2Info.pin = await invoke("t2otp_verify_pin", { pin: $("#oath-pw").value, fpEnable: null });
      $("#oath-pw").value = "";
      renderT2Info();
      $("#oath-unlock").hidden = true;
      $("#oath-accounts").hidden = false;
      await loadT2();
    });
    return;
  }
  await busy(async () => {
    oathInfo = await invoke("oath_unlock", { password: $("#oath-pw").value });
    renderOathInfo();
    $("#oath-unlock").hidden = true;
    $("#oath-accounts").hidden = false;
    await loadOath();
  });
}
async function loadOath() {
  oathAccounts = await invoke("oath_list");
  renderOath();
  if (!oathTimer) oathTimer = setInterval(oathTick, 1000);
}
function renderOath() {
  $("#oath-count").textContent = `${oathAccounts.length} stored`;
  $("#btn-oath-lock").hidden = !(t2Info && t2Info.pin && t2Info.pin.set && t2Info.pin.verified);
  const box = $("#oath-list");
  box.innerHTML = "";
  if (!oathAccounts.length) { box.innerHTML = `<p class="hint">No accounts on this key. Use "Add account" to store a TOTP or HOTP secret.</p>`; return; }
  const card = document.createElement("div"); card.className = "otp-card";
  for (const a of oathAccounts) {
    const el = document.createElement("div");
    el.className = "otp"; el.dataset.name = a.name;
    const codeCls = a.code ? "code" : "code empty";
    const codeTxt = a.code ? a.code.replace(/(\d{3})(?=\d)/g, "$1 ") : (a.touch ? "touch required" : "");
    el.innerHTML = `<div class="who"><b>${esc(a.issuer || a.account)}</b><small>${esc(a.issuer ? a.account : "")}${a.issuer && a.account ? " · " : ""}${a.kind.toUpperCase()} ${esc(a.algorithm)}${a.kind === "totp" && a.period !== 30 ? " · " + a.period + " s" : ""}</small></div>
      ${codeTxt ? `<span class="${codeCls}" title="Click to copy" data-act="copy">${codeTxt}</span>` : ""}
      <span class="buttons">${a.touch || a.kind === "hotp" ? `<button data-act="calc">${a.kind === "hotp" ? "Next code" : "Show"}</button>` : ""}<button data-act="del" class="danger">Delete</button></span>`;
    card.append(el);
  }
  box.append(card);
}
function oathTick() {
  if (!oathAccounts.length) return;
  const now = Math.floor(Date.now() / 1000);
  const left = 30 - (now % 30);
  $("#oath-timer").textContent = `· refresh in ${left} s`;
  $$("#oath-list .code").forEach((c) => c.classList.toggle("stale", left <= 5));
  if (left === 30 && mode === "oath") { if (t2Info) loadT2().catch(() => {}); else if (oathInfo) loadOath().catch(() => {}); }
}

// ---- Token2 OTP applet ----
let renderT2Info = function () {
  const c = t2Info.config;
  $("#t-serial").textContent = t2Info.serial || "—";
  $("#t-model-row").hidden = !t2Info.model;
  if (t2Info.model) $("#t-model").textContent = `${t2Info.model.revision} · ${t2Info.model.model}${t2Info.model.branding !== "Token2" ? " · " + t2Info.model.branding : ""}`;
  $("#t-fido").textContent = c.fido_version;
  $("#t-features").textContent = [c.nfc && "NFC", c.ccid && "CCID", c.fingerprint && "fingerprint", c.fido21 && "FIDO 2.1", c.hotp_supported && "HOTP", c.totp_supported && "TOTP"].filter(Boolean).join(", ");
  $("#t-totp").textContent = c.totp_supported ? "enabled" : "disabled";
  $("#btn-t2-totp").textContent = c.totp_supported ? "Disable TOTP" : "Enable TOTP";
  $("#t-btnhotp").textContent = c.button_hotp_unsupported ? "not supported" : c.button_hotp_configured ? `configured (${c.hotp_long_press ? "long press" : "tap"}${c.hotp_no_enter ? ", no Enter" : ""}${c.hotp_numpad ? ", numpad" : ""})` : "not set";
  $("#btn-t2-btnhotp").hidden = c.button_hotp_unsupported;
  $("#t-uvotp").textContent = c.mandatory_fingerprint_supported ? (c.otp_requires_fingerprint ? "required" : "not required") : "not supported on this model";
  const p = t2Info.pin || {};
  $("#t-pin").textContent = !p.supported ? "not supported (needs R3.4+)" : p.set ? (p.verified ? "set (unlocked)" : "set (locked)") : "not set";
  $("#btn-t2-setpin").textContent = p.set ? "Change OTP PIN" : "Set OTP PIN";
  const rv = (function(r){ const m=/^R(\d)(?:\.(\d))?/.exec(r||""); return m? parseInt(m[1])*10+(m[2]?parseInt(m[2]):0):0; })((t2Info.model && t2Info.model.revision) || "");
  $("#btn-t2-setpin").hidden = rv < 34;
  $("#btn-t2-lock").hidden = rv < 34 || !(p.set && p.verified);
  $("#btn-t2-lock").hidden = !(p.set && p.verified);
  $("#t-pin-row").hidden = !p.set;
  if (p.set) { pips($("#t-pin-pips"), Math.min(p.retries_left, 10)); $("#t-pin-retries").textContent = p.retries_left; }
  const fpCapable = p.supported && p.set && (c.fingerprint || c.mandatory_fingerprint_supported);
  $("#btn-t2-fp").hidden = !fpCapable;
  $("#btn-t2-fp").textContent = p.fingerprint_enabled ? "Stop requiring fingerprint for OTP" : "Require fingerprint for OTP";
  $("#btn-t2-unlock-fp").hidden = !(p.set && p.fingerprint_enabled);
  $("#t-uvotp").textContent = p.supported ? (p.fingerprint_enabled ? "yes (fingerprint unlock enabled)" : "no") : $("#t-uvotp").textContent;
}
$("#btn-t2-fp").addEventListener("click", async () => {
  const on = !t2Info.pin.fingerprint_enabled;
  if (!on) {
    const r = await confirmDialog("Stop requiring fingerprint for OTP?", "Enter the OTP PIN to confirm.", { pin: true, okLabel: "Disable" });
    if (!r.ok) return;
    await busy(async () => { t2Info.pin = await invoke("t2otp_verify_pin", { pin: r.pin, fpEnable: false }); toast("Fingerprint requirement removed"); renderT2Info(); });
    return;
  }
  // Enabling needs an enrolled fingerprint on the FIDO applet ("binding"); check and enroll first if missing.
  const r = await confirmDialog("Require fingerprint for OTP?", "Codes will be readable after a fingerprint touch instead of the PIN. The key must have at least one enrolled fingerprint; if it has none you will be guided to enroll one. Enter the OTP PIN to confirm.", { pin: true, okLabel: "Continue" });
  if (!r.ok) return;
  const otpPin = r.pin;
  if (!(await ensureConnected("fido"))) return toast("Could not open the FIDO applet to check fingerprints", true);
  if (!fidoInfo.supports_bio) return toast("This key has no fingerprint sensor", true);
  if (!fidoInfo.has_pin) return toast("Set a FIDO PIN first (FIDO → PIN), then enroll a fingerprint", true);
  let fpin = fidoPin || fidoBioPin;
  if (!fpin) {
    const f = await confirmDialog("FIDO PIN", "Enter the FIDO PIN to check the enrolled fingerprints.", { pin: true, okLabel: "Continue" });
    if (!f.ok || !f.pin) return;
    fpin = f.pin;
  }
  let bio;
  try { bio = await busy(() => invoke("fido_list_bio", { pin: fpin })); fidoBioPin = fpin; }
  catch (e) { return; }
  if (!bio.templates.length) {
    const e = await confirmDialog("No fingerprint enrolled", "Enroll a fingerprint now? It will be used to unlock OTP codes.", { okLabel: "Enroll" });
    if (!e.ok) return;
    showPage("oath");
    await runEnrollment("OTP", fpin);
    bio = await invoke("fido_list_bio", { pin: fpin }).catch(() => ({ templates: [] }));
    if (!bio.templates.length) return toast("No fingerprint was enrolled; fingerprint unlock not enabled", true);
  }
  await busy(async () => { t2Info.pin = await invoke("t2otp_verify_pin", { pin: otpPin, fpEnable: true }); toast("Fingerprint required for OTP"); renderT2Info(); });
  showPage("otp-protection");
});
$("#btn-t2-unlock-fp").addEventListener("click", async () => {
  progress.show("Touch the fingerprint sensor…");
  try {
    await busy(async () => {
      t2Info.pin = await invoke("t2otp_verify_fingerprint");
      renderT2Info();
      $("#oath-unlock").hidden = true; $("#oath-accounts").hidden = false;
      await loadT2();
    });
  } finally { progress.hide(); }
});
$("#btn-t2-setpin").addEventListener("click", () => {
  openDialog("dlg-t2-pin");
  const set = t2Info.pin.set;
  $("#t2-pin-title").textContent = set ? "Change OTP PIN" : "Set OTP PIN";
  $("#t2-curpin-wrap").hidden = !set;
  $("#btn-t2-pin-remove").hidden = !set;
  $("#dlg-t2-pin input[name=current]").required = set;
  $("#dlg-t2-pin input[name=pin]").required = true;
});
$("#btn-t2-pin-remove").addEventListener("click", async (e) => {
  e.preventDefault();
  const cur = $("#dlg-t2-pin input[name=current]").value;
  if (!cur) return toast("Enter the current PIN first", true);
  $("#dlg-t2-pin").close("cancel");
  const r = await confirmDialog("Remove the OTP PIN?", "Stored accounts will be readable without a PIN.", { okLabel: "Remove PIN" });
  if (!r.ok) return;
  await busy(async () => { t2Info.pin = await invoke("t2otp_change_pin", { current: cur, newPin: "" }); toast("OTP PIN removed"); renderT2Info(); });
});
$("#btn-t2-lock").addEventListener("click", async () => {
  await busy(async () => {
    t2Info.pin = await invoke("t2otp_lock"); toast("Locked"); renderT2Info();
    $("#oath-unlock").hidden = false; $("#oath-accounts").hidden = true; oathAccounts = [];
  });
});
$("#btn-t2-ifaces").addEventListener("click", () => {
  openDialog("dlg-t2-ifaces");
  const f = $("#dlg-t2-ifaces form"); const c = t2Info.config;
  f.fido.checked = !c.fido_disabled; f.keyboard.checked = !c.hotp_keystroke_disabled; f.ccid.checked = c.ccid || true;
});
async function refreshT2Info() { t2Info.config = await invoke("t2otp_config"); renderT2Info(); }
async function loadT2() {
  let entries;
  try { entries = await invoke("t2otp_list"); }
  catch (e) {
    if (/PIN required|not verified/i.test(String(e))) { t2Info.pin.verified = false; renderT2Info(); $("#oath-unlock").hidden = false; $("#oath-accounts").hidden = true; return; }
    throw e;
  }
  // map to the shared account model used by the OTP board
  oathAccounts = entries.map((e) => ({
    name: e.app + "\u0000" + e.account, issuer: e.app, account: e.account, kind: e.kind, algorithm: e.algorithm,
    period: e.timestep, touch: e.button, code: e.code, t2: true,
  }));
  renderOath();
  if (!oathTimer) oathTimer = setInterval(oathTick, 1000);
}
$("#btn-t2-add").addEventListener("click", () => openDialog("dlg-t2-add"));
$("#btn-t2-totp").addEventListener("click", async () => {
  const on = !t2Info.config.totp_supported;
  await busy(async () => { await invoke("t2otp_enable_totp", { enabled: on }); toast(on ? "TOTP enabled" : "TOTP disabled"); await refreshT2Info(); });
});
$("#btn-t2-btnhotp").addEventListener("click", () => {
  openDialog("dlg-t2-btnhotp");
  const f = $("#dlg-t2-btnhotp form"); const c = t2Info.config;
  f.no_enter.checked = c.hotp_no_enter; f.long_press.checked = c.hotp_long_press; f.numpad.checked = c.hotp_numpad;
  $("#btn-t2-btnhotp-delete").hidden = !c.button_hotp_configured;
});
$("#btn-t2-btnhotp-delete").addEventListener("click", async (e) => {
  e.preventDefault();
  $("#dlg-t2-btnhotp").close("cancel");
  const r = await confirmDialog("Remove the button HOTP seed?", "The key will no longer type a code on button press.", { okLabel: "Remove" });
  if (!r.ok) return;
  await busy(async () => { await invoke("t2otp_delete_button_hotp"); toast("Button HOTP seed removed"); await refreshT2Info(); });
});
$("#btn-t2-erase").addEventListener("click", async () => {
  const r = await confirmDialog("Erase all OTP accounts on the key?", "All TOTP/HOTP entries are erased. The key asks for a button press to confirm.", { okLabel: "Erase everything" });
  if (!r.ok) return;
  progress.show("Touch the key to confirm…");
  try { await busy(async () => { await invoke("t2otp_erase_all"); toast("All accounts erased"); await loadT2(); }); } finally { progress.hide(); }
});
$("#dlg-t2-add input[name=uri]").addEventListener("change", async (e) => {
  if (!e.target.value.trim()) return;
  try {
    const o = await invoke("oath_parse_uri", { uri: e.target.value });
    const f = e.target.form;
    f.app.value = o.issuer; f.account.value = o.account; f.secret.value = o.secret;
    f.kind.value = o.kind; f.algorithm.value = o.algorithm === "SHA256" ? "SHA256" : "SHA1";
    f.digits.value = o.digits; f.timestep.value = o.period;
  } catch (err) { toast(String(err), true); }
});
$("#oath-list").addEventListener("click", async (e) => {
  const t = e.target.closest("[data-act]");
  if (!t) return;
  const row = t.closest("[data-name]");
  const a = oathAccounts.find((x) => x.name === row.dataset.name);
  if (t.dataset.act === "copy") {
    if (a.code) { navigator.clipboard?.writeText(a.code).then(() => toast("Code copied")).catch(() => {}); }
  } else if (t.dataset.act === "calc") {
    if (a.touch) progress.show("Touch the key…");
    try {
      const code = a.t2
        ? (await busy(() => invoke("t2otp_read", { app: a.issuer, account: a.account }))).code
        : await busy(() => invoke("oath_calculate", { name: a.name, kind: a.kind, period: a.period }));
      a.code = code; renderOath();
    } finally { progress.hide(); }
  } else if (t.dataset.act === "del") {
    const r = await confirmDialog("Delete this account from the key?", `${a.issuer ? a.issuer + " — " : ""}${a.account}. You will need the original secret or QR code to add it again.`, { okLabel: "Delete" });
    if (!r.ok) return;
    if (a.t2) await busy(async () => { await invoke("t2otp_delete", { app: a.issuer, account: a.account }); toast("Account deleted"); await loadT2(); });
    else await busy(async () => { await invoke("oath_delete", { name: a.name }); toast("Account deleted"); await loadOath(); });
  }
});
$("#btn-oath-refresh").addEventListener("click", loadOathReaders);
$("#btn-oath-lock").addEventListener("click", () => $("#btn-t2-lock").click());
$("#btn-oath-connect").addEventListener("click", oathConnect);
$("#btn-oath-detect") && $("#btn-oath-detect").addEventListener("click", rescanAll);
$("#btn-oath-unlock").addEventListener("click", oathUnlock);
$("#oath-pw").addEventListener("keydown", (e) => { if (e.key === "Enter") oathUnlock(); });
$("#btn-oath-add").addEventListener("click", () => openDialog("dlg-oath-add"));
$("#btn-oath-removepw").addEventListener("click", async () => {
  const r = await confirmDialog("Remove the applet password?", "Anyone with the key will be able to read the codes.", { okLabel: "Remove password" });
  if (!r.ok) return;
  await busy(async () => { oathInfo = await invoke("oath_set_password", { password: "" }); toast("Password removed"); renderOathInfo(); });
});
$("#btn-oath-reset").addEventListener("click", async () => {
  const r = await confirmDialog("Erase all OTP accounts?", "All TOTP/HOTP secrets and the applet password are erased. This cannot be undone.", { okLabel: "Erase everything" });
  if (!r.ok) return;
  await busy(async () => { oathInfo = await invoke("oath_reset"); toast("OATH applet erased"); renderOathInfo(); await loadOath(); });
});
// otpauth URI autofill in the add dialog
async function fillFromOtpauthUri(uri, dlgId) {
  try {
    const o = await invoke("oath_parse_uri", { uri });
    const f = $("#" + dlgId + " form");
    const setv = (name, val) => { if (f[name] !== undefined) f[name].value = val; };
    if (dlgId === "dlg-t2-add") { setv("app", o.issuer); setv("account", o.account); setv("secret", o.secret); setv("kind", o.kind); setv("algorithm", o.algorithm === "SHA256" ? "SHA256" : "SHA1"); setv("digits", o.digits); setv("timestep", o.period); }
    else { setv("issuer", o.issuer); setv("account", o.account); setv("secret", o.secret); setv("kind", o.kind); setv("algorithm", ["SHA1","SHA256","SHA512"].includes(o.algorithm) ? o.algorithm : "SHA1"); setv("digits", String(o.digits)); setv("period", o.period); setv("counter", o.counter); }
    toast("Filled from QR");
  } catch (e) { toast(String(e), true); }
}
async function scanQr(kind, dlgId) {
  try {
    const uris = await busy(() => invoke(kind === "screen" ? "qr_scan_screen" : "qr_scan_file"));
    if (!uris || !uris.length) return toast("No OTP QR code found", true);
    const otpauth = uris.find((u) => u.startsWith("otpauth://"));
    if (!otpauth) return toast("QR found but it is not an OTP account (migration payloads not supported yet)", true);
    await fillFromOtpauthUri(otpauth, dlgId);
  } catch (e) { toast(String(e), true); }
}
document.addEventListener("click", (e) => {
  const b = e.target.closest("button[data-qr]");
  if (!b) return;
  const dlg = b.closest("dialog");
  scanQr(b.dataset.qr, dlg.id);
});
// header "Add OTP profile": open the right add-dialog for this key
$("#btn-oath-add-qr").addEventListener("click", async () => {
  const dlgId = t2Info ? "dlg-t2-add" : "dlg-oath-add";
  openDialog(dlgId);
  await scanQr("screen", dlgId);
});
$("#pfx-browse").addEventListener("click", async () => {
  const pick = await invoke("pfx_pick");
  if (!pick) return;
  const f = $("#dlg-pfx form");
  f.file.value = pick.file; f.file.dataset.path = pick.path;
  await pfxCheck();
});
async function pfxCheck() {
  const f = $("#dlg-pfx form");
  const path = f.file.dataset.path;
  if (!path) { $("#pfx-status").textContent = ""; $("#pfx-import").disabled = true; return; }
  try {
    const [algo, hasCert] = await invoke("pfx_probe", { path, password: f.password.value || "" });
    $("#pfx-status").textContent = `✓ ${algo} key${hasCert ? " + certificate" : ""} — ready to import`;
    $("#pfx-status").className = "hint ok-text";
    $("#pfx-import").disabled = false;
  } catch (e) {
    $("#pfx-status").textContent = String(e);
    $("#pfx-status").className = "hint err";
    $("#pfx-import").disabled = true;
  }
}
$("#dlg-pfx input[name=password]").addEventListener("input", () => { clearTimeout(window._pfxT); window._pfxT = setTimeout(pfxCheck, 300); });
$("#dlg-oath-add input[name=uri]").addEventListener("change", async (e) => {
  if (!e.target.value.trim()) return;
  try {
    const o = await invoke("oath_parse_uri", { uri: e.target.value });
    const f = e.target.form;
    f.issuer.value = o.issuer; f.account.value = o.account; f.secret.value = o.secret;
    f.kind.value = o.kind; f.algorithm.value = ["SHA1", "SHA256", "SHA512"].includes(o.algorithm) ? o.algorithm : "SHA1";
    f.digits.value = String(o.digits); f.period.value = o.period; f.counter.value = o.counter;
  } catch (err) { toast(String(err), true); }
});

// ================================ menubar ===================================
// Pages the board can show: piv | fido | oath | settings | fido-settings | otp-protection | about
var page = "home";
var PAGE_IDS = ["settings-board", "fido-settings-board", "otp-protection-board", "about-board", "home-board"];
function clearStalePages() {
  // when nothing is connected, wipe any residual per-applet content
  if (!t2Info) { const b = $("#oath-list"); if (b && b.innerHTML !== undefined) b.innerHTML = ""; }
  if (!info) { const s = $("#primary-slots"); if (s) s.innerHTML = ""; const r = $("#retired-slots"); if (r) r.innerHTML = ""; }
  if (!fidoInfo) { const t = $("#fido-templates"); if (t) t.innerHTML = ""; const rp = $("#fido-rps"); if (rp) rp.innerHTML = ""; }
}
function showPage(p) {
  page = p;
  clearStalePages();
  if (p === "home") {
    ["empty","slots","fido-empty","fido-board","oath-empty","oath-board"].forEach((id) => { const e = document.getElementById(id); if (e) e.hidden = true; });
    PAGE_IDS.forEach((id) => { const e = document.getElementById(id); if (e) e.hidden = id !== "home-board"; });
    resetKeyPanels();
    renderIdent();
    renderHome();
    renderHomeInfo();
    return;
  }
  if (["piv", "fido", "oath"].includes(p)) {
    setMode(p);
    renderIdent();
    resetKeyPanels();
    PAGE_IDS.forEach((id) => ($("#" + id).hidden = true));
    if (p === "oath" && (t2Info || oathInfo)) {
      const locked = t2Info ? (t2Info.pin.set && !t2Info.pin.verified) : !oathInfo.unlocked;
      $("#oath-unlock").hidden = !locked; $("#oath-accounts").hidden = locked;
      if (!locked) { if (t2Info) loadT2().catch(() => {}); else loadOath().catch(() => {}); }
    }
    return;
  }
  // keep the sidebar of the relevant mode, replace the board
  const m = p === "fido-settings" ? "fido" : p === "otp-protection" ? "oath" : mode;
  setMode(m);
  ["empty", "slots", "fido-empty", "fido-board", "oath-empty", "oath-board"].forEach((id) => ($("#" + id).hidden = true));
  PAGE_IDS.forEach((id) => ($("#" + id).hidden = id !== p + "-board" && !(p === "settings" && id === "settings-board")));
  if (p === "settings") renderSettingsPage();
  if (p === "fido-settings") renderFidoSettings();
  if (p === "otp-protection") renderOtpProtection();
}
// hide the old sidebar action stacks: the menus replace them
$$(".actions").forEach((n) => (n.hidden = true));

let _capsForKey = "";  // cache the probe result per inserted key
$("#pcsc-warning-dismiss") && $("#pcsc-warning-dismiss").addEventListener("click", () => { const b=$("#pcsc-warning"); if(b) b.hidden=true; try{ sessionStorage.setItem("pcscWarnDismissed","1"); }catch(_){} });
async function checkPcscHealth() {
  // Only meaningful where PC/SC is the transport for FIDO/PIV. If the service is
  // down, those applets silently vanish; show an actionable banner.
  const banner = $("#pcsc-warning"); if (!banner) return;
  if (sessionStorage.getItem("pcscWarnDismissed") === "1") { banner.hidden = true; return; }
  try {
    const h = await invoke("pcsc_health");
    if (h && h.available === false) {
      const t = $("#pcsc-warning-text"); if (t) t.textContent = h.message || "The smart-card service is not available.";
      banner.hidden = false;
    } else { banner.hidden = true; }
  } catch (_) { /* command missing on older builds — ignore */ }
}
async function refreshCaps() {
  // If ANY applet session is live, never probe: probing connects to the reader
  // and can reset the card mid-operation (the cert -5 bug). We already know the
  // caps for a connected key, so just keep the cached set + the live session.
  if (anyConnected()) {
    caps.piv = caps.piv || !!info;
    caps.token2_otp = caps.token2_otp || !!t2Info;
    caps.oath = caps.oath || !!oathInfo;
    caps.fido = caps.fido || !!fidoInfo;
    if (t2Info && t2Info.model && t2Info.model.revision) { caps.revision = t2Info.model.revision; caps.model = t2Info.model.model; }
    // If the revision is still unknown but the OTP applet exists, probe once to
    // fill it so Device Settings gating is correct (only when nothing PIV/OTP is
    // live on the reader — a FIDO CCID session shares the card, so skip then).
    if (!caps.revision && caps.token2_otp && !info && !t2Info && !oathInfo && !fidoInfo) {
      try { const pr = await invoke("key_capabilities", {}); if (pr.revision) { caps.revision = pr.revision; caps.model = pr.model; } } catch (_) {}
    }
    capsProbed = true;
    applyCaps();
    if (page === "home") renderHomeInfo();
    return;
  }
  // No session: safe to probe. Cache by reader signature so we don't re-probe the
  // same key every second.
  const sig = ($("#oath-reader") && $("#oath-reader").value) || "";
  try {
    let probed = await invoke("key_capabilities", {});
    // one retry if the OTP applet is present but the serial/revision didn't read
    if (probed.token2_otp && !probed.revision) {
      try { const p2 = await invoke("key_capabilities", {}); if (p2.revision) probed = p2; } catch (_) {}
    }
caps = { piv: probed.piv, oath: probed.oath, token2_otp: probed.token2_otp, fido: probed.fido, openpgp: probed.openpgp, revision: probed.revision || "", model: probed.model || "", serial: probed.serial || "" };
    plog(`refreshCaps probe -> piv=${probed.piv} otp=${probed.token2_otp} fido=${probed.fido} openpgp=${probed.openpgp} readers=${_presentReadersCount}`);
    // HID-only Token2 key (PIN+ Release2, no reader): OATH tunnels over CTAPHID.
    // Autodetect (cached) and mark OATH available so the OTP card is usable.
    if (!caps.token2_otp && !caps.oath && !(_presentReadersCount > 0)) {
      if (_otpHidCache === undefined) {
        try { const hd = await invoke("otp_hid_detect"); _otpHidCache = (hd && hd.length) ? hd[0] : null; } catch (_) { _otpHidCache = null; }
      }
      if (_otpHidCache) { caps.oath = true; if (!caps.serial && _otpHidCache.serial) caps.serial = _otpHidCache.serial; }
    }
    capsProbed = true;
    _capsForKey = sig;
    if (page === "home") renderHomeInfo();
  } catch (_) {
    if (!capsProbed) caps = { piv: false, oath: false, token2_otp: false, fido: false, openpgp: false };
  }
  applyCaps();
}
function applyCaps() {
  if (!capsProbed) return; // don't grey anything until the first real probe
  const otp = caps.token2_otp || caps.oath;
  const connected = anyConnected();
  // when a key is connected, HIDE items for applets it lacks; otherwise just disable
  const dis = (mi, off) => { const b = document.querySelector(`[data-mi="${mi}"]`); if (b) { b.disabled = off; b.hidden = off && connected; b.title = off ? "Not available on this key" : ""; } };
  const pivItems = ["piv-certs","piv-pin","piv-admin","piv-puk","piv-unblock","piv-retries","piv-attest","piv-reset"];
  const otpItems = ["otp-accounts","otp-hidhotp","otp-reset"];
  const otpPinItems = ["otp-protection"]; // R3.4+ only
  const t2Items = ["device-settings"]; // Token2 OTP applet only
  const fidoItems = ["fido-pin","fido-passkeys","fido-fingerprints","fido-settings","fido-reset"];
  pivItems.forEach((mi) => dis(mi, !caps.piv));
  otpItems.forEach((mi) => dis(mi, !otp));
  const revNumOf = (r) => { const m=/^R(\d)(?:\.(\d))?/.exec(r||""); return m? parseInt(m[1])*10+(m[2]?parseInt(m[2]):0):0; };
  const rev = caps.revision || (t2Info && t2Info.model && t2Info.model.revision) || "";
  const n = revNumOf(rev);
  const rvn = n;
  otpPinItems.forEach((mi) => dis(mi, !caps.token2_otp || rvn < 34));
  const hasOtpApplet = caps.token2_otp;
  const showIface = hasOtpApplet && n > 0;              // USB interfaces (FIDO/HID/CCID) — all OTP-applet keys, incl. R3.4+
  const showApplet = hasOtpApplet && n >= 34;            // Applets (OpenPGP/PIV)
  const ds = document.querySelector('[data-mi="device-settings"]'); if (ds) { ds.hidden = connected && !(showIface || showApplet); ds.disabled = !(showIface || showApplet); }
  fidoItems.forEach((mi) => dis(mi, !caps.fido));
  // grey the top-level menu button when the whole group is unavailable
  const menuOff = (name, off) => { const m = document.querySelector(`.menu[data-menu="${name}"] .menu-btn`); if (m) { m.style.opacity = off ? 0.4 : ""; m.title = off ? "Not available on this key" : ""; } };
  menuOff("piv", !caps.piv); menuOff("fido", !caps.fido); menuOff("otp", !otp);
  // fully hide a menu button when connected and the group is entirely unavailable
  const hideMenu = (name, hide) => { const m = document.querySelector(`.menu[data-menu="${name}"]`); if (m) m.hidden = hide && connected; };
  hideMenu("piv", !caps.piv); hideMenu("fido", !caps.fido); hideMenu("otp", !otp);
  const settingsHasAny = showIface || showApplet;
  const sm = document.querySelector('.menu[data-menu="settings"]'); if (sm) sm.hidden = connected && !settingsHasAny;
}

// dropdown behaviour
$$(".menu").forEach((m) => {
  m.querySelector(".menu-btn").addEventListener("click", (e) => { e.stopPropagation(); const open = m.classList.contains("open"); $$(".menu").forEach((x) => x.classList.remove("open")); if (!open) m.classList.add("open"); });
});
document.addEventListener("click", () => $$(".menu").forEach((x) => x.classList.remove("open")));

function featureAvailable(m) {
  if (!capsProbed) return true; // unknown yet, allow the attempt
  if (m === "piv") return caps.piv;
  if (m === "fido") return caps.fido;
  if (m === "oath") return caps.token2_otp || caps.oath;
  return true;
}
async function menuAction(mi) {
  const btn = document.querySelector(`[data-mi="${mi}"]`);
  if (btn && btn.disabled) return toast("Not available on this key", true);
  const need = async (m) => {
    if (!featureAvailable(m)) { toast(m === "oath" ? "This key has no OTP applet" : m === "piv" ? "This key has no PIV applet" : "This key has no FIDO2 support", true); return false; }
    const ok = await ensureConnected(m);
    if (!ok && !(m === "fido" && _fidoMsgShown)) toast(anyConnected() ? (m === "oath" ? "This key has no OTP applet" : m === "piv" ? "This key has no PIV applet" : "This key has no FIDO2 support") : "No key detected — insert a key", true);
    return ok;
  };
  switch (mi) {
    case "reconnect": await reconnectAll(); break;
    case "disconnect": await disconnectAllApplets(); await teardown("Disconnected"); toast("Key disconnected"); break;
    case "diag": try { const d = await invoke("capabilities_debug"); let ohd = ""; try { ohd = await invoke("otp_hid_debug"); } catch (_) {} const otpcfg = t2Info && t2Info.config ? `\n=== OTP config (Token2 applet) ===\nraw: ${t2Info.config.raw}\nfido_version: ${t2Info.config.fido_version}\nhotp_supported: ${t2Info.config.hotp_supported}  totp_supported: ${t2Info.config.totp_supported}\nbutton_hotp_configured: ${t2Info.config.button_hotp_configured}\nbutton_hotp_unsupported: ${t2Info.config.button_hotp_unsupported}  (byte9 bit6 -> hides the Button-HOTP panel)\nhotp_numpad: ${t2Info.config.hotp_numpad}  hotp_long_press: ${t2Info.config.hotp_long_press}  ccid: ${t2Info.config.ccid}  nfc: ${t2Info.config.nfc}\n` : "\n(no Token2 OTP applet connected — open the OTP page first)\n";
    const ui = `\n=== UI state ===\ncapsProbed: ${capsProbed}\ncaps: ${JSON.stringify(caps)}\nfidoDevices(${fidoDevices.length}): ${JSON.stringify(fidoDevices.map(d=>({p:d.product,t:d.transport})))}\nconnected: piv=${!!info} fido=${!!fidoInfo} otp=${!!t2Info} oath=${!!oathInfo}\nelevated(ui): ${isElevated}\nDevice Settings should show: ${(caps.token2_otp && (function(r){const m=/^R(\\d)(?:\\.(\\d))?/.exec(r||"");return m?parseInt(m[1])*10+(m[2]?parseInt(m[2]):0):0;})(caps.revision) >= 34) ? "Applets (R3.4+)" : (caps.token2_otp && (function(r){const m=/^R(\\d)(?:\\.(\\d))?/.exec(r||"");return m?parseInt(m[1])*10+(m[2]?parseInt(m[2]):0):0;})(caps.revision) > 0) ? "USB interfaces" : "hidden"}`; showText("Capability diagnostics", d + ui + "\n\n=== probe log ===\n" + _probeLog.join("\n") + otpcfg + "\n=== OTP-HID probe ===\n" + ohd, "capabilities.txt"); } catch (err) { toast(String(err), true); } break;
    case "exit": { if (info) await disconnect().catch(() => {}); if (fidoInfo) await fidoDisconnect().catch(() => {}); if (oathInfo || t2Info) await oathDisconnect().catch(() => {}); await invoke("exit_app"); break; }
    // OTP
    case "otp-accounts": if (await need("oath")) { showPage("oath"); if (t2Info) { const locked = t2Info.pin.set && !t2Info.pin.verified; $("#oath-unlock").hidden = !locked; $("#oath-accounts").hidden = locked; if (!locked) await loadT2(); } else if (oathInfo) { $("#oath-unlock").hidden = oathInfo.unlocked; $("#oath-accounts").hidden = !oathInfo.unlocked; if (oathInfo.unlocked) await loadOath(); } } break;
    case "otp-hidhotp": if (await need("oath")) { if (t2Info) { showPage("oath"); $("#btn-t2-btnhotp").click(); } else toast("HID-HOTP is a Token2 OTP applet feature; this key has only the OATH applet", true); } break;
    case "otp-protection": if (await need("oath")) showPage("otp-protection"); break;
    case "otp-reset": if (await need("oath")) { showPage("oath"); if (t2Info) { const r = await confirmDialog("Reset the OTP applet?", "All TOTP/HOTP accounts, the OTP PIN and button-HOTP are erased. The key asks for a button press.", { okLabel: "Erase everything" }); if (!r.ok) return; progress.show("Touch the key to confirm…"); try { await busy(() => invoke("t2otp_applet_reset")); toast("OTP applet reset"); t2Info.pin = await invoke("t2otp_pin_status").catch(() => t2Info.pin); await loadT2(); renderT2Info(); } finally { progress.hide(); } } else $("#btn-oath-reset").click(); } break;
    // FIDO
    case "fido-pin": if (await need("fido")) { showPage("fido"); $("#btn-fido-pin").click(); } break;
    case "fido-passkeys": if (await need("fido")) { showPage("fido"); showFidoSub("passkeys"); } break;
    case "fido-fingerprints": if (await need("fido")) { showPage("fido"); if (!fidoInfo.supports_bio) return toast("This key has no fingerprint sensor"); showFidoSub("fingerprints"); } break;
    case "fido-settings": if (await need("fido")) showPage("fido-settings"); break;
    case "fido-reset": if (await need("fido")) { showPage("fido"); $("#btn-fido-reset").click(); } break;
    // PIV
    case "piv-certs": if (await need("piv")) showPage("piv"); break;
    case "piv-pin": if (await need("piv")) { showPage("piv"); openDialog("dlg-pin"); } break;
    case "piv-admin": if (await need("piv")) { showPage("piv"); openDialog("dlg-mgm"); } break;
    case "piv-puk": if (await need("piv")) { showPage("piv"); openDialog("dlg-puk"); } break;
    case "piv-unblock": if (await need("piv")) { showPage("piv"); openDialog("dlg-unblock"); setupUnblockToggle(); } break;
    case "piv-retries": if (await need("piv")) { showPage("piv"); openDialog("dlg-retries"); } break;
    case "piv-attest": if (await need("piv")) { showPage("piv"); $("#btn-attest-chain").click(); } break;
    case "piv-fplogin": if (await need("piv")) { showPage("piv"); await openPivFpDialog(); } break;
    case "piv-reset": if (await need("piv")) { showPage("piv"); $("#btn-reset").click(); } break;
    // Settings
    case "device-settings": if (await need("oath")) { if (t2Info) showPage("settings"); else toast("Device settings need the Token2 OTP applet", true); } break;
    case "about": showPage("about"); break;
  }
}
$$(".dropdown button[data-mi]").forEach((b) => b.addEventListener("click", (e) => { e.stopPropagation(); $$(".menu").forEach((x) => x.classList.remove("open")); menuAction(b.dataset.mi).catch((err) => toast(String(err), true)); }));

// ---- Settings page (Token2 OTP applet: interfaces + applets)
function revNum(rev) { const m = /^R(\d)(?:\.(\d))?/.exec(rev || ""); return m ? parseInt(m[1]) * 10 + (m[2] ? parseInt(m[2]) : 0) : 0; }
async function renderSettingsPage() {
  // Applet/interface config only applies to Token2 keys with the OTP applet.
  if (!t2Info) return;
  const c = t2Info.config;
  const rev = t2Info.model && t2Info.model.revision;
  const n = revNum(rev);
  const ifaceCard = $("#set-interfaces-card"); // FIDO / HID-HOTP / CCID — all OTP-applet keys
  const appletCard = $("#set-applets-card");   // USB/NFC OpenPGP+PIV — R3.4+
  const showIface = n > 0;
  const showApplet = n >= 34;
  if (ifaceCard) ifaceCard.hidden = !showIface;
  if (appletCard) appletCard.hidden = !showApplet;
  // section heading for interfaces (if present)
  if (showIface) {
    $("#tg-fido").checked = !c.fido_disabled;
    $("#tg-hid").checked = !c.hotp_keystroke_disabled;
    $("#tg-ccid").checked = true;
  }
  if (!showApplet) return;
  try {
    const f = await invoke("t2otp_applet_flags");
    $("#tg-usb-piv").checked = f.piv_usb; $("#tg-usb-pgp").checked = f.openpgp_usb;
    $("#tg-nfc-piv").checked = f.piv_nfc; $("#tg-nfc-pgp").checked = f.openpgp_nfc;
    ["tg-usb-piv","tg-usb-pgp","tg-nfc-piv","tg-nfc-pgp"].forEach((id) => $("#" + id).disabled = false);
    $("#btn-apply-applets").disabled = false;
    $("#applet-mode-line").textContent = `Applet flags: 0x${f.raw.toString(16).padStart(2,"0")} — toggles are live.`;
  } catch (e) {
    ["tg-usb-piv","tg-usb-pgp","tg-nfc-piv","tg-nfc-pgp"].forEach((id) => { $("#" + id).checked = true; $("#" + id).disabled = true; });
    $("#btn-apply-applets").disabled = true;
    $("#applet-mode-line").textContent = "Could not read applet state: " + e;
  }
}
$("#btn-apply-applets") && $("#btn-apply-applets").addEventListener("click", async () => {
  const args = { usbPiv: $("#tg-usb-piv").checked, usbPgp: $("#tg-usb-pgp").checked, nfcPiv: $("#tg-nfc-piv").checked, nfcPgp: $("#tg-nfc-pgp").checked };
  if (!args.usbPiv && !args.usbPgp && !args.nfcPiv && !args.nfcPgp && !$("#tg-hid").checked) {
    // still ok — CCID/FIDO stay; but warn if disabling everything visible
  }
  const r = await confirmDialog("Apply applet changes?", "Disabled applets stop responding until re-enabled. The key may re-enumerate; reconnect it afterwards.", { okLabel: "Apply" });
  if (!r.ok) return;
  const disablingAllCcid = !args.usbPiv && !args.usbPgp;
  await busy(async () => { await invoke("t2otp_set_applet_flags", args); });
  toast(disablingAllCcid ? "Applied — the card channel changed; reconnecting…" : "Applet settings applied — reconnecting…");
  // full teardown + re-detect handles the re-enumeration cleanly
  await reconnectAll();
});
$("#btn-apply-interfaces").addEventListener("click", async () => {
  const on = $("#tg-hid").checked;
  if (on === !t2Info.config.hotp_keystroke_disabled) return toast("No change");
  const r = await confirmDialog(on ? "Enable the HID-HOTP interface?" : "Disable the HID-HOTP interface?", "The key re-enumerates on USB. Reconnect it afterwards.", { okLabel: "Apply" });
  if (!r.ok) return;
  await busy(async () => { await invoke("t2otp_set_keyboard_interface", { keyboardEnabled: on }); });
  toast("Applied — reconnecting…");
  await reconnectAll();
});

// ---- FIDO settings page
let _noteCache = {};
// Note feature temporarily disabled (CLI largeBlob write still deadlocks).
const NOTE_FEATURE = false;
function noteShowHome(note) { const dn = $("#di-note"), row = $("#di-note-row"); if (dn && row) { dn.textContent = note || ""; row.hidden = !note; } }
function noteSerial() { return caps.serial || (t2Info && t2Info.serial) || (fidoInfo && fidoInfo.serial) || ""; }
async function loadFidoNote() {
  const card = $("#fs-note-card"), banner = $("#fido-note-banner"), row = $("#di-note-row");
  if (card) card.hidden = true; if (banner) banner.hidden = true; if (row) row.hidden = true;
  if (!NOTE_FEATURE) return;
}
async function maybeLoadNoteForHome(serial) {
  if (!NOTE_FEATURE) { const row = $("#di-note-row"); if (row) row.hidden = true; return; }
}
function renderFidoSettings() {
  { const card = $("#fs-note-card"), ta = $("#fs-note-text"); const sup = NOTE_FEATURE && fidoInfo && fidoInfo.supports_largeblob;
    if (card) card.hidden = !sup;
    const sn = noteSerial();
    if (sup && ta) { if (sn in _noteCache) ta.value = _noteCache[sn]; else loadFidoNote().catch(()=>{}); } }
  { const fp = $("#fs-go-fingerprints"); if (fp) fp.hidden = !(fidoInfo && fidoInfo.supports_bio);
    document.querySelectorAll("#fido-settings-board .nc-ico[data-icon]").forEach((el) => { if (!el.innerHTML) el.innerHTML = svg(el.dataset.icon); }); }
  if (!fidoInfo) { $("#fido-pin-state").textContent = "No FIDO2 key connected."; return; }
  $("#fido-pin-state").textContent = fidoInfo.new_pin_required
    ? "The key requires a PIN change before anything else (the current PIN is shorter than the configured minimum). Change it now."
    : fidoInfo.has_pin ? "Your FIDO key already has a PIN set. You can change it, or reset the FIDO applet." : "Your FIDO key has no PIN yet.";
  { const cp = $("#fs-change-pin"); if (cp) cp.textContent = fidoInfo.has_pin ? "Change PIN" : "Set PIN"; }
  $("#fs-min-num").value = fidoInfo.min_pin_len || 4;
  $("#fs-min-alpha").value = 10;
  // alphanumeric minimum is a Token2 vendor command tunnelled through the OTP
  // applet; it does not exist on FIDO-only keys -> hide the whole row there.
  const hasT2Otp = !!caps.token2_otp;
  const alphaRow = $("#fs-min-alpha-row");
  if (alphaRow) alphaRow.hidden = !hasT2Otp;
  $("#fs-min-hint").textContent = hasT2Otp
    ? "The numeric minimum is the standard CTAP setting; the alphanumeric minimum is a Token2 feature applied over the smart-card channel (no admin rights)."
    : "Minimum PIN length is the standard CTAP setting.";
  const canCfg = !!fidoInfo.supports_config;
  const hasAlwaysUv = canCfg && fidoInfo.options.some(([k]) => k === "alwaysUv");
  const fpr = $("#fs-forcepin-row"); if (fpr) fpr.hidden = !(fidoInfo.has_pin && fidoInfo.supports_config);
  $("#fs-always-uv").checked = !!fidoInfo.always_uv;
  $("#fs-always-uv").disabled = !hasAlwaysUv;
  const row = $("#fs-always-uv").closest(".setting-row");
  if (row) { row.style.opacity = hasAlwaysUv ? "" : "0.5"; row.title = hasAlwaysUv ? "" : "This key does not support changing always-UV"; }
  // min PIN length + force change also need authenticatorConfig
  $("#fs-min-num-btn").disabled = !canCfg;
  $("#fs-min-alpha-btn").disabled = !canCfg;
}
$("#fs-go-passkeys") && $("#fs-go-passkeys").addEventListener("click", () => { showPage("fido"); showFidoSub("passkeys"); });
$("#fs-back-passkeys") && $("#fs-back-passkeys").addEventListener("click", () => showPage("fido-settings"));
$("#fs-back-bio") && $("#fs-back-bio").addEventListener("click", () => showPage("fido-settings"));
$("#fs-back-unlock") && $("#fs-back-unlock").addEventListener("click", () => showPage("fido-settings"));
$("#fs-change-pin") && $("#fs-change-pin").addEventListener("click", () => $("#btn-fido-pin").click());
{ const ta = $("#fs-note-text"), cnt = $("#fs-note-count"); if (ta && cnt) { const upd = () => cnt.textContent = String(ta.value.length); ta.addEventListener("input", upd); upd(); } }
$("#fs-note-save") && $("#fs-note-save").addEventListener("click", async () => {
  if (!NOTE_FEATURE) return;
  const text = ($("#fs-note-text") && $("#fs-note-text").value || "").slice(0, 56);
  let pin = fidoPin;
  if (!pin) { const r = await confirmDialog("FIDO PIN", "Enter the FIDO PIN to save the note.", { pin: true }); if (!r.ok) return; pin = r.pin; }
  busy(async () => {
    await invoke("fido_note_set", { text, pin });
    fidoPin = pin;
    try { await refreshFidoInfo(); } catch (_) {}
    toast("Note saved to the key"); await loadFidoNote();
  }).catch((e) => toast(String(e), true));
});
$("#fs-note-clear") && $("#fs-note-clear").addEventListener("click", async () => {
  if (!NOTE_FEATURE) return;
  let pin = fidoPin;
  if (!pin) { const r = await confirmDialog("FIDO PIN", "Enter the FIDO PIN to clear the note.", { pin: true }); if (!r.ok) return; pin = r.pin; }
  busy(async () => {
    await invoke("fido_note_clear", { pin });
    fidoPin = pin;
    try { await refreshFidoInfo(); } catch (_) {}
    const ta = $("#fs-note-text"); if (ta) ta.value = ""; const sn = noteSerial(); if (sn) _noteCache[sn] = "";
    toast("Note cleared"); noteShowHome("");
  }).catch((e) => toast(String(e), true));
});
$("#fs-go-fingerprints") && $("#fs-go-fingerprints").addEventListener("click", () => { if (fidoInfo && !fidoInfo.supports_bio) return toast("This key has no fingerprint sensor"); showPage("fido"); showFidoSub("fingerprints"); });
$("#fs-reset").addEventListener("click", () => $("#btn-fido-reset").click());
$("#piv-fp-toggle") && $("#piv-fp-toggle").addEventListener("change", async (e) => {
  const enable = e.target.checked;
  const r = await confirmDialog(enable ? "Enable fingerprint for PIV login?" : "Disable fingerprint for PIV login?",
    enable ? "This key will accept the enrolled fingerprint for PIV/smart-card login. A fingerprint must already be enrolled (FIDO \u2192 Fingerprints)." : "PIV login will require the PIN again.",
    { pin: true, defaults: true, okLabel: enable ? "Enable" : "Disable" });
  if (!r.ok) { e.target.checked = !enable; return; }
  try {
    await busy(() => invoke("piv_set_fp_binding", { pin: r.pin, enable }));
    toast(enable ? "Fingerprint for PIV login enabled" : "Disabled");
  } catch (err) { e.target.checked = !enable; toast(String(err), true); }
});

$("#fs-forcepin") && $("#fs-forcepin").addEventListener("click", () => $("#btn-fido-forcepin").click());
$("#fs-min-num-btn").addEventListener("click", async () => {
  const len = Number($("#fs-min-num").value);
  const r = await confirmDialog("Set minimum PIN length?", `New minimum: ${len} characters. Can only be increased.` + (fidoInfo.has_uv ? " Leave the PIN empty to confirm with a fingerprint." : ""), { pin: true, okLabel: "Set" });
  if (!r.ok) return;
  await uvCall(() => busy(async () => { await invoke("fido_set_min_pin_len", { len, pin: r.pin || "" }); toast("Minimum PIN length set"); await refreshFidoInfo(); renderFidoSettings(); }));
});
$("#fs-min-alpha-btn").addEventListener("click", async () => {
  const len = Number($("#fs-min-alpha").value);
  const r = await confirmDialog("Set minimum alphanumeric PIN length?", `New minimum: ${len} characters. Enter the FIDO PIN to confirm.`, { pin: true, okLabel: "Set" });
  if (!r.ok || !r.pin) return;
  // goes through the Token2 OTP applet's CTAP tunnel on the same reader
  if (!t2Info) { const ok = await ensureConnected("oath"); if (!ok || !t2Info) return toast("Needs the Token2 OTP applet on this key", true); showPage("fido-settings"); }
  await busy(async () => { await invoke("t2otp_set_alpha_min_pin_len", { len, pin: r.pin }); toast("Minimum alphanumeric PIN length set"); });
});
$("#fs-always-uv").addEventListener("change", async (e) => {
  const want = e.target.checked;
  const r = await confirmDialog(want ? "Enforce verification on every use?" : "Stop enforcing verification?", "Enter the PIN to confirm." + (fidoInfo.has_uv ? " Leave it empty to confirm with a fingerprint." : ""), { pin: true, okLabel: "Apply" });
  if (!r.ok) { e.target.checked = !want; return; }
  try { await uvCall(() => busy(async () => { await invoke("fido_toggle_always_uv", { pin: r.pin || "" }); toast("Applied"); await refreshFidoInfo(); })); }
  finally { renderFidoSettings(); }
});

// ---- OTP protection page (Token2 OTP applet PIN + fingerprint)
function renderOtpProtection() {
  if (!t2Info && !oathInfo) {
    $("#op-state").textContent = "No key connected.";
    ["op-setpin","op-changepin","op-removepin","op-lock"].forEach((id) => $("#" + id).hidden = true);
    $("#op-fp-row").hidden = true; $("#op-hint").textContent = "";
    return;
  }
  if (!t2Info) { $("#op-state").textContent = "This key uses the OATH applet; protection there is the applet password (OTP menu → Accounts)."; ["op-setpin","op-changepin","op-removepin","op-lock"].forEach((id) => $("#" + id).hidden = true); $("#op-fp-row").hidden = true; return; }
  const p = t2Info.pin;
  const rev = t2Info.model && t2Info.model.revision;
  const n = (function(r){ const m=/^R(\d)(?:\.(\d))?/.exec(r||""); return m? parseInt(m[1])*10+(m[2]?parseInt(m[2]):0):0; })(rev);
  const otpPinSupported = n >= 34; // OTP PIN protection is R3.4+
  if (!otpPinSupported) {
    $("#op-state").textContent = "This key does not support OTP protection (requires R3.4 or newer).";
    ["op-setpin","op-changepin","op-removepin","op-lock"].forEach((id) => $("#" + id).hidden = true);
    $("#op-fp-row").hidden = true; $("#op-hint").textContent = "";
    return;
  }
  $("#op-state").textContent = p.set ? `OTP protection is on${p.verified ? " (unlocked)" : " (locked)"} — ${p.retries_left} attempts left.` : "OTP protection is off: anyone with the key can read its codes.";
  $("#op-setpin").hidden = p.set;
  $("#op-changepin").hidden = !p.set; $("#op-removepin").hidden = !p.set; $("#op-lock").hidden = !(p.set && p.verified);
  const fpCapable = p.supported && p.set && (t2Info.config.fingerprint || t2Info.config.mandatory_fingerprint_supported);
  $("#op-fp-row").hidden = !fpCapable;
  $("#op-fp").checked = !!p.fingerprint_enabled;
  $("#op-hint").textContent = p.set ? "Numeric PINs need 6+ digits (no sequences, repeats or palindromes); alphanumeric ones 10+ characters with two character classes. 100 wrong attempts lock the applet." : "";
}
$("#op-setpin").addEventListener("click", () => $("#btn-t2-setpin").click());
$("#op-changepin").addEventListener("click", () => $("#btn-t2-setpin").click());
$("#op-removepin").addEventListener("click", () => { $("#btn-t2-setpin").click(); $("#btn-t2-pin-remove").hidden = false; });
$("#op-lock").addEventListener("click", () => $("#btn-t2-lock").click());
$("#op-fp").addEventListener("change", (e) => { e.target.checked = !e.target.checked; $("#btn-t2-fp").click(); });

// after any T2 pin change the protection page must re-render
const _renderT2Info = renderT2Info;
renderT2Info = function () { _renderT2Info(); if (page === "otp-protection") renderOtpProtection(); if (page === "settings") renderSettingsPage(); };
const _renderFidoInfo = renderFidoInfo;
renderFidoInfo = function () { _renderFidoInfo(); loadFidoNote().catch(()=>{}); if (page === "fido-settings") renderFidoSettings(); };
let _keyName = "";
let _identCache = { usbName:"", modelName:"", revision:"", serial:"", appsStr:"" };
function prettyKeyName(n) {
  if (!n) return "";
  // trim the trailing PC/SC reader index and clean up
  return n.replace(/\s+\d+\s*$/, "").replace(/\(\d+\)\s*$/, "").trim();
}
function updateStatus() {
  const active = info ? "PIV" : fidoInfo ? "FIDO2" : t2Info ? "OTP" : oathInfo ? "OATH" : null;
  const detected = capsProbed && (caps.piv || caps.fido || caps.token2_otp || caps.oath);
  // Prefer the name of the key that actually holds the active session, so the
  // header matches the left pane (e.g. a FIDO session on a different key).
  let activeName = "";
  if (fidoInfo) { const d = fidoDevices.find((x) => x.path === fidoInfo.path); activeName = d && (d.product || d.manufacturer); }
  const name = prettyKeyName(activeName || _keyName) || (detected ? "Security key" : "");
  const serial = (t2Info && t2Info.serial) || (info && info.serial_full) || "";
  const label = serial ? `${name} · ${serial}` : name;
  if (active) $("#menubar-status").textContent = `${label || "Key"} — ${active} active`;
  else if (detected) $("#menubar-status").textContent = label || "Key detected";
  else $("#menubar-status").textContent = "No key — insert one";
}
setInterval(updateStatus, 800);

// ================= connection state machine (single, serialized) =============
// One poller reads presence() ~1s and reconciles: connect on insert, tear down
// on removal, retry transient failures with backoff. All device work goes
// through runExclusive so nothing races.
let _lock = Promise.resolve();
function runExclusive(fn) {
  const next = _lock.then(fn, fn);
  _lock = next.catch(() => {});
  return next;
}
let _lastPresence = "";
let _otpHidActive = false; // current OTP session is over the HID tunnel
let _otpHidCache = undefined; // null=probed-none, obj=found; reset on presence change
let _presentReadersCount = 0;
let _probeLog = [];
function plog(m){ _probeLog.push(`${(performance.now()/1000).toFixed(1)}s ${m}`); if(_probeLog.length>40)_probeLog.shift(); }
let _retryUntil = 0;      // suppress reconnect spam after a failed attempt
let _autoConnect = true;  // may be turned off if the user explicitly disconnects (future)

function anyConnected() { return !!(info || fidoInfo || oathInfo || t2Info); }

async function readPresence() {
  try { const p = await invoke("presence"); _keyName = p.key_name || ""; return p; }
  catch (_) { return { readers: [], fido_devices: 0, token2_reader: null }; }
}

async function teardown(reason) {
  clearInterval(oathTimer); oathTimer = null;
  try { if (info) await invoke("disconnect"); } catch (_) {}
  try { if (fidoInfo) await invoke("fido_disconnect"); } catch (_) {}
  try { if (oathInfo || t2Info) await invoke("oath_disconnect"); } catch (_) {}
  info = null; fidoInfo = null; oathInfo = null; t2Info = null;
  _otpHidActive = false;
  fidoPin = ""; fidoBioPin = ""; oathAccounts = [];
  $("#oath-unlock") && ($("#oath-unlock").hidden = true);
  $("#oath-accounts") && ($("#oath-accounts").hidden = true);
  $("#oath-pw") && ($("#oath-pw").value = "");
  caps = { piv: false, oath: false, token2_otp: false, fido: false, openpgp: false, revision: "", model: "", serial: "" };
  capsProbed = false;
  _keyName = "";
  _identCache = { usbName: "", modelName: "", revision: "", serial: "", appsStr: "" }; _noteCache = {}; { const nr = $("#di-note-row"); if (nr) nr.hidden = true; }
  _capsForKey = "";
  _lastPresence = "";
  fidoBioPin = "";
  // clear the Home device-info panel
  const hi = $("#home-info"); if (hi) hi.hidden = true;
  ["di-model","di-rev","di-serial","di-apps","di-access"].forEach((id) => { const e = $("#" + id); if (e) e.textContent = "—"; });
  { const s2 = $("#key-ident"); if (s2) s2.hidden = true; const nm = $("#ident-name"); if (nm) nm.textContent = "Security key"; const ib = $("#ident-illus"); if (ib) ib.innerHTML = ""; }
  ["key-box","fido-dev-box","t2otp-dev-box","oath-dev-box"].forEach((id) => { const e = $("#" + id); if (e) e.hidden = true; });
  ["k-serial","k-model","k-version","k-mgm","k-pin-retries","f-product","f-firmware","f-versions","f-aaguid","o-version","o-devid","t-serial","t-fido"].forEach((id) => { const e = $("#" + id); if (e) e.textContent = ""; });
  if (["piv","fido","oath","fido-settings","otp-protection","settings"].includes(page)) showPage("home"); else if (page === "home") renderHome();
  if (reason) toast(reason);
}

// FIDO connect strategy:
//  - Elevated (admin): use the standard HID CTAP path first (full features, any
//    key), fall back to the OTP-applet CCID tunnel.
//  - Non-elevated: HID FIDO is blocked by Windows, so use the CCID tunnel only.
//    A Token2 key without the OTP applet has no tunnel -> must run as admin.
let _fidoMsgShown = false;
async function connectFido(p) {
  _fidoMsgShown = false;
  // over NFC or with an external reader, token2_reader may be null; fall back to
  // reader 0. fido_connect_ccid scans all pcsc slots internally to find the card.
  const idx = p.token2_reader ? Math.max(0, p.readers.indexOf(p.token2_reader)) : (p.readers.length ? 0 : -1);
  const tryHid = async () => {
    await loadFidoDevices();
    if ($("#fido-device") && $("#fido-device").value) { try { await fidoConnect(); return !!fidoInfo; } catch (_) {} }
    return false;
  };
  let ccidErr = "";
  const tryCcid = async () => {
    if (idx < 0) return false;
    try { fidoInfo = await busy(() => invoke("fido_connect_ccid", { slot: idx })); return !!fidoInfo; } catch (e) { ccidErr = String(e); return false; }
  };

  if (isElevated) {
    if (await tryHid()) return;
    if (await tryCcid()) return;
    _fidoMsgShown = true; toast("Could not connect the FIDO applet. Try reinserting the key.", true);
    return;
  }
  // non-elevated: the CCID tunnel rides the OTP applet, which every Token2 FIDO
  // key exposes — so always attempt it on a Token2 reader regardless of the
  // capability probe (which can miss the OTP applet on a busy card).
  if (p.token2_reader || p.readers.length) {
    if (await tryCcid()) return;
    _fidoMsgShown = true;
    const nfcish = envOs === "windows" && !isElevated && /INSUFFICIENT_BUFFER|0x80100027|u2f|FIDO_ERR_INTERNAL|6a86/i.test(ccidErr || "");
    toast(nfcish
      ? "Over NFC, Windows lets only an administrator manage FIDO2 — the contactless FIDO applet is reserved for the system. Use USB for non-admin FIDO, or run the app as administrator for NFC."
      : "FIDO over the card failed: " + (ccidErr || "no response") + " — if this key has no OTP applet, FIDO management needs admin.", true);
  } else if (p.fido_devices > 0) {
    // a HID FIDO key exists but we can't reach it non-admin
    _fidoMsgShown = true;
    toast("This FIDO key is USB-HID only. On Windows it can be managed only as administrator — right-click the app and choose \"Run as administrator\".", true);
  } else {
    _fidoMsgShown = true;
    toast("No FIDO key detected — insert one.", true);
  }
}

// A Token2 smart card is single-channel: PIV, the OTP applet, and the FIDO
// tunnel (also through the OTP applet) cannot be held at once. So we open at
// most ONE applet, and switching applets disconnects the others first.
async function disconnectAllApplets() {
  clearInterval(oathTimer); oathTimer = null;
  try { if (info) await invoke("disconnect"); } catch (_) {}
  try { if (fidoInfo) await invoke("fido_disconnect"); } catch (_) {}
  try { if (oathInfo || t2Info) await invoke("oath_disconnect"); } catch (_) {}
  info = null; fidoInfo = null; oathInfo = null; t2Info = null;
  fidoPin = ""; fidoBioPin = ""; oathAccounts = [];
}

// Connect one applet. want == null means "just detect" — probe capabilities
// without opening an applet (so Home shows the right cards without holding the card).
async function connectPresent(p, want) {
  let reader = p.token2_reader || p.readers[0];
  if (reader && typeof reader !== "string") reader = String(reader.name || reader.reader || "");

  if (want == null) {
    // detect-only: probe capabilities from the reader without keeping a session.
    await refreshCaps();
    if (page === "home") renderHome(); renderIdent();
    return;
  }

  // exclusive: drop any other applet session before opening the requested one
  const already = (want === "piv" && info) || (want === "fido" && fidoInfo) || (want === "oath" && (oathInfo || t2Info));
  if (!already) await disconnectAllApplets();

  if (want === "fido") {
    await connectFido(p);
  } else if (want === "oath") {
    await oathConnect(reader).catch(() => {});
  } else if (want === "piv" && reader) {
    await connect(reader).catch(() => {});
  }
  await refreshCaps();
  if (page === "home") renderHome();
}

// Ensure a given applet is connected (used by menu actions). Returns bool.
async function ensureConnected(m) {
  const have = () => (m === "piv" && !!info) || (m === "fido" && !!fidoInfo) || (m === "oath" && !!(oathInfo || t2Info));
  if (have()) return true;
  await runExclusive(async () => { await connectPresent(await readPresence(), m); });
  return have();
}

// The poller: reconcile actual presence with our connection state.
async function poll() {
  if (_opInFlight > 0) return;      // never contend with an in-flight user operation
  await runExclusive(async () => {
    if (_opInFlight > 0) return;
    const p = await readPresence();
    const sig = p.readers.join("|") + "#" + p.fido_devices;
    const _prevReaders = _presentReadersCount;
    _presentReadersCount = p.readers.length;
    const readersAppeared = _prevReaders === 0 && p.readers.length > 0;
    const changed = sig !== _lastPresence;
    _lastPresence = sig;
    plog(`poll readers=${p.readers.length} [${p.readers.join(",")}] fido=${p.fido_devices} prev=${_prevReaders} appeared=${readersAppeared} changed=${changed}`);
    if (changed) _otpHidCache = undefined; // re-probe HID-OTP on any plug/unplug
    // Clean handover: if a key is connected but the present primary reader no longer
    // matches it (a different key was plugged), tear everything down so no stale key
    // data (name, serial, caps) lingers; the auto-connect below binds the new key.
    if (changed && (info || t2Info || oathInfo)) {
      const primary = p.token2_reader || p.readers[0] || "";
      const connectedReader = (info && info.reader) || _keyName || "";
      if (primary && connectedReader && primary !== connectedReader) {
        await teardown("Switched key");
        _lastPresence = ""; // force re-detect on the next tick
        return;
      }
    }
    if (changed && fidoInfo) {
      try {
        await loadFidoDevices();
        if (!fidoDevices.some((d) => d.path === fidoInfo.path)) {
          try { await invoke("fido_disconnect"); } catch (_) {}
          fidoInfo = null; fidoPin = ""; fidoBioPin = "";
          if (["fido","fido-settings"].includes(page)) showPage("home");
        }
        if (["fido","fido-settings"].includes(page) && fidoInfo) renderFidoInfo();
      } catch (_) {}
    }
    if (changed && page === "home") renderHome();
    // Linux: an already-plugged card can be enumerated by pcscd a few seconds
    // after launch. When the PC/SC reader list goes 0 -> N, re-probe caps so
    // PIV / OpenPGP (PC/SC-only applets) appear without a manual replug, even
    // if FIDO/OTP are already connected over HID.
    if (readersAppeared) {
      // only safe to probe the reader when no PC/SC applet session is held
      const holdingCard = !!info || !!oathInfo || (!!t2Info && !_otpHidActive) || (!!fidoInfo && fidoInfo.transport === "ccid");
      if (!holdingCard) {
        try {
          const pr = await invoke("key_capabilities", {});
          plog(`readersAppeared probe -> piv=${pr.piv} otp=${pr.token2_otp} fido=${pr.fido} openpgp=${pr.openpgp}`);
          caps.piv = pr.piv; caps.openpgp = pr.openpgp;
          caps.oath = caps.oath || pr.oath; caps.token2_otp = caps.token2_otp || pr.token2_otp;
          if (pr.revision) { caps.revision = pr.revision; caps.model = pr.model; }
          if (!caps.serial && pr.serial) caps.serial = pr.serial;
          _capsForKey = "";
          applyCaps();
          if (page === "home") renderHome();
          renderIdent();
        } catch (_) {}
      }
    }

    // A Token2 OTP session over HID has no PC/SC reader; the key is present as a
    // HID device. Only tear it down when that HID device is gone.
    if (_otpHidActive && t2Info) {
      if (changed) {
        // presence changed while a HID-OTP session is live: re-probe to see if the
        // key is still attached; only tear down if it's genuinely gone.
        let present = false;
        try { const hd = await invoke("otp_hid_detect"); present = !!(hd && hd.length); } catch (_) { present = false; }
        if (!present) { _otpHidActive = false; return teardown("Key removed"); }
      }
    } else
    // removal: something connected but its transport is gone
    if (info || oathInfo || t2Info) {
      if (p.readers.length === 0) {
        // PIV/OTP reader gone
        if (fidoInfo && p.fido_devices > 0) {
          // FIDO still there: only tear down the PC/SC side
          try { if (info) await invoke("disconnect"); } catch (_) {}
          try { if (oathInfo || t2Info) await invoke("oath_disconnect"); } catch (_) {}
          info = null; oathInfo = null; t2Info = null; oathAccounts = [];
          clearInterval(oathTimer); oathTimer = null;
          await refreshCaps(); if (page === "home") renderHome();
        } else {
          return teardown("Key removed");
        }
      }
    }
    if (fidoInfo && p.fido_devices === 0) {
      if (info || oathInfo || t2Info) {
        fidoInfo = null; fidoPin = ""; fidoBioPin = "";
        await refreshCaps(); if (page === "home") renderHome();
        if (["fido","fido-settings"].includes(page)) showPage("home");
      } else {
        return teardown("Key removed");
      }
    }

    // insertion / first connect
    if (_opInFlight === 0 && _autoConnect && !anyConnected() && (p.readers.length || p.fido_devices) && Date.now() > _retryUntil) {
      await connectPresent(p, null);
      if (!anyConnected()) _retryUntil = Date.now() + 4000; // back off after a failed attempt
      else if (page === "home") renderHome();
    } else if (_opInFlight === 0 && !anyConnected() && !p.readers.length && !p.fido_devices) {
      // no key at all: mark capabilities as known-empty so the UI greys out,
      // and clear the left-pane device info + cached key name.
      if (!capsProbed || caps.piv || caps.oath || caps.token2_otp || caps.fido || caps.openpgp || _keyName) {
        caps = { piv: false, oath: false, token2_otp: false, fido: false, openpgp: false, revision: "", model: "", serial: "" };
        capsProbed = true;
        _keyName = "";
        _identCache = { usbName: "", modelName: "", revision: "", serial: "", appsStr: "" }; _noteCache = {}; { const nr = $("#di-note-row"); if (nr) nr.hidden = true; }
        const hi = $("#home-info"); if (hi) hi.hidden = true;
        ["di-model","di-rev","di-serial","di-apps","di-access"].forEach((id) => { const e = $("#" + id); if (e) e.textContent = "—"; });
  { const s2 = $("#key-ident"); if (s2) s2.hidden = true; const nm = $("#ident-name"); if (nm) nm.textContent = "Security key"; const ib = $("#ident-illus"); if (ib) ib.innerHTML = ""; }
        applyCaps();
        if (page === "home") renderHome(); renderIdent();
      }
    }
  });
}

async function rescanAll() { _retryUntil = 0; _lastPresence = ""; await poll(); }

// Wireframe illustration of the detected key, chosen from the model name / serial.
// Generic USB-A key when the form factor is unknown. Line-art in brand colour.
function keyIllustration(model, serial) {
  const m = (model || "").toLowerCase();
  const has = (w) => m.includes(w);
  const usbc = has("usb-c") || has("type-c") || has("typec");
  const sd = String(serial || "").replace(/\D/g, "");
  // Traced product outlines (from Token2 renders). Name-based matches take priority;
  // Bio before Dual so "Bio3 Dual" shows the Bio3 shape; minis before plain USB.
  let file = null;
  if (has("nano") || has("epass")) file = "nano";            // Nano (compact block; incl. ePass)
  else if (has("card"))       file = "card";                 // FIDO smart-card
  else if (has("bio"))        file = "bio";                  // Bio3 (incl. Bio3 Dual)
  else if (has("mini"))       file = usbc ? "mini-c" : "mini-a"; // mini USB-C / USB-A
  else if (has("ace"))        file = "ace";                  // PIN+ Dual Ace
  else if (has("dual") || has("a+c")) file = "dual";         // dual-port stick
  else if (usbc)              file = "typec";                // USB-C / Type-C stick
  // If the model is unknown, fall back on the serial family: 663xxx are Nano keys.
  // Nano serial families: 663 (current), 993 (future), 363 (temporary test batch).
  if (!file) file = /^(663|993|363|633)/.test(sd) ? "nano" : "generic";
  return `<img class="key-illus" src="keys/${file}.svg" alt="${model || "security key"}">`;
}
function renderHomeInfo() { renderIdent(); }
function settingsAvail() {
  if (!caps.token2_otp) return false;
  // use the probed revision (caps.revision), same as the Settings menu gating —
  // t2Info is only set while the OTP applet is connected, which it isn't on Home.
  const rev = caps.revision || (t2Info && t2Info.model && t2Info.model.revision) || "";
  const m = /^R(\d)(?:\.(\d))?/.exec(rev); const n = m ? parseInt(m[1])*10+(m[2]?parseInt(m[2]):0) : 0;
  return n > 0 || !!caps.token2_otp; // known revision, or any Token2 OTP-applet key (unlisted prefix)
}
function renderHome() {
  const detected = capsProbed && (caps.piv || caps.fido || caps.token2_otp || caps.oath);
  const connected = detected; // cards are shown from detected capabilities
  const serial = (t2Info && t2Info.serial) || (info && info.serial_full) || "";
  const model = (t2Info && t2Info.model) || (info && info.model);
  $("#home-title").textContent = detected ? (model ? `Token2 ${model.model}` : "Security key detected") : "No key detected";
  if (detected) {
    const bits = [];
    if (caps.piv) bits.push("PIV");
    if (caps.token2_otp) bits.push("OTP");
    if (caps.oath && !caps.token2_otp) bits.push("OATH");
    if (caps.fido) bits.push("FIDO2");
    if (caps.openpgp) bits.push("OpenPGP");
    const rev = caps.revision ? ` · ${caps.revision}` : "";
    $("#home-sub").textContent = (serial ? `Serial ${serial}${rev} · ` : rev ? rev.slice(3) + " · " : "") + (bits.length ? bits.join(", ") : "no applications detected");
  } else {
    $("#home-sub").textContent = "Insert a security key — it is detected automatically.";
  }
  const cards = [
    { key: "fido", icon: "passkey", title: "FIDO2 / Passkeys", desc: "PIN, passkeys, fingerprints, reset", on: !!fidoInfo, avail: caps.fido },
    { key: "oath", icon: "otp", title: "One-time passwords", desc: "TOTP/HOTP accounts and PIN protection", on: !!(oathInfo || t2Info), avail: caps.token2_otp || caps.oath },
    { key: "piv", icon: "cert", title: "PIV (smart card)", desc: "Certificates, keys, PIN/PUK", on: !!info, avail: caps.piv },
    { key: "settings", icon: "settings", title: "Device settings", desc: "Interfaces and applets", on: false, avail: settingsAvail() },
  ];
  const box = $("#home-cards"); box.innerHTML = "";
  for (const c of cards) {
    if (capsProbed && connected && !c.avail) continue; // hide applets this key lacks
    const b = document.createElement("button");
    b.className = "app-card" + (c.on ? " on" : "");
    b.disabled = capsProbed && !c.avail;
    b.innerHTML = `<div class="ac-top">${svg(c.icon)}<h3>${c.title}</h3></div><p>${c.desc}</p><div class="ac-status">${b.disabled ? "Not available on this key" : c.on ? "Connected" : "Tap to open"}</div>`;
    b.addEventListener("click", async () => {
      if (b.disabled) return;
      let target = c.key === "settings" ? "settings" : c.key;
      if (c.key === "fido") target = "fido-settings";   // Home FIDO card opens Settings, not passkeys
      // connect the applet this card needs (exclusive) before navigating
      const appMode = (target === "settings") ? "oath" : (target === "fido-settings") ? "fido" : target;
      if (["piv", "fido", "oath"].includes(appMode)) { const ok = await ensureConnected(appMode); if (!ok) { if (!(appMode === "fido" && _fidoMsgShown)) toast("Could not connect " + appMode.toUpperCase(), true); return; } }
      showPage(target);
    });
    box.append(b);
  }
  if (connected && !box.children.length) { const p = document.createElement("p"); p.className = "hint"; p.textContent = "This key exposes no manageable applications."; box.append(p); }
}
// Full reconnect: disconnect everything, wipe every page, then re-detect from
// zero. This is the reliable recovery after a card reset (e.g. an OTP applet
// operation that resets the card, or an applet enable/disable re-enumeration).
async function reconnectAll() {
  await busy(async () => {
    await teardown();               // clears sessions, caps, panels, returns to Home
    _lastPresence = ""; _retryUntil = 0;
    await poll();                   // re-detect and auto-connect the inserted key
  });
  toast("Reconnected");
}
// keyboard: F5 / Ctrl+R reconnect
document.addEventListener("keydown", (e) => { if (e.key === "F5" || (e.ctrlKey && e.key.toLowerCase() === "r")) { e.preventDefault(); reconnectAll(); } });
document.addEventListener("keydown", async (e) => { if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "d") { e.preventDefault(); try { const d = await invoke("capabilities_debug"); let ohd = ""; try { ohd = await invoke("otp_hid_debug"); } catch (_) {} showText("Capability diagnostics", d, "capabilities.txt"); } catch (err) { toast(String(err), true); } } });
$("#btn-home").addEventListener("click", () => showPage("home"));
showPage("home");
// PC/SC service health: warn if the smart-card service isn't running (no keys otherwise)
let _pcscWarned = false;
async function checkPcsc() {
  try {
    const st = await invoke("pcsc_status");
    const w = document.getElementById("pcsc-warn");
    if (!w) return;
    if (!st.ok) {
      document.getElementById("pcsc-warn-text").textContent = st.message || "The smart-card service isn't running; keys won't be detected.";
      w.hidden = false; _pcscWarned = true;
    } else if (_pcscWarned) {
      // recovered
      w.hidden = true; _pcscWarned = false;
    }
  } catch (_) {}
}
document.getElementById("pcsc-warn-x") && document.getElementById("pcsc-warn-x").addEventListener("click", () => { const w=document.getElementById("pcsc-warn"); if (w) w.hidden = true; });
checkPcsc();
setInterval(checkPcsc, 5000);
setInterval(poll, 1000);
poll();

// Enter submits the primary action on any dialog (except in a textarea).
document.addEventListener("keydown", (e) => {
  if (e.key !== "Enter" || e.isComposing) return;
  const dlg = document.querySelector("dialog[open]");
  if (!dlg) return;
  const t = e.target;
  if (t && (t.tagName === "TEXTAREA" || (t.tagName === "BUTTON" && t.value !== "ok"))) return;
  const ok = dlg.querySelector('button[value="ok"], button.primary:not([formnovalidate])');
  if (ok && !ok.disabled) { e.preventDefault(); ok.click(); }
});


