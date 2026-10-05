#!/bin/bash

# Script to produce universal binaries for OSX by combining 2 binary sets
if [ "$#" -ne 3 ]; then
    echo "This script combines x86_64 and arm64 binaries into universal binaries for MacOS"
    echo ""
    echo "      Usage: ./make_universal_binaries.sh <path/to/x86_64_binaries> <path/to/arm64_binaries> <version number>"
    echo ""
    echo "The path to the ARM and AMD binaries are the 'pkg_ARCH' directories created by the make_release_binaries.sh script.\n"
    echo "IMPORTANT: distribution.xml file must be in the same location as this script"
    exit 0
fi

X86_64_DIR=$1 #pkg_amd
ARM64_DIR=$2 # pkg_arm
RELEASE_VERSION=$3

set -x

cp -av $ARM64_DIR pkg_universal

lipo -create -output pkg_universal/root/usr/local/bin/token2-piv-tool  $X86_64_DIR/root/usr/local/bin/token2-piv-tool $ARM64_DIR/root/usr/local/bin/token2-piv-tool

lipo -create -output pkg_universal/root/usr/local/lib/libz.1.dylib  $X86_64_DIR/root/usr/local/lib/libz.1.dylib $ARM64_DIR/root/usr/local/lib/libz.1.dylib
lipo -create -output pkg_universal/root/usr/local/lib/libcrypto.3.dylib  $X86_64_DIR/root/usr/local/lib/libcrypto.3.dylib $ARM64_DIR/root/usr/local/lib/libcrypto.3.dylib
lipo -create -output pkg_universal/root/usr/local/lib/libt2piv.$RELEASE_VERSION.dylib  $X86_64_DIR/root/usr/local/lib/libt2piv.$RELEASE_VERSION.dylib $ARM64_DIR/root/usr/local/lib/libt2piv.$RELEASE_VERSION.dylib
lipo -create -output pkg_universal/root/usr/local/lib/libt2cs11.$RELEASE_VERSION.dylib  $X86_64_DIR/root/usr/local/lib/libt2cs11.$RELEASE_VERSION.dylib $ARM64_DIR/root/usr/local/lib/libt2cs11.$RELEASE_VERSION.dylib

lipo -create -output pkg_universal/root/usr/local/lib/libt2piv.a  $X86_64_DIR/root/usr/local/lib/libt2piv.a $ARM64_DIR/root/usr/local/lib/libt2piv.a
lipo -create -output pkg_universal/root/usr/local/lib/libt2cs11.a  $X86_64_DIR/root/usr/local/lib/libt2cs11.a $ARM64_DIR/root/usr/local/lib/libt2cs11.a

ls -l pkg_universal/root/usr/local/lib
read -p "Press Enter to continue"


lipo -archs pkg_universal/root/usr/local/bin/token2-piv-tool
lipo -archs pkg_universal/root/usr/local/lib/libz.1.dylib
lipo -archs pkg_universal/root/usr/local/lib/libcrypto.3.dylib
lipo -archs pkg_universal/root/usr/local/lib/libt2piv.dylib
lipo -archs pkg_universal/root/usr/local/lib/libt2cs11.dylib
lipo -archs pkg_universal/root/usr/local/lib/libt2piv.a
lipo -archs pkg_universal/root/usr/local/lib/libt2cs11.a
read -p "Press Enter to continue"

rm pkg_universal/comp/*

pkgbuild --root=pkg_universal/root --identifier "com.yubico.token2-piv-tool" --version "$RELEASE_VERSION" pkg_universal/comp/token2-piv-tool.pkg
productbuild  --package-path pkg_universal/comp --distribution distribution.xml token2-piv-tool-$RELEASE_VERSION-mac-universal.pkg

read -p "Insert signing key then press Enter to continue"
productsign --sign 'Installer' token2-piv-tool-$RELEASE_VERSION-mac-universal.pkg token2-piv-tool-$RELEASE_VERSION-mac-universal-signed.pkg

set +x