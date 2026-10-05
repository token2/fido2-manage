#!/usr/bin/env bash
#
# sign-notarize-macos.sh — codesign + notarize + staple the macOS build.
#
# Run this on your Mac against the artifact downloaded from the "Build macOS"
# CI run (the token2-key-manager-macos zip). It signs the .app with your
# Developer ID Application certificate, (re)builds a DMG around the signed app,
# notarizes it with Apple, and staples the ticket.
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
# Usage:
#   ./sign-notarize-macos.sh "/path/to/Token2 Key Manager.app"
#   ./sign-notarize-macos.sh "/path/to/token2-key-manager-macos"   # a folder containing the .app
#   ./sign-notarize-macos.sh "/path/to/Token2-Key-Manager.dmg"     # we extract the .app from it
#
set -euo pipefail

TEAM_ID="BWF2GDTZ48"
NOTARY_PROFILE="${NOTARY_PROFILE:-notary-profile}"
VOLNAME="Token2 Key Manager"
OUT_DMG="${OUT_DMG:-Token2-Key-Manager.dmg}"

say() { printf "\033[1;36m==>\033[0m %s\n" "$*"; }
die() { printf "\033[1;31merror:\033[0m %s\n" "$*" >&2; exit 1; }

[ $# -ge 1 ] || die "usage: $0 <path-to-.app | folder-with-.app | .dmg>"
INPUT="$1"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ---- 1. locate the .app from whatever was passed in ----
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

# ---- 2. find the Developer ID *Application* identity (not Installer) ----
say "Looking for a Developer ID Application certificate…"
IDLINE="$(security find-identity -v -p codesigning \
  | grep "Developer ID Application" | grep "$TEAM_ID" | head -1 || true)"
[ -n "$IDLINE" ] || die "No 'Developer ID Application' cert for team $TEAM_ID in your keychain.
       You have an Installer cert (that signs .pkg only). Create an Application
       cert at https://developer.apple.com/account/resources/certificates"
# the identity is the quoted name on that line
ID="$(echo "$IDLINE" | sed -E 's/^[^"]*"([^"]+)".*/\1/')"
say "Signing identity: $ID"

# ---- 3. remove quarantine + stray extended attributes (cause 'resource fork' codesign errors) ----
say "Clearing extended attributes…"
xattr -cr "$APP" 2>/dev/null || true

# ---- 4. entitlements (Tauri/WebKit + loading our own dylibs under hardened runtime) ----
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

# ---- 5. sign inside-out: nested dylibs + helper binaries first, bundle last ----
say "Signing nested dylibs and CLI tools…"
# every Mach-O file under the bundle except the main executable
/usr/bin/find "$APP/Contents" -type f | while IFS= read -r f; do
  # is it a Mach-O (dylib or executable)?
  if file "$f" | grep -q "Mach-O"; then
    codesign --force --timestamp --options runtime \
      --entitlements "$ENT" --sign "$ID" "$f" >/dev/null 2>&1 || \
      codesign --force --timestamp --options runtime \
        --entitlements "$ENT" --sign "$ID" "$f"
  fi
done

say "Signing the .app bundle…"
codesign --force --timestamp --options runtime \
  --entitlements "$ENT" --sign "$ID" "$APP"

say "Verifying signature…"
codesign --verify --deep --strict --verbose=2 "$APP"

# ---- 6. (re)build a DMG around the signed app ----
say "Building DMG…"
STAGE="$WORK/dmgroot"; mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications" 2>/dev/null || true
rm -f "$OUT_DMG"
hdiutil create -volname "$VOLNAME" -srcfolder "$STAGE" -ov -format UDZO "$OUT_DMG" >/dev/null
say "Signing DMG…"
codesign --force --timestamp --sign "$ID" "$OUT_DMG"

# ---- 7. notarize + staple ----
say "Submitting to Apple notary service (this can take a few minutes)…"
xcrun notarytool submit "$OUT_DMG" --keychain-profile "$NOTARY_PROFILE" --wait

say "Stapling the ticket…"
xcrun stapler staple "$OUT_DMG"
xcrun stapler validate "$OUT_DMG"

say "Gatekeeper assessment:"
spctl -a -t open --context context:primary-signature -vv "$OUT_DMG" || true

printf "\n\033[1;32mDone.\033[0m Notarized, stapled DMG: %s\n" "$OUT_DMG"
