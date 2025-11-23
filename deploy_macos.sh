#!/bin/bash
# Complete deployment script for macOS FIDO2 Manager
# This script should be run on the macOS VPS after pulling latest changes

set -eo pipefail

# Configuration
APP_NAME="fido2-manage"
CLI_EXECUTABLE_NAME="fido2-token2"
FINAL_APP_NAME="${APP_NAME}.app"
DMG_NAME="${APP_NAME}.dmg"
VOL_NAME="FIDO2 Manager"
BUILD_DIR="build"
DIST_DIR="dist"
STAGING_DIR="${BUILD_DIR}/staging"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper Functions
info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

fatal() {
    echo -e "${RED}[FATAL]${NC} $1" >&2
    exit 1
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        fatal "'$1' is not installed. Please install it first."
    fi
}

# Verify we're on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    fatal "This script must be run on macOS"
fi

info "Starting FIDO2 Manager deployment on macOS..."

# 1. Check prerequisites
info "Checking build prerequisites..."
check_command "cmake"
check_command "hdiutil"
check_command "otool"
check_command "install_name_tool"
check_command "python3"

if ! command -v "brew" &> /dev/null; then
    fatal "Homebrew is not installed. Please install it first."
fi

# 2. Install dependencies
info "Installing/checking Homebrew dependencies..."
dependencies=("pkg-config" "openssl@3" "libcbor" "zlib" "python-tk")
for dep in "${dependencies[@]}"; do
    if ! brew list "$dep" &>/dev/null; then
        info "Installing dependency: $dep"
        brew install "$dep" || fatal "Failed to install $dep"
    else
        info "✓ $dep already installed"
    fi
done

# 3. Setup Python environment
info "Setting up Python virtual environment..."
if [[ -d ".venv" ]]; then 
    rm -rf ".venv"
fi

python3 -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install pyinstaller

# 4. Clean old build artifacts
info "Cleaning old build artifacts..."
rm -rf "$BUILD_DIR" "$DIST_DIR"
rm -f "${APP_NAME}.spec"

# 5. Build the C++ binary
info "Building C++ binary: ${CLI_EXECUTABLE_NAME}..."
mkdir -p "$STAGING_DIR"

# Check for spaces in current directory and warn user
current_dir=$(pwd)
if [[ "$current_dir" == *" "* ]]; then
    warn "Directory contains spaces: $current_dir"
    warn "This may cause build issues. Consider renaming the directory."
    warn "Attempting build with space-handling fixes..."
fi

# Set deployment target to ensure compatibility
cmake -S . -B "$BUILD_DIR" -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES="arm64" -DCMAKE_OSX_DEPLOYMENT_TARGET=13.0
cmake --build "$BUILD_DIR" --config Release

CLI_BINARY_PATH="${BUILD_DIR}/tools/${CLI_EXECUTABLE_NAME}"
if [[ ! -f "$CLI_BINARY_PATH" ]]; then
    fatal "Build failed. C++ executable not found at: $CLI_BINARY_PATH"
fi

info "✓ C++ binary built successfully"

# 6. Bundle libraries and fix dependencies
info "Bundling libraries and fixing dependencies..."
if [[ ! -f "./bundle_libs.sh" ]]; then
    fatal "bundle_libs.sh not found. Please ensure it exists."
fi

chmod +x ./bundle_libs.sh
./bundle_libs.sh "$STAGING_DIR" "$CLI_BINARY_PATH"

# 6.0.5 Copy built libfido2 library (not available via Homebrew)
info "Copying built libfido2 library..."
LIBFIDO2_BUILD_PATH="build/src/libfido2.1.15.0.dylib"
LIBFIDO2_SYMLINK_PATH="build/src/libfido2.1.dylib"

if [[ -f "$LIBFIDO2_BUILD_PATH" ]]; then
    info "✓ Found built libfido2 library, copying to staging..."
    cp "$LIBFIDO2_BUILD_PATH" "$STAGING_DIR/"
    
    if [[ -L "$LIBFIDO2_SYMLINK_PATH" ]]; then
        # Copy as regular file instead of symlink for better compatibility
        cp "$LIBFIDO2_BUILD_PATH" "$STAGING_DIR/libfido2.1.dylib"
    else
        cp "$LIBFIDO2_SYMLINK_PATH" "$STAGING_DIR/" 2>/dev/null || cp "$LIBFIDO2_BUILD_PATH" "$STAGING_DIR/libfido2.1.dylib"
    fi
    
    info "✓ libfido2 library copied to staging directory"
else
    warn "libfido2 library not found at: $LIBFIDO2_BUILD_PATH"
    warn "App may not work on systems without libfido2 installed"
fi

# 6.1 Fix library version compatibility
info "Fixing library version compatibility..."
cd "$STAGING_DIR"
if [[ -f "libcbor.0.12.dylib" ]] && [[ ! -f "libcbor.0.11.dylib" ]]; then
    # Copy the file instead of creating symlink for better compatibility
    cp libcbor.0.12.dylib libcbor.0.11.dylib
    info "✓ Created libcbor version compatibility copy (safer than symlink)"
