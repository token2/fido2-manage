/*
 * Copyright (c) 2014-2020 Yubico AB
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are
 * met:
 *
 *   * Redistributions of source code must retain the above copyright
 *     notice, this list of conditions and the following disclaimer.
 *
 *   * Redistributions in binary form must reproduce the above
 *     copyright notice, this list of conditions and the following
 *     disclaimer in the documentation and/or other materials provided
 *     with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 * A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
 * OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
 * LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 */

#ifndef T2PIV_INTERNAL_H
#define T2PIV_INTERNAL_H

#include "t2piv-config.h"
#include "t2piv.h"

#include <stdbool.h>

#ifdef BACKEND_PCSC
#ifdef HAVE_PCSC_WINSCARD_H
# include <PCSC/wintypes.h>
# include <PCSC/winscard.h>
#else
# include <winscard.h>
#endif
#endif

// Typedef DWORD (defined by pcsc lib) to pcsc_word to make it clear that this
// is not the Windows meaning of DWORD, but the PCSC library's meaning.  This
// differs: Windows defines a DWORD as 32-bits, but pcsclite defines it as
// 'unsigned long' on x86_64 Linux, which is often 64-bits.
typedef DWORD pcsc_word;
typedef LONG pcsc_long;

#ifdef __cplusplus
extern "C"
{
#endif

#define DES_LEN_DES   8
#define DES_LEN_3DES  DES_LEN_DES*3

#define READER_LEN  32
#define MAX_READERS 16

// the object size is restricted to the firmware's message buffer size, which
// always contains 0x5C + 1 byte len + 3 byte id + 0x53 + 3 byte len = 9 bytes,
// so while the message buffer == CB_BUF_MAX, the maximum object we can store
// is CB_BUF_MAX - 9
#define CB_OBJ_MAX_NEO      (CB_BUF_MAX_NEO - 9)
#define CB_OBJ_MAX_YK4      (CB_BUF_MAX_YK4 - 9)
#define CB_OBJ_MAX          CB_OBJ_MAX_YK4

#define CB_BUF_MAX_NEO      2048
#define CB_BUF_MAX_YK4      3072
#define CB_BUF_MAX          CB_BUF_MAX_YK4

#define CB_ATR_MAX          33

#define CHREF_ACT_CHANGE_PIN 0
#define CHREF_ACT_UNBLOCK_PIN 1
#define CHREF_ACT_CHANGE_PUK 2

#define TAG_ADMIN             0x80
#define TAG_ADMIN_FLAGS_1     0x81
#define TAG_ADMIN_SALT        0x82
#define TAG_ADMIN_TIMESTAMP   0x83
#define TAG_PROTECTED         0x88
#define TAG_PROTECTED_FLAGS_1 0x81
#define TAG_PROTECTED_MGM     0x89
#define TAG_MSCMAP            0x81
#define TAG_MSROOTS_END       0x82
#define TAG_MSROOTS_MID       0x83

#define TAG_RSA_MODULUS       0x81
#define TAG_RSA_EXP           0x82
#define TAG_ECC_POINT         0x86

#define CB_ECC_POINTP256    65
#define CB_ECC_POINTP384    97
#define CB_ECC_POINT25519   32

#define T2PIV_OBJ_ADMIN_DATA 0x5fff00
#define T2PIV_OBJ_ATTESTATION 0x5fff01
#define	T2PIV_OBJ_MSCMAP      0x5fff10
#define	T2PIV_OBJ_MSROOTS1    0x5fff11
#define T2PIV_OBJ_MSROOTS2    0x5fff12
#define T2PIV_OBJ_MSROOTS3    0x5fff13
#define T2PIV_OBJ_MSROOTS4    0x5fff14
#define T2PIV_OBJ_MSROOTS5    0x5fff15

#define ADMIN_FLAGS_1_PUK_BLOCKED    0x01
#define ADMIN_FLAGS_1_PROTECTED_MGM  0x02

#define CB_ADMIN_SALT         16
#define CB_ADMIN_TIMESTAMP    4

#define ITER_MGM_PBKDF2       10000

#define PROTECTED_FLAGS_1_PUK_NOBLOCK 0x01

#define CB_OBJ_TAG_MIN      2                       // 1 byte tag + 1 byte len
#define CB_OBJ_TAG_MAX      (CB_OBJ_TAG_MIN + 2)      // 1 byte tag + 3 bytes len

#define CB_PIN_MAX          8

#define SCP11_SESSION_KEY_LEN 16
#define SCP11_MAC_LEN 16
#define SCP11_HALF_MAC_LEN 8

#define SCP11B_KID 0x13
#define SCP11B_KVN 0x1
#define SCP11_KEY_USAGE 0x3c
#define SCP11_KEY_TYPE 0x88
#define SCP11_CERTIFICATE_STORE_TAG 0xBF21
#define SCP11_ePK_SD_ECKA_TAG 0x5F49

typedef enum {
    PKCS5_OK = 0,
    PKCS5_GENERAL_ERROR = -1
} pkcs5_rc;

typedef enum {
  PRNG_OK = 0,
  PRNG_GENERAL_ERROR = -1
} prng_rc;

typedef struct _t2piv_version_t {
  uint8_t major;
  uint8_t minor;
  uint8_t patch;
} t2piv_version_t;

typedef struct _t2piv_scp11_state {
  uint8_t security_level;
  uint32_t enc_counter;
  uint8_t senc[SCP11_SESSION_KEY_LEN];
  uint8_t smac[SCP11_SESSION_KEY_LEN];
  uint8_t srmac[SCP11_SESSION_KEY_LEN];
  uint8_t mac_chain[SCP11_MAC_LEN];
} t2piv_scp11_state;

struct t2piv_state {
  SCARDCONTEXT context;
  SCARDHANDLE card;
  pcsc_word protocol;
  char reader[2048];
  int tries;
  char *pin;
  uint8_t *mgm_key;
  uint32_t mgm_len;
  t2piv_allocator allocator;
  uint32_t model;
  t2piv_version_t ver;
  uint32_t serial;
  char serial_str[32];   /* Token2: full serial from the OTP applet (ASCII digits) */
  t2piv_scp11_state scp11_state;
};

union u_APDU {
  struct {
    unsigned char cla;
    unsigned char ins;
    unsigned char p1;
    unsigned char p2;
    unsigned char lc;
    unsigned char data[T2PIV_OBJ_MAX_SIZE - 5]; // Max message bytes - first bytes in apdu - Le
  } st;
  unsigned char raw[T2PIV_OBJ_MAX_SIZE]; // Max message size the device can receive
};

typedef union u_APDU APDU;

pkcs5_rc pkcs5_pbkdf2_sha1(const uint8_t* password, const size_t cb_password, const uint8_t* salt, const size_t cb_salt, uint64_t iterations, const uint8_t* key, const size_t cb_key);
bool   t2_des_is_weak_key(const unsigned char *key, const size_t cb_key);
unsigned char* hash_sha256(const unsigned char* data, size_t count, unsigned char* md_buf);

prng_rc _t2piv_prng_generate(unsigned char *buffer, const size_t cb_req);
t2piv_rc _t2piv_begin_transaction(t2piv_state *state);
t2piv_rc _t2piv_end_transaction(t2piv_state *state);
t2piv_rc _t2piv_ensure_application_selected(t2piv_state *state, bool scp11);
t2piv_rc _t2piv_select_application(t2piv_state *state, bool scp11);
size_t _t2piv_get_length_size(size_t length);
size_t _t2piv_set_length(unsigned char *buffer, size_t length);
size_t _t2piv_get_length(const unsigned char *buffer, const unsigned char* end, size_t *len);
t2piv_rc scp11_open_secure_channel(t2piv_state* state);

void* _t2piv_alloc(t2piv_state *state, size_t size);
void* _t2piv_realloc(t2piv_state *state, void *address, size_t size);
void _t2piv_free(t2piv_state *state, void *data);
t2piv_rc _t2piv_save_object(t2piv_state *state, int object_id, unsigned char *indata, size_t len);
t2piv_rc _t2piv_fetch_object(t2piv_state *state, int object_id, unsigned char *data, unsigned long *len);
t2piv_rc _t2piv_send_apdu(t2piv_state *state, APDU *apdu, unsigned char *data, unsigned long *recv_len, int *sw);
t2piv_rc _t2piv_transfer_data(
    t2piv_state *state,
    const unsigned char *templ,
    const unsigned char *in_data,
    unsigned long in_len,
    unsigned char *out_data,
    unsigned long *out_len,
    int *sw);

/* authentication functions not ready for public api */
t2piv_rc t2piv_auth_getchallenge(t2piv_state *state, t2piv_metadata *metadata, uint8_t *challenge, unsigned long *challenge_len);
t2piv_rc t2piv_auth_verifyresponse(t2piv_state *state, t2piv_metadata *metadata, uint8_t *response, unsigned long response_len);
t2piv_rc t2piv_auth_deauthenticate(t2piv_state *state);
t2piv_rc t2piv_auth_get_verified(t2piv_state *state);
t2piv_rc t2piv_auth_verify(t2piv_state* state, uint8_t* pin, size_t* p_pin_len, int *tries, bool force_select, bool bio, bool verify_spin);

typedef enum _setting_source_t {
  SETTING_SOURCE_USER,
  SETTING_SOURCE_ADMIN,
  SETTING_SOURCE_DEFAULT
} setting_source_t;

typedef struct _setting_bool_t {
  bool value;
  setting_source_t source;
} setting_bool_t;

setting_bool_t setting_get_bool(const char *sz_setting, bool f_default);

typedef enum _yc_log_level_t {
  YC_LOG_LEVEL_ERROR,
  YC_LOG_LEVEL_WARN,
  YC_LOG_LEVEL_INFO,
  YC_LOG_LEVEL_VERBOSE,
  YC_LOG_LEVEL_DEBUG
} yc_log_level_t;

void yc_log_event(const char *sz_source, uint32_t id, yc_log_level_t level, const char *sz_format, ...);

void _t2piv_set_debug(void (*dbg)(const char *));
void _t2piv_debug(const char *file, int line, const char *func, int lvl, const char *fmt, ...);

#define DBG(fmt, ...) _t2piv_debug(__FILE__, __LINE__, __FUNCTION__, 1, fmt, ##__VA_ARGS__)
#define DBG2(fmt, ...) _t2piv_debug(__FILE__, __LINE__, __FUNCTION__, 2, fmt, ##__VA_ARGS__)
#define DBG3(fmt, ...) _t2piv_debug(__FILE__, __LINE__, __FUNCTION__, 3, fmt, ##__VA_ARGS__)

#ifdef _WIN32
//#include <windows.h>
#define yc_memzero SecureZeroMemory
#elif defined(HAVE_EXPLICIT_BZERO)
#include <strings.h>
#define yc_memzero explicit_bzero
#elif defined(__linux__)
#include <openssl/crypto.h>
#define yc_memzero OPENSSL_cleanse
#else
#define __STDC_WANT_LIB_EXT1__ 1
#include <string.h>
#define yc_memzero(_p, _n) (void)memset_s(_p, (rsize_t)_n, 0, (rsize_t)_n)
#endif

#ifdef __cplusplus
}
#endif

#endif
