#!/usr/bin/env bash
# rebrand.sh — turn an upstream yubico-piv-tool checkout into token2-piv-tool.
#
# Usage: ./rebrand.sh <upstream-src-dir> <output-dir>
#
# Re-runnable against any upstream release: it copies the tree, then applies
# the branding/default changes below. Yubico copyright headers and COPYING
# (BSD-2-Clause) are kept untouched — required by the license.
#
# Everything user-visible changes; the internal C API (ykpiv_*) and the
# libykpiv library name are kept so existing code linking libykpiv keeps
# working and future upstream merges stay trivial.

set -euo pipefail
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
SRC=$(cd "${1:?upstream src dir}" && pwd)
OUT=${2:?output dir}

# ---- knobs -----------------------------------------------------------------
TOOL=token2-piv-tool                       # CLI binary name
PKCS11=t2cs11                               # PKCS#11 module: libt2cs11.so / libt2cs11.dll
LIBNAME=t2piv                               # shared PIV library: libt2piv.so / libt2piv.dll (C API stays ykpiv_*)
CMVAR=token2_piv_tool                       # cmake variable prefix
READER_DEFAULT=TOKEN2                       # substring matched against PC/SC reader name
MGM_KEY_DEFAULT=865362865362865362865362865362865362865362865362
MANUFACTURER='Token2 (www.token2.com)'
TOKEN_MODEL='Token2 PIV'
ATTEST_OID='1.3.6.1.4.1.66563.3'            # Token2 PEN arc; upstream uses 1.3.6.1.4.1.41482.3
YUBI_MGM_KEY=010203040506070801020304050607080102030405060708
# ---------------------------------------------------------------------------

rm -rf "$OUT"
cp -a "$SRC" "$OUT"
cd "$OUT"
rm -rf .git

# 1. file renames
git_mv() { mv "$1" "$2"; }
git_mv tool/yubico-piv-tool.c   tool/$TOOL.c
git_mv tool/yubico-piv-tool.h2m tool/$TOOL.h2m
git_mv debian/yubico-piv-tool.install debian/$TOOL.install
git_mv debian/ykcs11.install debian/$PKCS11.install

# helper: sed over all text files except COPYING and .git
files() { grep -rIl --exclude-dir=.git --exclude=COPYING --exclude=rebrand.sh "$1" . ; }

# 2. names in build system, packaging, docs
for f in $(files 'yubico-piv-tool\|yubico_piv_tool\|ykcs11'); do
  sed -i \
    -e "s/yubico_piv_tool_/${CMVAR}_/g" \
    -e "s/yubico-piv-tool/${TOOL}/g" \
    -e "s/yubico\\\\-piv\\\\-tool/${TOOL//-/\\\\-}/g" \
    "$f"
done
# PKCS#11 module output name + package name, but keep the ykcs11/ source dir
# and the YKCS11_* macros (internal).
sed -i "s/OUTPUT_NAME libykcs11/OUTPUT_NAME lib${PKCS11}/; s/OUTPUT_NAME ykcs11)/OUTPUT_NAME ${PKCS11})/" ykcs11/CMakeLists.txt
sed -i "s/ykcs11/${PKCS11}/g" debian/$PKCS11.install debian/control 2>/dev/null || true

# 2b. PIV library output name (headers, ykpiv_* symbols and cmake target keep their names)
sed -i "s/OUTPUT_NAME libykpiv)/OUTPUT_NAME lib${LIBNAME})/; s/OUTPUT_NAME ykpiv)/OUTPUT_NAME ${LIBNAME})/" lib/CMakeLists.txt
sed -i "s/^Libs: -L\${libdir} -lykpiv/Libs: -L\${libdir} -l${LIBNAME}/; s/^Description: Yubico PIV C Library/Description: Token2 PIV C Library/" lib/ykpiv.pc.in
sed -i "s/^Libs: -L\${libdir} -lykcs11/Libs: -L\${libdir} -l${PKCS11}/; s/^Description: Yubico PIV PKCS#11 Module/Description: Token2 PIV PKCS#11 Module/" ykcs11/ykcs11.pc.in

