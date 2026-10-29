# Building fido2-manage / Token2 Key Manager on Linux

Build from source on Debian/Ubuntu. This links against your system `libcbor`,
which avoids the cross-CPU codegen issue seen with the prebuilt binaries.

## 1. Dependencies

```bash
sudo apt update
sudo apt install -y build-essential cmake git pkg-config \
  libssl-dev libpcsclite-dev pcscd libudev-dev zlib1g-dev libcbor-dev \
  libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

Rust (if you don't have it):

```bash
curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
```

## 2. Clone the repo

```bash
git clone https://github.com/token2/fido2-manage
cd fido2-manage
REPO="$PWD"
```

> For the exact source of a release instead of `main`, add `--branch 0.2.0`
> (or the tag you want) to the `git clone` above.

## 3. Build libt2piv (PIV library)

```bash
cmake -S token2-piv-tool -B /tmp/t2build -DCMAKE_BUILD_TYPE=Release -DGENERATE_MAN_PAGES=OFF
cmake --build /tmp/t2build --target t2piv_shared -j"$(nproc)"
```

## 4. Build libfido2 1.15.0 with the Token2 CCID patch (static)

```bash
rm -rf /tmp/libfido2
git clone --depth 1 --branch 1.15.0 https://github.com/Yubico/libfido2 /tmp/libfido2
( cd /tmp/libfido2 && git apply --ignore-whitespace "$REPO/libfido2-token2.patch" )
cmake -S /tmp/libfido2 -B /tmp/libfido2/build \
  -DBUILD_TOOLS=OFF -DBUILD_EXAMPLES=OFF -DUSE_PCSC=ON \
  -DBUILD_STATIC_LIBS=ON -DBUILD_SHARED_LIBS=OFF
cmake --build /tmp/libfido2/build --target fido2 -j"$(nproc)"
```

## 5. Build the binaries

```bash
export T2PIV_LIB_DIR=/tmp/t2build/lib FIDO2_LIB_DIR=/tmp/libfido2/build/src \
       FIDO2_LINK_STATIC=1 FIDO2_LIB_NAME=fido2
cd token2-piv-gui/src-tauri
mkdir -p libs && touch libs/.keep
cargo build --release --bin fido2-manage --bin token2-key-manager
```

The binaries land in `token2-piv-gui/src-tauri/target/release/`:
`fido2-manage` and `token2-key-manager`.

## 6. Run

Keep `libt2piv.so` (from `/tmp/t2build/lib`) next to the binaries or on
`LD_LIBRARY_PATH`:

```bash
cd token2-piv-gui/src-tauri/target/release
cp /tmp/t2build/lib/libt2piv.so* .
LD_LIBRARY_PATH=. ./fido2-manage -info
LD_LIBRARY_PATH=. ./token2-key-manager
```

`./fido2-manage -info` on a plugged-in key should show the full FIDO2 details
(firmware, versions, AAGUID, PIN state).