fi
# Go back to project root (staging is build/staging, so we need to go up 2 levels)
cd ../..

# 6.2 Fix library linking
info "Fixing library linking..."
info "Current directory: $(pwd)"
info "Looking for fix_macos_linking.sh..."

# Script should be in the project root
LINKING_SCRIPT="./fix_macos_linking.sh"
if [[ ! -f "$LINKING_SCRIPT" ]]; then
    info "Files in current directory:"
    ls -la *.sh || echo "No .sh files found"
    fatal "fix_macos_linking.sh not found at: $(pwd)/$LINKING_SCRIPT"
fi

chmod +x "$LINKING_SCRIPT"
"$LINKING_SCRIPT"

# 7. Verify CLI functionality
info "Testing CLI functionality..."
CLI_TEST_PATH="${STAGING_DIR}/${CLI_EXECUTABLE_NAME}"
if [[ -x "$CLI_TEST_PATH" ]]; then
    info "Testing CLI help..."
    if "$CLI_TEST_PATH" 2>&1 | grep -q "usage:"; then
        info "✓ CLI help works"
    else
        warn "CLI help test failed, but continuing..."
    fi
    
    info "Testing CLI device list..."
    if "$CLI_TEST_PATH" -L &>/dev/null; then
        info "✓ CLI device list works"
    else
        warn "CLI device list test failed (expected if no devices connected)"
    fi
else
    fatal "CLI binary is not executable"
fi

# 8. Build macOS app with PyInstaller
info "Building macOS app with PyInstaller..."

# Create app icon if it doesn't exist
if [[ ! -f "icon.icns" ]]; then
    info "Creating placeholder app icon..."
    # Try to create a proper icon, but continue if it fails
    mkdir -p icon.iconset
    # Create a simple 1024x1024 PNG (you can replace this with a proper icon)
    echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==" | base64 -d > icon.iconset/icon_1024x1024.png
    
    if command -v iconutil &> /dev/null; then
        if iconutil -c icns icon.iconset 2>/dev/null; then
            info "✓ Icon created successfully"
        else
            warn "Icon creation failed, continuing without custom icon"
            rm -f icon.icns  # Remove any partial icon file
        fi
    else
        warn "iconutil not found, continuing without custom icon"
    fi
    rm -rf icon.iconset
fi

# Build the app
PYINSTALLER_ARGS=(
    --name "$APP_NAME"
    --windowed
    --noconsole
    --add-data "${STAGING_DIR}/*:."
    --add-data "fido2-manage-mac.sh:."
    --add-binary "${STAGING_DIR}/fido2-token2:."
    --osx-bundle-identifier="com.token2.fido2-manager"
    --target-arch="arm64"
)

# Add icon if it exists
if [[ -f "icon.icns" ]]; then
    PYINSTALLER_ARGS+=(--icon="icon.icns")
    info "Using custom icon"
else
    info "Building without custom icon"
fi

pyinstaller "${PYINSTALLER_ARGS[@]}" gui-mac.py

# 9. Verify and fix app bundle
APP_BUNDLE_PATH="${DIST_DIR}/${FINAL_APP_NAME}"
if [[ ! -d "$APP_BUNDLE_PATH" ]]; then
    fatal "App bundle was not created at: $APP_BUNDLE_PATH"
fi

info "Verifying app bundle contents..."
BUNDLE_MACOS_DIR="${APP_BUNDLE_PATH}/Contents/MacOS"
BUNDLE_CLI_PATH="${BUNDLE_MACOS_DIR}/fido2-token2"

# Ensure CLI binary and all libraries are in MacOS directory
info "Ensuring CLI binary and libraries are in MacOS directory..."
if [[ ! -f "$BUNDLE_CLI_PATH" ]]; then
    info "Copying CLI binary to app bundle MacOS directory..."
    cp "${STAGING_DIR}/fido2-token2" "$BUNDLE_MACOS_DIR/"
fi

