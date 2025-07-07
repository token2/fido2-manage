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

# Fix libcbor reference
echo "Fixing libcbor reference..."
install_name_tool -change "/opt/homebrew/opt/libcbor/lib/libcbor.0.11.dylib" "@executable_path/libcbor.0.11.dylib" "$BINARY_PATH"
install_name_tool -change "/opt/homebrew/Cellar/libcbor/0.12.0/lib/libcbor.0.11.dylib" "@executable_path/libcbor.0.11.dylib" "$BINARY_PATH"

# Fix OpenSSL reference
echo "Fixing OpenSSL reference..."
install_name_tool -change "/opt/homebrew/opt/openssl@3/lib/libcrypto.3.dylib" "@executable_path/libcrypto.3.dylib" "$BINARY_PATH"

# Fix libfido2 @rpath reference (from local build)
echo "Fixing libfido2 @rpath reference..."
install_name_tool -change "@rpath/libfido2.1.dylib" "@executable_path/libfido2.1.dylib" "$BINARY_PATH"

# Fix any other homebrew references
echo "Checking for remaining homebrew references..."
homebrew_deps=$(otool -L "$BINARY_PATH" | grep -E '/opt/homebrew/|/usr/local/' | awk '{print $1}' || true)

if [[ -n "$homebrew_deps" ]]; then
    echo "Found additional homebrew dependencies to fix:"
    while IFS= read -r dep; do
        if [[ -n "$dep" ]]; then
            lib_name=$(basename "$dep")
            echo "  Fixing: $dep -> @executable_path/$lib_name"
            install_name_tool -change "$dep" "@executable_path/$lib_name" "$BINARY_PATH"
        fi
    done <<< "$homebrew_deps"
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