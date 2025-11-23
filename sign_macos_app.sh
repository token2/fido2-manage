#!/bin/bash
# Code signing script for FIDO2 Manager
# This script must be run AFTER building but BEFORE creating the DMG

set -e

# Configuration - MUST BE CHANGED!
DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)" # <-- CHANGE THIS!
APP_BUNDLE="dist/fido2-manage.app"
ENTITLEMENTS_FILE="entitlements.plist"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Check if developer ID is set
if [[ "$DEVELOPER_ID" == "Developer ID Application: Your Name (TEAMID)" ]]; then
    error "Please set DEVELOPER_ID in this script first!"
fi

# Check if app bundle exists
if [[ ! -d "$APP_BUNDLE" ]]; then
    error "App bundle not found at $APP_BUNDLE. Run deploy_macos.sh first!"
fi

# Create entitlements file if it doesn't exist
if [[ ! -f "$ENTITLEMENTS_FILE" ]]; then
    info "Creating entitlements file..."
    cat > "$ENTITLEMENTS_FILE" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Allow app to be run -->
    <key>com.apple.security.app-sandbox</key>
    <false/>
    
    <!-- Allow USB device access for FIDO2 devices -->
    <key>com.apple.security.device.usb</key>
    <true/>
    
    <!-- Allow smartcard access -->
    <key>com.apple.security.smartcard</key>
    <true/>
    
    <!-- Allow execution of unsigned code (for PyInstaller) -->
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    
    <!-- Allow DYLD environment variables -->
    <key>com.apple.security.cs.allow-dyld-environment-variables</key>
    <true/>
    
    <!-- Disable library validation -->
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
</dict>
</plist>
EOF
fi

info "Starting code signing process..."
info "Using identity: $DEVELOPER_ID"

# Step 1: Sign all dynamic libraries
info "Signing dynamic libraries..."
find "$APP_BUNDLE" -name "*.dylib" -type f | while read -r lib; do
    info "  Signing: $(basename "$lib")"
    codesign --force --sign "$DEVELOPER_ID" \
        --timestamp \
        --options runtime \
        "$lib" || error "Failed to sign $(basename "$lib")"
done

# Step 2: Sign the fido2-token2 binary
info "Signing fido2-token2 executables..."
find "$APP_BUNDLE" -name "fido2-token2" -type f | while read -r binary; do
    info "  Signing: $binary"
    codesign --force --sign "$DEVELOPER_ID" \
        --timestamp \
        --options runtime \
        --entitlements "$ENTITLEMENTS_FILE" \
        "$binary" || error "Failed to sign fido2-token2"
done

# Step 3: Sign shell scripts (optional but recommended)
info "Signing shell scripts..."
find "$APP_BUNDLE" -name "*.sh" -type f | while read -r script; do
    info "  Signing: $(basename "$script")"
    codesign --force --sign "$DEVELOPER_ID" \
        --timestamp \
        "$script" || warn "Failed to sign $(basename "$script") - continuing anyway"
done

# Step 4: Sign Python/PyInstaller files
info "Signing Python components..."
find "$APP_BUNDLE" -name "*.so" -type f | while read -r lib; do
    info "  Signing: $(basename "$lib")"
    codesign --force --sign "$DEVELOPER_ID" \
        --timestamp \
        --options runtime \
        "$lib" || error "Failed to sign $(basename "$lib")"
done

# Step 5: Sign the main executable
info "Signing main executable..."
main_exec="$APP_BUNDLE/Contents/MacOS/fido2-manage"
if [[ -f "$main_exec" ]]; then
    codesign --force --sign "$DEVELOPER_ID" \
        --timestamp \
        --options runtime \
        --entitlements "$ENTITLEMENTS_FILE" \
        "$main_exec" || error "Failed to sign main executable"
fi

# Step 6: Sign the entire app bundle
info "Signing app bundle..."
codesign --force --deep --sign "$DEVELOPER_ID" \
    --timestamp \
    --options runtime \
    --entitlements "$ENTITLEMENTS_FILE" \
    "$APP_BUNDLE" || error "Failed to sign app bundle"

# Verify the signature
info "Verifying signature..."
if codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"; then
    info "✅ App bundle signed successfully!"
else
    error "❌ Signature verification failed!"
fi

# Check signature details
info ""
info "Signature details:"
codesign -dvv "$APP_BUNDLE" 2>&1 | grep -E 'Authority|TeamIdentifier|Timestamp'

# Verify entitlements
info ""
info "Entitlements summary:"
codesign -d --entitlements - "$APP_BUNDLE" 2>&1 | grep -E 'security\.(usb|smartcard|cs\.)' || true

info ""
info "✅ Code signing complete!"
info ""
info "Next steps:"
info "1. Test the app locally: open $APP_BUNDLE"
info "2. Create signed DMG: ./create_signed_dmg.sh"
info "3. Notarize the DMG for distribution: ./notarize_app.sh"