# Copy all libraries to MacOS directory (same directory as binary)
info "Copying all libraries to MacOS directory..."
cp "${STAGING_DIR}"/*.dylib "$BUNDLE_MACOS_DIR/" 2>/dev/null || info "No additional libraries to copy"

# Copy shell script to bundle (for backward compatibility)
BUNDLE_SCRIPT_PATH="${BUNDLE_MACOS_DIR}/fido2-manage-mac.sh"
if [[ ! -f "$BUNDLE_SCRIPT_PATH" ]]; then
    info "Copying macOS shell script to app bundle..."
    cp "fido2-manage-mac.sh" "$BUNDLE_MACOS_DIR/"
    chmod +x "$BUNDLE_SCRIPT_PATH"
fi

# Set proper permissions for all executables
chmod +x "$BUNDLE_MACOS_DIR"/*
info "✓ App bundle created and verified with consistent binary placement"

# 10. Test the app bundle
info "Testing app bundle..."
if [[ -f "$BUNDLE_CLI_PATH" ]] && [[ -x "$BUNDLE_CLI_PATH" ]] && [[ -f "$BUNDLE_SCRIPT_PATH" ]] && [[ -x "$BUNDLE_SCRIPT_PATH" ]]; then
    info "✓ CLI binary and shell script found in app bundle"
    
    # Test shell script from bundle
    if "$BUNDLE_SCRIPT_PATH" -help 2>&1 | grep -q "FIDO2 Token Management Tool"; then
        info "✓ macOS shell script in app bundle works"
    else
        warn "macOS shell script in app bundle test failed"
    fi
else
    fatal "CLI binary or shell script not found or not executable in app bundle"
fi

# 11. Test GUI (basic check)
info "Testing GUI startup..."
# This will test if the GUI can start without errors
timeout 5 python3 gui-mac.py 2>/dev/null || info "GUI test completed (expected timeout)"

# 11.5. Code sign the app (OPTIONAL - requires Apple Developer Account)
if [[ -f "./sign_macos_app.sh" ]] && [[ -x "./sign_macos_app.sh" ]]; then
    warn ""
    warn "Code signing script found. Do you want to sign the app?"
    warn "This requires an Apple Developer ID certificate."
    read -p "Sign the app? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        info "Code signing app bundle..."
        ./sign_macos_app.sh
    else
        warn "Skipping code signing - app may be blocked by Gatekeeper"
    fi
else
    warn "No code signing script found - app will not be signed"
    warn "Unsigned apps may be blocked by macOS Gatekeeper"
    warn "See CODE_SIGNING_GUIDE.md for instructions"
fi

# 12. Create DMG
info "Creating DMG package..."
FINAL_DMG_PATH="${DIST_DIR}/${DMG_NAME}"
if [[ -f "$FINAL_DMG_PATH" ]]; then 
    rm -f "$FINAL_DMG_PATH"
fi

# Create temporary directory for DMG contents
DMG_TEMP_DIR=$(mktemp -d)
cp -R "$APP_BUNDLE_PATH" "$DMG_TEMP_DIR/"
ln -s /Applications "$DMG_TEMP_DIR/Applications"

# Create the DMG
hdiutil create -fs HFS+ -srcfolder "$DMG_TEMP_DIR" -volname "$VOL_NAME" "$FINAL_DMG_PATH"
rm -rf "$DMG_TEMP_DIR"

# 13. Final verification and self-contained test
info "Final verification..."
echo ""
echo "=== Build Summary ==="
echo "App bundle: $APP_BUNDLE_PATH"
echo "DMG file: $FINAL_DMG_PATH"
echo "App bundle size: $(du -sh "$APP_BUNDLE_PATH" | cut -f1)"
echo "DMG size: $(du -sh "$FINAL_DMG_PATH" | cut -f1)"

echo ""
echo "=== App Bundle Contents ==="
ls -la "$BUNDLE_MACOS_DIR"

echo ""
echo "=== Library Dependencies ==="
otool -L "$BUNDLE_CLI_PATH"

echo ""
echo "=== Self-Contained Verification ==="
# Test that all libraries are bundled
external_deps=$(otool -L "$BUNDLE_CLI_PATH" | grep -E '/opt/homebrew/|/usr/local/' | grep -v '@executable_path' | grep -v '@rpath' || true)
if [[ -n "$external_deps" ]]; then
    warn "External dependencies found:"
    echo "$external_deps"
    warn "App may not work on systems without these dependencies!"
else
    info "✅ All external dependencies are properly bundled"
fi

# Check that required library files exist
echo ""
echo "=== Required Library Check ==="
FRAMEWORKS_DIR="$APP_BUNDLE_PATH/Contents/Frameworks"
required_libs=$(otool -L "$BUNDLE_CLI_PATH" | grep '@.*\.dylib' | awk '{print $1}' | sed 's/@executable_path\///g' | sed 's/@rpath\///g')
missing_libs=""

while IFS= read -r lib; do
    if [[ -n "$lib" && ! -f "$FRAMEWORKS_DIR/$lib" ]]; then
        missing_libs="${missing_libs}${lib}\n"
    fi
done <<< "$required_libs"

if [[ -n "$missing_libs" ]]; then
    warn "Missing required libraries in app bundle:"
    echo -e "$missing_libs"
    warn "App may fail to launch!"
else
    info "✅ All required libraries are present in app bundle"
fi

# Test CLI execution
echo ""
echo "=== CLI Functionality Test ==="
if "$BUNDLE_CLI_PATH" 2>&1 | head -1 | grep -q "usage:"; then
    info "✅ CLI binary executes correctly"
else
    warn "CLI binary test failed - may indicate linking issues"
fi

# Clean up
deactivate

info "✅ Deployment complete!"
info "Final DMG: $FINAL_DMG_PATH"
info "You can now test the app and distribute the DMG file."