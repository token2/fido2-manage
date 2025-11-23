#!/bin/bash
# Script to fix macOS library linking for fido2-token2 binary
# This script should be run on macOS with proper tools

set -e

STAGING_DIR="build/staging"
BINARY_NAME="fido2-token2"
BINARY_PATH="${STAGING_DIR}/${BINARY_NAME}"

# Also check alternative paths
if [[ ! -f "$BINARY_PATH" ]]; then
    STAGING_DIR="staging"
    BINARY_PATH="${STAGING_DIR}/${BINARY_NAME}"
fi

echo "=== Fixing macOS Library Linking ==="
echo "Target binary: $BINARY_PATH"

# Check if we're on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "ERROR: This script must be run on macOS"
    echo "Please run this script on your macOS build machine"
    exit 1
fi

# Check if required tools are available
for tool in otool install_name_tool; do
    if ! command -v "$tool" &> /dev/null; then
        echo "ERROR: $tool is required but not found"
        exit 1
    fi
done

# Check if binary exists
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "ERROR: Binary not found at $BINARY_PATH"
    exit 1
fi

echo "Current library dependencies:"
otool -L "$BINARY_PATH"

echo ""
echo "=== Fixing Library References ==="

# Fix all homebrew/local references dynamically
echo "Checking for external references..."
# capture all dependencies that look like they come from homebrew or local install
# Exclude system libraries (/usr/lib, /System/Library)
external_deps=$(otool -L "$BINARY_PATH" | grep -E '/opt/homebrew/|/usr/local/' | awk '{print $1}' || true)

if [[ -n "$external_deps" ]]; then
    echo "Found external dependencies to fix:"
    while IFS= read -r dep; do
        if [[ -n "$dep" ]]; then
            lib_name=$(basename "$dep")
            echo "  Fixing: $dep -> @executable_path/$lib_name"
            # We use || true to continue if for some reason the change fails (though it shouldn't if otool saw it)
            install_name_tool -change "$dep" "@executable_path/$lib_name" "$BINARY_PATH" || echo "WARNING: Failed to change $dep"
        fi
    done <<< "$external_deps"
else
    echo "No external dependencies found (or already fixed)."
fi

# Special handling for libfido2 if it's linked with @rpath
# This is sometimes needed if cmake setup uses RPATH
echo "Checking for @rpath/libfido2..."
if otool -L "$BINARY_PATH" | grep -q "@rpath/libfido2"; then
    echo "  Fixing @rpath/libfido2 reference..."
    install_name_tool -change "@rpath/libfido2.1.dylib" "@executable_path/libfido2.1.dylib" "$BINARY_PATH" || true
fi

# Fix library IDs for the bundled libraries
echo ""
echo "=== Fixing Library IDs ==="
for lib in "${STAGING_DIR}"/*.dylib; do
    if [[ -f "$lib" ]]; then
        lib_name=$(basename "$lib")
        echo "Setting ID for $lib_name"
        install_name_tool -id "@executable_path/$lib_name" "$lib"
    fi
done

echo ""
echo "=== Final Verification ==="
echo "Updated library dependencies:"
otool -L "$BINARY_PATH"

echo ""
echo "Checking for remaining external dependencies..."
remaining_deps=$(otool -L "$BINARY_PATH" | grep -E '/opt/homebrew/|/usr/local/' | grep -v '@executable_path' || true)
if [[ -n "$remaining_deps" ]]; then
    echo "WARNING: Some external dependencies remain:"
    echo "$remaining_deps"
    exit 1
else
    echo "SUCCESS: All external dependencies have been fixed!"
fi

echo ""
echo "Library linking fix complete!"