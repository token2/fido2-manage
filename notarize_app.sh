#!/bin/bash
# Notarize the app for distribution
# Notarization is required for apps distributed outside the Mac App Store

set -e

# Configuration - ALL MUST BE CHANGED!
APPLE_ID="your-apple-id@example.com"              # Your Apple ID email
TEAM_ID="TEAMID"                                  # Your Team ID (from developer account)
APP_PASSWORD="xxxx-xxxx-xxxx-xxxx"               # App-specific password from appleid.apple.com
DMG_FILE="dist/fido2-manage.dmg"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Check configuration
if [[ "$APPLE_ID" == "your-apple-id@example.com" ]]; then
    error "Please configure APPLE_ID in this script!"
fi

if [[ "$TEAM_ID" == "TEAMID" ]]; then
    error "Please configure TEAM_ID in this script!"
fi

if [[ "$APP_PASSWORD" == "xxxx-xxxx-xxxx-xxxx" ]]; then
    error "Please configure APP_PASSWORD in this script!"
fi

# Check if DMG exists
if [[ ! -f "$DMG_FILE" ]]; then
    error "DMG not found at $DMG_FILE. Run ./create_signed_dmg.sh first!"
fi

# Store credentials in keychain (optional but recommended)
info "Setting up notarization credentials..."
xcrun notarytool store-credentials "FIDO2_MANAGER_NOTARIZE" \
    --apple-id "$APPLE_ID" \
    --team-id "$TEAM_ID" \
    --password "$APP_PASSWORD" 2>/dev/null || true

# Submit for notarization
info "Submitting DMG for notarization..."
info "This may take 5-15 minutes..."

SUBMISSION_ID=$(xcrun notarytool submit "$DMG_FILE" \
    --keychain-profile "FIDO2_MANAGER_NOTARIZE" \
    --wait 2>&1 | grep "id:" | head -1 | awk '{print $2}')

if [[ -z "$SUBMISSION_ID" ]]; then
    # Fallback to direct credentials if keychain profile fails
    info "Using direct credentials..."
    xcrun notarytool submit "$DMG_FILE" \
        --apple-id "$APPLE_ID" \
        --team-id "$TEAM_ID" \
        --password "$APP_PASSWORD" \
        --wait
else
    info "Submission ID: $SUBMISSION_ID"
fi

# Get notarization info
info "Checking notarization status..."
xcrun notarytool info "$SUBMISSION_ID" \
    --keychain-profile "FIDO2_MANAGER_NOTARIZE" 2>/dev/null || \
xcrun notarytool info "$SUBMISSION_ID" \
    --apple-id "$APPLE_ID" \
    --team-id "$TEAM_ID" \
    --password "$APP_PASSWORD"

# Staple the notarization ticket to the DMG
info "Stapling notarization ticket to DMG..."
xcrun stapler staple "$DMG_FILE" || error "Failed to staple notarization ticket"

# Verify the stapled DMG
info "Verifying notarized DMG..."
xcrun stapler validate "$DMG_FILE" || error "Validation failed"

# Final verification
info "Running final security check..."
spctl -a -t open --context context:primary-signature -v "$DMG_FILE" || error "Security check failed"

info ""
info "✅ Notarization complete!"
info ""
info "The DMG is now ready for distribution."
info "Users can download and install without security warnings."
info ""
info "Distribution checklist:"
info "[ ] Upload to GitHub Releases"
info "[ ] Update download links"
info "[ ] Test download on clean Mac"
info "[ ] Announce release"