#!/usr/bin/env bash
#
# sign-notarize-macos.sh — codesign + notarize + staple the macOS build.
#
# Two modes:
#
#   App / DMG (default) — signs the GUI .app, wraps it in a DMG, notarizes, staples:
#     ./sign-notarize-macos.sh "/path/to/Token2 Key Manager.app"
#     ./sign-notarize-macos.sh "/path/to/token2-key-manager-macos"   # folder containing the .app
#     ./sign-notarize-macos.sh "/path/to/Token2-Key-Manager.dmg"     # we extract the .app from it
#
#   Portable CLI binaries — signs every Mach-O in a folder inside-out and notarizes
#   the loose binaries (no DMG; they stay a plain folder):
#     ./sign-notarize-macos.sh --portable "/path/to/portable-folder"
#   Add --dmg to ALSO build a stapled Token2-Key-Manager-CLI.dmg (verifies offline):
#     ./sign-notarize-macos.sh --portable --dmg "/path/to/portable-folder"
#
# Prerequisites (one-time):
#   1. A "Developer ID Application" certificate in your login keychain.
#      (NOT "Developer ID Installer" — that one only signs .pkg files.)
#      Check with:  security find-identity -v -p codesigning
#      If you only have the Installer cert, create an Application cert at
#      https://developer.apple.com/account/resources/certificates  ->  "+"
#      ->  "Developer ID Application".
#   2. A stored notary profile, created once with:
#      xcrun notarytool store-credentials "notary-profile" \
#        --apple-id "you@example.com" --team-id "BWF2GDTZ48" --password "<app-specific-password>"
#
# Env overrides:
#   NOTARY_PROFILE   notary keychain profile name     (default: notary-profile)
#   OUT_DMG          output DMG filename              (defaults per mode)
#
set -euo pipefail

TEAM_ID="BWF2GDTZ48"
NOTARY_PROFILE="${NOTARY_PROFILE:-notary-profile}"

say() { printf "\033[1;36m==>\033[0m %s\n" "$*"; }
die() { printf "\033[1;31merror:\033[0m %s\n" "$*" >&2; exit 1; }

# ---- parse mode ----
MODE="app"
PORTABLE_DMG=0
while [ $# -gt 0 ]; do
  case "$1" in
    --portable) MODE="portable"; shift ;;
    --dmg)      PORTABLE_DMG=1;   shift ;;   # portable mode: also build a stapled DMG
    --)         shift; break ;;
    -*)         die "unknown option: $1" ;;
    *)          break ;;
  esac
done
[ $# -ge 1 ] || die "usage:
  $0 <path-to-.app | folder | .dmg>          # sign + notarize + staple the GUI app, in a DMG
  $0 --portable <folder>                     # sign + notarize the loose CLI binaries (no DMG)
  $0 --portable --dmg <folder>               # ... and also build a stapled Token2-Key-Manager-CLI.dmg"
INPUT="$1"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ---- find the Developer ID *Application* identity (not Installer) ----
say "Looking for a Developer ID Application certificate…"
IDLINE="$(security find-identity -v -p codesigning \
  | grep "Developer ID Application" | grep "$TEAM_ID" | head -1 || true)"
[ -n "$IDLINE" ] || die "No 'Developer ID Application' cert for team $TEAM_ID in your keychain.
       You may have an Installer cert (that signs .pkg only). Create an Application
       cert at https://developer.apple.com/account/resources/certificates"
ID="$(echo "$IDLINE" | sed -E 's/^[^"]*"([^"]+)".*/\1/')"
say "Signing identity: $ID"

# ---- entitlements (Tauri/WebKit + loading our own dylibs under hardened runtime;
#      harmless on the CLI binaries, needed if a GUI binary is present) ----
ENT="$WORK/entitlements.plist"
cat > "$ENT" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.cs.allow-jit</key><true/>
  <key>com.apple.security.cs.allow-unsigned-executable-memory</key><true/>
  <key>com.apple.security.cs.disable-library-validation</key><true/>
</dict>
</plist>
PLIST

# ---- helper: sign every Mach-O under a path, inside-out (deepest files first) ----
sign_machos() {
  local root="$1"
  # reverse-depth order so nested dylibs are signed before the things that load them
  /usr/bin/find "$root" -type f -print | /usr/bin/awk '{ print gsub(/\//,"/"), $0 }' \
    | sort -rn | cut -d' ' -f2- | while IFS= read -r f; do
    if file "$f" | grep -q "Mach-O"; then
      codesign --force --timestamp --options runtime \
        --entitlements "$ENT" --sign "$ID" "$f"
    fi
  done
}

# ---- helper: notarize loose binaries via a throwaway zip (can't staple these) ----
notarize_loose() {
  local dir="$1"
  local zip="$WORK/notarize-payload.zip"
  say "Zipping the folder for notary submission…"
  ( cd "$dir" && /usr/bin/zip -qr "$zip" . )
  say "Submitting to Apple notary service (this can take a few minutes)…"
  xcrun notarytool submit "$zip" --keychain-profile "$NOTARY_PROFILE" --wait
  rm -f "$zip"
  say "Notarized. Verifying the signed binaries:"
  for probe in fido2-manage token2-piv-tool token2-key-manager; do
    [ -e "$dir/$probe" ] && codesign --verify --strict --verbose=2 "$dir/$probe"
  done
}