# 2c. Token2 terminology: the PIV management key is the "Admin PIN"
sed -i 's/"Management key to use, if no value is specified key will be asked for"/"Admin PIN (PIV management key, hex) to use; asked for if no value is given"/; s/New management key to use for action set-mgm-key/New Admin PIN to use for action set-mgm-key/; s/New management key algorithm/New Admin PIN algorithm/' tool/cmdline.ggo
sed -i 's/read_pw("management key"/read_pw("Admin PIN"/; s/read_pw("new management key"/read_pw("new Admin PIN"/; s/Failed to read management key from stdin/Failed to read Admin PIN from stdin/g; s/Successfully set new management key/Successfully set new Admin PIN/; s/pin-protected management key metadata/PIN-protected Admin PIN metadata/g' tool/$TOOL.c
# drop the vendor blog link in the algorithm help text
sed -i 's|See https://www.yubico.com/blog/comparing-asymmetric-encryption-algorithms||' tool/$TOOL.c

# 3. defaults
sed -i "s/default=\"Yubikey\"/default=\"${READER_DEFAULT}\"/" tool/cmdline.ggo
for f in $(files "$YUBI_MGM_KEY"); do sed -i "s/${YUBI_MGM_KEY}/${MGM_KEY_DEFAULT}/g" "$f"; done

# 4. PKCS#11 identity
sed -i "s|#define YKCS11_MANUFACTURER \"Yubico (www.yubico.com)\"|#define YKCS11_MANUFACTURER \"${MANUFACTURER}\"|" ykcs11/ykcs11.c
sed -i "s/static const char \*token_model = \"YubiKey XXX\";/static const char *token_model = \"${TOKEN_MODEL}\";/" ykcs11/token.c
sed -i 's/"YubiKey PIV #%u"/"Token2 PIV #%u"/; s/"YubiKey PIV Slot %x"/"Token2 PIV Slot %x"/' ykcs11/token.c
# get_token_model(): drop the NEO/YK4/YK5 suffix patching, model string is fixed
python3 - <<'EOF'
import re,io
p='ykcs11/token.c'; s=open(p).read()
s=re.sub(r'(CK_RV get_token_model\(ykpiv_state \*state, CK_UTF8CHAR_PTR str, CK_ULONG len\) \{).*?\n\}\n',
 r'\1\n\n  (void)state;\n  if (strlen(token_model) > len)\n    return CKR_BUFFER_TOO_SMALL;\n  memstrcpy(str, len, token_model);\n  return CKR_OK;\n}\n', s, count=1, flags=re.S)
open(p,'w').write(s)
EOF

# 5. attestation OID arc (tool-side CSR/cert extensions)
sed -i "s|#define YKPIV_ATTESTATION_OID \"1.3.6.1.4.1.41482.3\"|#define YKPIV_ATTESTATION_OID \"${ATTEST_OID}\"|" tool/$TOOL.c
sed -i 's/"Yubico PIV Attestation Certificate"/"Token2 PIV Attestation Certificate"/g; s/"Yubico PIV X.509 Attestation"/"Token2 PIV X.509 Attestation"/; s/"Yubico PIV Attestation"/"Token2 PIV Attestation"/' tool/$TOOL.c

# 6. user-facing strings: YubiKey -> Token2 key, except copyright lines,
#    ATR/DEVTYPE identifiers and URLs
for f in $(files 'YubiKey\|Yubikey\|YubiKeys'); do
  case "$f" in *.h|*.c|*.ggo|*.h2m|*.adoc|README|NEWS|*.txt|*.in|*.pc.in|*.md) ;; *) continue;; esac
  sed -i \
    -e '/Copyright/b' -e '/YKPIV_ATR_/b' -e '/DEVTYPE_/b' -e '/yubico\.com/b' -e '/^#/b' \
    -e 's/YubiKeys/Token2 keys/g; s/YubiKey/Token2 key/g; s/Yubikey/Token2 key/g' \
    "$f"
