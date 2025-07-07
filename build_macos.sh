#!/bin/bash
set -eo pipefail

# --- Configuration ---
APP_NAME="fido2-manage"
CLI_EXECUTABLE_NAME="fido2-token2"
FINAL_APP_NAME="${APP_NAME}.app"
DMG_NAME="${APP_NAME}.dmg"
VOL_NAME="FIDO2 Manager"
BUILD_DIR="build"
DIST_DIR="dist"
STAGING_DIR="${BUILD_DIR}/staging" # A clean directory for our portable binaries

# --- Helper Functions ---
info() {
    echo "[INFO] $1"
}

fatal() {
    echo "[FATAL] $1" >&2
    exit 1
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        fatal "'$1' is not installed. Please install it first for the build environment."
    fi
}

# --- Build Steps ---

# 1. Prerequisite Checks for the Build Machine
info "Checking build machine prerequisites..."
check_command "cmake"
check_command "hdiutil"
check_command "otool"
check_command "install_name_tool"
if ! command -v "brew" &> /dev/null; then
    fatal "Homebrew is not installed. It is required to fetch build dependencies."
fi
dependencies=("pkg-config" "openssl@3" "libcbor" "zlib" "python-tk")
info "Checking Homebrew dependencies..."
for dep in "${dependencies[@]}"; do
    if ! brew list "$dep" &>/dev/null; then
        info "Dependency '$dep' not found. Installing with Homebrew..."
        brew install "$dep" || fatal "Failed to install '$dep'."
    fi
done
PYTHON_EXEC="/opt/homebrew/bin/python3"

# 2. Setup Python environment and install PyInstaller
info "Setting up Python virtual environment and installing PyInstaller..."
if [ -d ".venv" ]; then rm -rf ".venv"; fi
"$PYTHON_EXEC" -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
# Let pip choose the latest compatible version of PyInstaller
pip install pyinstaller
deactivate

# 3. Build the C++ binary
info "Building the C++ binary: ${CLI_EXECUTABLE_NAME}..."
# Clean up old build artifacts to ensure a fresh build
if [ -d "$BUILD_DIR" ]; then rm -rf "$BUILD_DIR"; fi
if [ -d "$DIST_DIR" ]; then rm -rf "$DIST_DIR"; fi
if [ -f "${APP_NAME}.spec" ]; then rm -f "${APP_NAME}.spec"; fi

mkdir -p "$STAGING_DIR"
cmake -S . -B "$BUILD_DIR" -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES="arm64"
cmake --build "$BUILD_DIR" --config Release
CLI_BINARY_PATH="${BUILD_DIR}/tools/${CLI_EXECUTABLE_NAME}"
if [ ! -f "$CLI_BINARY_PATH" ]; then
    fatal "Build failed. C++ Executable not found."
fi

# 4. Make the C++ binary and its dependencies portable
info "Making the C++ binary and its libraries portable..."
if [ ! -f "./bundle_libs.sh" ]; then
    fatal "./bundle_libs.sh not found. Please create it first."
fi
chmod +x ./bundle_libs.sh
# The bundle_libs script copies the binary and its dylib dependencies into the staging folder
# and corrects their internal linkage paths.
./bundle_libs.sh "$STAGING_DIR" "$CLI_BINARY_PATH"

# 4.1 Fix library version compatibility issues
info "Fixing library version compatibility..."
cd "$STAGING_DIR"
# Create symlink for libcbor version compatibility
if [ -f "libcbor.0.12.dylib" ] && [ ! -f "libcbor.0.11.dylib" ]; then
    ln -sf libcbor.0.12.dylib libcbor.0.11.dylib
fi
cd ..

# 4.2 Fix library linking (this must be done on macOS)
info "Fixing library linking..."
if [ ! -f "./fix_macos_linking.sh" ]; then
    fatal "./fix_macos_linking.sh not found. Please run this script to create it."
fi
chmod +x ./fix_macos_linking.sh
./fix_macos_linking.sh

info "Portable binary and libraries are in ${STAGING_DIR}. Verifying linkage..."
# This check is for you to confirm it worked. All paths should start with @rpath or be system paths.
otool -L "${STAGING_DIR}/${CLI_EXECUTABLE_NAME}"

# 5. Create the standalone macOS app using PyInstaller
info "Creating the standalone .app bundle with PyInstaller..."
source .venv/bin/activate
# Bundle the Python script and include the entire staging directory as data.
# PyInstaller will place the contents of the staging directory in the root of the app bundle.
pyinstaller --name "$APP_NAME" \
            --windowed \
            --noconsole \
            --add-data "${STAGING_DIR}/*:." \
            gui-mac.py
deactivate

# 6. Package the final .app into a .dmg
info "Packaging into ${DMG_NAME}..."
APP_BUNDLE_PATH="${DIST_DIR}/${FINAL_APP_NAME}"
FINAL_DMG_PATH="${DIST_DIR}/${DMG_NAME}"

if [ -f "$FINAL_DMG_PATH" ]; then rm -f "$FINAL_DMG_PATH"; fi

hdiutil create -fs HFS+ -srcfolder "$APP_BUNDLE_PATH" -volname "$VOL_NAME" "$FINAL_DMG_PATH"

info "Process complete! The final distributable file is: ${FINAL_DMG_PATH}"