# ---- helper: build a DMG from a staging dir, sign it, notarize, staple, verify ----
build_sign_notarize_staple() {
  local srcdir="$1" volname="$2" out_dmg="$3"
  say "Building DMG…"
  rm -f "$out_dmg"
  hdiutil create -volname "$volname" -srcfolder "$srcdir" -ov -format UDZO "$out_dmg" >/dev/null
  say "Signing DMG…"
  codesign --force --timestamp --sign "$ID" "$out_dmg"
  say "Submitting to Apple notary service (this can take a few minutes)…"
  xcrun notarytool submit "$out_dmg" --keychain-profile "$NOTARY_PROFILE" --wait
  say "Stapling the ticket…"
  xcrun stapler staple "$out_dmg"
  xcrun stapler validate "$out_dmg"
  say "Gatekeeper assessment:"
  spctl -a -t open --context context:primary-signature -vv "$out_dmg" || true
  printf "\n\033[1;32mDone.\033[0m Notarized, stapled DMG: %s\n" "$out_dmg"
}

# =====================================================================
if [ "$MODE" = "portable" ]; then
  # ------------------------- PORTABLE CLI MODE -------------------------
  DIR="$INPUT"
  [ -d "$DIR" ] || die "not a folder: $DIR"
  OUT_DMG="${OUT_DMG:-Token2-Key-Manager-CLI.dmg}"
  VOLNAME="Token2 Key Manager CLI"
  say "Portable folder: $DIR"

  say "Clearing extended attributes…"
  xattr -cr "$DIR" 2>/dev/null || true

  say "Signing every Mach-O in the folder (dylibs first, then executables)…"
  sign_machos "$DIR"

  # sanity-check the binaries we expect (fails early if a signature didn't take)
  for probe in fido2-manage token2-piv-tool token2-key-manager; do
    [ -e "$DIR/$probe" ] && codesign --verify --strict --verbose=2 "$DIR/$probe"
  done

  # notarize the loose binaries (no DMG) — ticket can't be stapled to bare files,
  # so these pass Gatekeeper when the Mac is online.
  notarize_loose "$DIR"
  printf "\n\033[1;32mDone.\033[0m Signed + notarized binaries in: %s\n" "$DIR"
  printf "   (Loose binaries can't be stapled, so an offline first-run may be blocked.\n"
  printf "    For an offline-verifiable, stapled artifact, re-run with --dmg.)\n"

  if [ "$PORTABLE_DMG" = "1" ]; then
    STAGE="$WORK/dmgroot"; mkdir -p "$STAGE"
    cp -R "$DIR" "$STAGE/"
    build_sign_notarize_staple "$STAGE" "$VOLNAME" "$OUT_DMG"
  fi
  exit 0
fi

# ------------------------- APP / DMG MODE (default) -------------------------
OUT_DMG="${OUT_DMG:-Token2-Key-Manager.dmg}"
VOLNAME="Token2 Key Manager"

# ---- locate the .app from whatever was passed in ----
APP=""
case "$INPUT" in
  *.app)
    APP="$INPUT" ;;
  *.dmg)
    say "Mounting DMG to extract the .app…"
    MNT="$(hdiutil attach -nobrowse -readonly "$INPUT" | awk -F'\t' 'END{print $NF}')"
    found="$(/usr/bin/find "$MNT" -maxdepth 2 -name '*.app' -print -quit)"
    [ -n "$found" ] || { hdiutil detach "$MNT" >/dev/null; die "no .app inside the DMG"; }
    cp -R "$found" "$WORK/"
    hdiutil detach "$MNT" >/dev/null
    APP="$WORK/$(basename "$found")" ;;
  *)
    # a folder (e.g. the unzipped artifact) — find the .app inside
    found="$(/usr/bin/find "$INPUT" -maxdepth 3 -name '*.app' -print -quit)"
    [ -n "$found" ] || die "no .app found under: $INPUT"
    APP="$found" ;;
esac
[ -d "$APP" ] || die "not a bundle: $APP"
say "App bundle: $APP"

# ---- remove quarantine + stray extended attributes (avoid 'resource fork' errors) ----
say "Clearing extended attributes…"
xattr -cr "$APP" 2>/dev/null || true

# ---- sign inside-out: nested dylibs + helper binaries first, bundle last ----
say "Signing nested dylibs and CLI tools…"
sign_machos "$APP/Contents"

say "Signing the .app bundle…"
codesign --force --timestamp --options runtime \
  --entitlements "$ENT" --sign "$ID" "$APP"

say "Verifying signature…"
codesign --verify --deep --strict --verbose=2 "$APP"

# ---- build DMG around the signed app (+ Applications symlink) ----
STAGE="$WORK/dmgroot"; mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications" 2>/dev/null || true
build_sign_notarize_staple "$STAGE" "$VOLNAME" "$OUT_DMG"