done
# version-gate messages: the gate is on the applet's reported version, say so
sed -i 's/only supported in Token2 key version 5.7.0/only supported by applet version 5.7.0/; s/only available with Token2 keys with version number 5.7.0/only available with applet version 5.7.0/' lib/ykpiv.c lib/util.c ykcs11/token.c
sed -i 's/"Token2 key PIV Library"/"Token2 PIV Library"/' lib/util.c
# "YubiKey 4 or newer" style caveats are meaningless for Token2 keys
for f in $(files 'Token2 key 4'); do sed -i 's/ Only available on Token2 key 4 or newer//; s/ (only available on Token2 key 4)//; s/(also only available on Token2 key 4)//' "$f"; done
sed -i 's/Tool for managing Personal Identity Verification credentials on Token2 keys/Tool for managing PIV credentials on Token2 security keys/' tool/CMakeLists.txt
# ykcs11 API test expected the reader name to start with "Yubico"
sed -i 's/strncmp(reader_buf, "Yubico", 6)/strncmp(reader_buf, "TOKEN2", 6)/' lib/tests/api.c

# 6b. lowercase "yubikey" in user-facing messages
sed -i 's/Failed to connect to yubikey: %s/Failed to connect to Token2 key: %s/' tool/$TOOL.c

# 6c. serial numbers: (a) the PIV GET SERIAL value is packed BCD on Token2
#     (last 8 digits), (b) the full serial comes from the OTP applet over CCID
#     via the new ykpiv_get_serial_str(); CLI status, PKCS#11 label and the GUI
#     use it. Kept as a unified diff so it re-applies on upstream updates.
patch -p1 --no-backup-if-mismatch < "$SCRIPT_DIR/token2-serial.patch"
cp "$SCRIPT_DIR/token2-serial.patch" .

# 6e. PKCS#12 / PEM private-key import as a single library call
#     (ykpiv_util_import_key_blob), used by the GUI's "Import PFX"
cp "$SCRIPT_DIR/t2import.c" lib/
sed -i "s/^        util.c$/        util.c\n        t2import.c/" lib/CMakeLists.txt
python3 - <<'EOF2'
p='lib/ykpiv.h'; s=open(p).read()
decl = """  ykpiv_rc ykpiv_get_serial_str(ykpiv_state *state, char *buf, size_t len);

  /**
   * Token2: import a private key from a PKCS#12/PFX blob or a PEM private key
   * (optionally followed by its certificate) into a slot, in one call.
   * Returns YKPIV_AUTHENTICATION_ERROR on a wrong PKCS#12 password,
   * YKPIV_ALGORITHM_ERROR for unsupported keys, YKPIV_PARSE_ERROR for unreadable input.
   */
  ykpiv_rc ykpiv_util_import_key_blob(ykpiv_state *state, unsigned char slot,
                                      const unsigned char *data, size_t data_len, const char *password,
                                      unsigned char pin_policy, unsigned char touch_policy,
                                      int write_cert, unsigned char *algorithm_out, int *cert_written);
  /** Parse a PKCS#12/PEM blob without touching the device. */
  ykpiv_rc ykpiv_util_probe_key_blob(const unsigned char *data, size_t data_len, const char *password,
                                     unsigned char *algorithm_out, int *has_cert);
  /** OpenSSL detail for the last probe/import failure (empty if none). */
  const char *ykpiv_util_import_last_error(void);"""
s=s.replace("  ykpiv_rc ykpiv_get_serial_str(ykpiv_state *state, char *buf, size_t len);", decl, 1)
open(p,'w').write(s)
EOF2
cp "$SCRIPT_DIR/t2import.c" .

