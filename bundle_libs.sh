#!/usr/bin/env bash
#
# Standalone script to bundle Homebrew libraries with a macOS binary
# This makes the binary portable and self-contained
#
set -e

# Configuration
TARGET_DIR="$1"
BINARY_PATH="$2"

if [[ ! -d "$TARGET_DIR" ]] || [[ ! -f "$BINARY_PATH" ]]; then
    echo "Usage: $0 <target_directory> <path_to_binary>"
    echo "Example: $0 ./staging ./build/tools/fido2-token2"
    exit 1
fi

# Copy the main binary into the target directory
BINARY_NAME=$(basename "$BINARY_PATH")
cp "$BINARY_PATH" "$TARGET_DIR/"
PORTABLE_BINARY_PATH="${TARGET_DIR}/${BINARY_NAME}"

echo "--- Making ${BINARY_NAME} portable ---"
echo "Target directory: $TARGET_DIR"

# Keep track of processed libraries to avoid infinite loops
processed_libs=":"

# Function to process a single binary (executable or library)
process_binary() {
    local file_to_fix="$1"
    local depth="${2:-0}"
    local indent=$(printf "%*s" $((depth * 2)) "")
    
    echo "${indent}Processing: $(basename "$file_to_fix")"
    
    # Get the list of Homebrew/local library dependencies
    # Matches /opt/homebrew/ (Apple Silicon) and /usr/local/ (Intel/Legacy)
    local deps=$(otool -L "$file_to_fix" 2>/dev/null | grep -E '/opt/homebrew/|/usr/local/' | awk '{print $1}' | tr -d ':' | grep -v "^$file_to_fix$" || true)
    
    if [[ -z "$deps" ]]; then
        echo "${indent}  No external dependencies found"
        return
    fi
    
    while IFS= read -r lib; do
        if [[ -z "$lib" ]]; then
            continue
        fi
        
        local lib_name=$(basename "$lib")
        local new_lib_path="${TARGET_DIR}/${lib_name}"
        
        # Skip if already processed
        if echo "$processed_libs" | grep -q ":${lib_name}:"; then
            echo "${indent}  -> ${lib_name} (already processed, updating reference)"
            install_name_tool -change "$lib" "@executable_path/${lib_name}" "$file_to_fix" 2>/dev/null || {
                echo "${indent}     Warning: Could not update reference to ${lib_name}"
            }
            continue
        fi
        
        # Mark as processed
        processed_libs="${processed_libs}${lib_name}:"
        
        # Copy the library if it doesn't exist
        if [[ ! -f "$new_lib_path" ]]; then
            echo "${indent}  -> Copying ${lib_name}"
            if ! cp "$lib" "$TARGET_DIR/"; then
                echo "${indent}     Error: Could not copy ${lib}"
                continue
            fi
            chmod 755 "$new_lib_path"
        fi
        
        # Update the dependency path in the current binary
        echo "${indent}  -> Updating reference to ${lib_name}"
        install_name_tool -change "$lib" "@executable_path/${lib_name}" "$file_to_fix" 2>/dev/null || {
            echo "${indent}     Warning: Could not update reference to ${lib_name}"
        }
        
        # Set the library's own ID to be relative (only for libraries, not the main executable)
        if [[ "$new_lib_path" != "$PORTABLE_BINARY_PATH" ]]; then
            install_name_tool -id "@executable_path/${lib_name}" "$new_lib_path" 2>/dev/null || {
                echo "${indent}     Warning: Could not set ID for ${lib_name}"
            }
        fi
        
        # Recursively process this library's dependencies
        if [[ -f "$new_lib_path" ]] && [[ $depth -lt 5 ]]; then
            process_binary "$new_lib_path" $((depth + 1))
        fi
        
    done <<< "$deps"
}

# Start processing with the main executable
echo "Starting dependency analysis..."
process_binary "$PORTABLE_BINARY_PATH"

echo ""
echo "--- Bundling Summary ---"
echo "Bundled libraries:"
ls -la "$TARGET_DIR"

echo ""
echo "--- Final Verification ---"
echo "Main executable dependencies:"
otool -L "$PORTABLE_BINARY_PATH"

echo ""
echo "--- Checking for remaining external dependencies ---"
remaining_deps=$(otool -L "$PORTABLE_BINARY_PATH" | grep -E '/opt/homebrew/|/usr/local/' | grep -v '@executable_path' || true)
if [[ -n "$remaining_deps" ]]; then
    echo "WARNING: Some external dependencies remain:"
    echo "$remaining_deps"
else
    echo "SUCCESS: All external dependencies have been bundled!"
fi

echo ""
echo "--- Library ID Verification ---"
for lib in "$TARGET_DIR"/*.dylib; do
    if [[ -f "$lib" ]]; then
        lib_name=$(basename "$lib")
        lib_id=$(otool -D "$lib" | tail -n1)
        echo "${lib_name}: ${lib_id}"
    fi
done

echo ""
echo "Bundling complete for ${BINARY_NAME}"