# 9. Deep rename: no Yubico-derived identifiers, file names or artifacts remain
#    (copyright headers and COPYING are kept, as the BSD-2 license requires).
#    ykpiv_* -> t2piv_*, YKPIV_* -> T2PIV_*, ykcs11 -> t2cs11, CKA_YUBICO_* ->
#    CKA_TOKEN2_*, yk_* helpers -> t2_*, yubico_version -> token2_version.
# file/dir renames first
git_mv lib/ykpiv.h            lib/t2piv.h
git_mv lib/ykpiv.c            lib/t2piv.c
git_mv lib/ykpiv.pc.in        lib/t2piv.pc.in
git_mv lib/ykpiv-config.h.in  lib/t2piv-config.h.in
git_mv ykcs11                 t2cs11
git_mv t2cs11/ykcs11.c        t2cs11/t2cs11.c
git_mv t2cs11/ykcs11.h        t2cs11/t2cs11.h
git_mv t2cs11/ykcs11.pc.in    t2cs11/t2cs11.pc.in
git_mv t2cs11/ykcs11-config.h.in t2cs11/t2cs11-config.h.in
git_mv t2cs11/pkcs11y.h       t2cs11/pkcs11t2.h
git_mv "$TOOL.wxs" "$TOOL.wxs" 2>/dev/null || true
# identifier renames in every text file except COPYING; Copyright lines untouched
for f in $(grep -rIl --exclude-dir=.git --exclude=COPYING --exclude=rebrand.sh --exclude='*.patch' --exclude='t2import.c' -e 'ykpiv\|YKPIV\|ykcs11\|YKCS11\|YUBICO\|yubico\|Yubico\|yubikey\|YubiKey\|Yubikey\|yk_\|ykrc\|pkcs11y\|YKNEO\|DEVTYPE_YK\|\bYK\b' .); do
  sed -i \
    -e '/Copyright/b' \
    -e 's/ykpiv/t2piv/g; s/YKPIV/T2PIV/g; s/Ykpiv/T2piv/g' \
    -e 's/ykcs11/t2cs11/g; s/YKCS11/T2CS11/g' \
    -e 's/pkcs11y\.h/pkcs11t2.h/g; s/PKCS11Y_H/PKCS11T2_H/g' \
    -e 's/CKA_YUBICO/CKA_TOKEN2/g; s/YUBICO_BASE_VENDOR/TOKEN2_BASE_VENDOR/g; s/YUBICO_PIV_TOOL/TOKEN2_PIV_TOOL/g; s/YUBIKEY_PIV_/TOKEN2_PIV_/g' \
    -e 's/yubico_version/token2_version/g; s/is_yubico/is_token2/g; s/yubikeypiv/token2piv/g; s/ykneomgr_check_version/t2_check_version/g' \
    -e 's/\byk_/t2_/g; s/\bykrc\b/t2rc/g; s/\bYK_/T2_/g; s/DEVTYPE_YK/DEVTYPE_T2/g; s/YKNEO/T2NEO/g' \
    -e 's/YubicoTestUnit/Token2TestUnit/g; s/YubicoTest/Token2Test/g; s/YubicoGenerated/Token2Generated/g' \
    -e 's/YUBIKEYS/TOKEN2 KEYS/g; s/YubiKEY/Token2 key/g' \
    "$f"
done
# the tool's own sources must include the renamed headers and the patch/import file too
sed -i 's/ykpiv/t2piv/g; s/YKPIV/T2PIV/g; s/\byk_/t2_/g' lib/t2import.c
# residual wording, URLs, config path and test fixtures
sed -i 's|URL: https://www.yubico.com/|URL: https://www.token2.com/|' lib/t2piv.pc.in t2cs11/t2cs11.pc.in
sed -i 's|/etc/yubico/token2piv.conf|/etc/token2/t2piv.conf|' lib/internal.c
sed -i 's/require a YubiKey to be plugged in/require a Token2 key to be plugged in/' cmake/options.cmake
sed -i 's/require a Yubikey to be connected/require a Token2 key to be connected/; s|O=yubico.com|O=token2.com|g' tool/tests/basic.sh
sed -i 's/"Yubico (www.yubico.com)"/"Token2 (www.token2.com)"/' t2cs11/tests/*.c
sed -i 's/from the yubikey/from the device/g' tool/$TOOL.c
sed -i 's/the yubikey can receive/the device can receive/' lib/internal.h
sed -i 's|/\* this is a custom sw for yubikey \*/|/* vendor-specific status word */|; s|//"YK"|// legacy device-type tag|' lib/t2piv.h
sed -i 's/to the yk applet/to the OTP applet/; s/YK5 implements/newer firmware implements/; s/Failed selecting yk application/Failed selecting OTP application/; s/mgmt\/yk application/mgmt\/OTP application/g; s/YK 5 below 5.3/firmware below 5.3/' lib/t2piv.c
sed -i 's|See YSA-2017-01 <https://www.yubico.com/support/security-advisories/ysa-2017-01/> |See the ROCA advisory (CVE-2017-15361) |' lib/util.c
sed -i 's|Please see https://yubi.co/ysa201701/ for details.|See the ROCA advisory (CVE-2017-15361).|' t2cs11/token.c
sed -i 's/The default behavior will change in a future Yubico release./The default behavior may change in a future release./' lib/util.c
sed -i "s/SCardConnect succeeded for 'Yubico Token2 key OTP+FIDO+CCID'/SCardConnect succeeded for 'Token2 key OTP+FIDO+CCID'/" lib/t2piv.c
sed -i 's|/\* Yubico vendor specific instructions \*/|/* vendor-specific instructions */|' lib/t2piv.h
sed -i 's|Software\\\\Yubico\\\\token2piv|Software\\\\Token2\\\\t2piv|' lib/internal.c
sed -i 's/"Yubico PIV key usage policy"/"Token2 PIV key usage policy"/' t2cs11/openssl_utils.c
for f in t2cs11/tests/ykcs11_*; do [ -e "$f" ] && git_mv "$f" "${f/ykcs11_/t2cs11_}"; done
sed -i 's/ykcs11_/t2cs11_/g' t2cs11/tests/CMakeLists.txt 2>/dev/null || true

# 7. README fork notice
cat > README.fork.adoc <<EOF
= ${TOOL}

Token2 build of Yubico's yubico-piv-tool (BSD-2-Clause, see COPYING). Same
code, same commands, with Token2 defaults:

* CLI binary: \`${TOOL}\` (drop-in replacement for \`yubico-piv-tool\`)
* PKCS#11 module: \`lib${PKCS11}\` (drop-in replacement for \`libykcs11\`)
* PIV library: \`lib${LIBNAME}\` — C API \`t2piv_*\` (header \`t2piv.h\`), PKCS#11 vendor attributes \`CKA_TOKEN2_*\` (header \`pkcs11t2.h\`)
* the PIV management key is called the Admin PIN, as on other Token2 tooling
* default reader filter \`-r ${READER_DEFAULT}\`
* default management key \`${MGM_KEY_DEFAULT}\`
* PKCS#11 manufacturer/model: ${MANUFACTURER} / ${TOKEN_MODEL}
* attestation extensions emitted under ${ATTEST_OID}.x (Token2 PEN 66563)

Rebuilt from upstream with rebrand.sh; Yubico's copyright notices are kept as
required by the license. Upstream: https://github.com/Yubico/yubico-piv-tool
EOF
# 8. pre-generate the gengetopt parser so Windows builds need no gengetopt
if command -v gengetopt >/dev/null; then
  ( cd tool && gengetopt --conf-parser -i cmdline.ggo --output-dir . )
fi
python3 - <<'EOF2'
p='cmake/gengetopt.cmake'; s=open(p).read()
s=s.replace('macro (find_gengetopt)\n    if (NOT GENGETOPT_EXECUTABLE)',
 'macro (find_gengetopt)\n    if (EXISTS ${CMAKE_CURRENT_SOURCE_DIR}/cmdline.c AND EXISTS ${CMAKE_CURRENT_SOURCE_DIR}/cmdline.h)\n        message (STATUS "Using pre-generated cmdline.c")\n        set (GENGETOPT_EXECUTABLE "pregenerated")\n    endif ()\n    if (NOT GENGETOPT_EXECUTABLE)')
s=s.replace('    execute_process(\n            COMMAND gengetopt',
 '    if (NOT GENGETOPT_EXECUTABLE STREQUAL "pregenerated")\n    execute_process(\n            COMMAND gengetopt')
s=s.replace('--output-dir ${CMAKE_CURRENT_SOURCE_DIR}\n    )\n', '--output-dir ${CMAKE_CURRENT_SOURCE_DIR}\n    )\n    endif ()\n')
open(p,'w').write(s)
EOF2
cp "$SCRIPT_DIR/rebrand.sh" .
echo "done: $OUT"
