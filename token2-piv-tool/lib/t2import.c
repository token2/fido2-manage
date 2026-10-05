/*
 * Token2 addition: import a private key (+ certificate) from a PKCS#12/PFX blob
 * or a PEM private key, in one library call. Built on the same OpenSSL the
 * library already links; mirrors the CLI's import-key logic.
 */
#include <string.h>
#include <openssl/pkcs12.h>
#include <openssl/pem.h>
#include <openssl/evp.h>
#include <openssl/rsa.h>
#include <openssl/ec.h>
#include <openssl/x509.h>
#include <openssl/err.h>
#include <stdio.h>
#include "t2piv.h"
#include "internal.h"
#include "../common/util.h"
#if OPENSSL_VERSION_NUMBER >= 0x30000000L
#include <openssl/provider.h>
#ifdef _WIN32
#include <windows.h>
#endif
/* PKCS#12 files exported by Windows commonly protect the certificates with
 * RC2-40-CBC, which OpenSSL 3 only offers through the "legacy" provider. Load
 * it (and keep "default") once; look for legacy.dll/.so next to the executable
 * first, then in OpenSSL's own module directory. */
static void t2_load_openssl_providers(void) {
  static int done = 0;
  if (done) return;
  done = 1;
#ifdef _WIN32
  char path[MAX_PATH] = {0};
  if (GetModuleFileNameA(NULL, path, sizeof(path)) > 0) {
    char *slash = strrchr(path, '\\');
    if (slash) { *slash = 0; OSSL_PROVIDER_set_default_search_path(NULL, path); }
  }
#endif
  OSSL_PROVIDER_load(NULL, "default");
  if (OSSL_PROVIDER_load(NULL, "legacy") == NULL) {
    /* fall back to the build-time module directory */
    OSSL_PROVIDER_set_default_search_path(NULL, NULL);
    OSSL_PROVIDER_load(NULL, "legacy");
  }
}
#else
static void t2_load_openssl_providers(void) {}
#endif

static t2piv_rc import_evp_key(t2piv_state *state, unsigned char slot, EVP_PKEY *pkey,
                               unsigned char pin_policy, unsigned char touch_policy, unsigned char *algorithm_out) {
  unsigned char algorithm = get_algorithm(pkey);
  if (algorithm == 0) return T2PIV_ALGORITHM_ERROR;
  if (algorithm_out) *algorithm_out = algorithm;

  if (T2PIV_IS_RSA(algorithm)) {
    RSA *rsa = EVP_PKEY_get1_RSA(pkey);
    unsigned char e[3] = {0}, p[256] = {0}, q[256] = {0}, dmp1[256] = {0}, dmq1[256] = {0}, iqmp[256] = {0};
    const BIGNUM *bn_e, *bn_p, *bn_q, *bn_dmp1, *bn_dmq1, *bn_iqmp;
    int element_len, len_e = sizeof(e), len_p, len_q, len_dmp1, len_dmq1, len_iqmp;
    t2piv_rc rc;
    if (!rsa) return T2PIV_ALGORITHM_ERROR;
    switch (algorithm) {
      case T2PIV_ALGO_RSA1024: element_len = 64; break;
      case T2PIV_ALGO_RSA2048: element_len = 128; break;
      case T2PIV_ALGO_RSA3072: element_len = 192; break;
      case T2PIV_ALGO_RSA4096: element_len = 256; break;
      default: RSA_free(rsa); return T2PIV_ALGORITHM_ERROR;
    }
    RSA_get0_key(rsa, NULL, &bn_e, NULL);
    RSA_get0_factors(rsa, &bn_p, &bn_q);
    RSA_get0_crt_params(rsa, &bn_dmp1, &bn_dmq1, &bn_iqmp);
    if (!set_component(e, bn_e, &len_e) || !(e[0] == 0x01 && e[1] == 0x00 && e[2] == 0x01)) {
      RSA_free(rsa); return T2PIV_ALGORITHM_ERROR; /* only e=65537 supported */
    }
    len_p = len_q = len_dmp1 = len_dmq1 = len_iqmp = element_len;
    if (!set_component(p, bn_p, &len_p) || !set_component(q, bn_q, &len_q) ||
        !set_component(dmp1, bn_dmp1, &len_dmp1) || !set_component(dmq1, bn_dmq1, &len_dmq1) ||
        !set_component(iqmp, bn_iqmp, &len_iqmp)) {
      RSA_free(rsa); return T2PIV_PARSE_ERROR;
    }
    rc = t2piv_import_private_key(state, slot, algorithm, p, len_p, q, len_q, dmp1, len_dmp1, dmq1, len_dmq1,
                                  iqmp, len_iqmp, NULL, 0, pin_policy, touch_policy);
    OPENSSL_cleanse(p, sizeof(p)); OPENSSL_cleanse(q, sizeof(q));
    OPENSSL_cleanse(dmp1, sizeof(dmp1)); OPENSSL_cleanse(dmq1, sizeof(dmq1)); OPENSSL_cleanse(iqmp, sizeof(iqmp));
    RSA_free(rsa);
    return rc;
  }
  if (T2PIV_IS_EC(algorithm)) {
    EC_KEY *ec = EVP_PKEY_get1_EC_KEY(pkey);
    unsigned char s[48] = {0};
    int element_len = algorithm == T2PIV_ALGO_ECCP384 ? 48 : 32;
    t2piv_rc rc;
    if (!ec) return T2PIV_ALGORITHM_ERROR;
    if (!set_component(s, EC_KEY_get0_private_key(ec), &element_len)) { EC_KEY_free(ec); return T2PIV_PARSE_ERROR; }
    rc = t2piv_import_private_key(state, slot, algorithm, NULL, 0, NULL, 0, NULL, 0, NULL, 0, NULL, 0,
                                  s, element_len, pin_policy, touch_policy);
    OPENSSL_cleanse(s, sizeof(s));
    EC_KEY_free(ec);
    return rc;
  }
#if (OPENSSL_VERSION_NUMBER >= 0x10100000L)
  if (T2PIV_IS_25519(algorithm)) {
    unsigned char s[48] = {0};
    size_t element_len = sizeof(s);
    t2piv_rc rc;
    if (EVP_PKEY_get_raw_private_key(pkey, s, &element_len) != 1) return T2PIV_PARSE_ERROR;
    rc = t2piv_import_private_key(state, slot, algorithm, NULL, 0, NULL, 0, NULL, 0, NULL, 0, NULL, 0,
                                  s, element_len, pin_policy, touch_policy);
    OPENSSL_cleanse(s, sizeof(s));
    return rc;
  }
#endif
  return T2PIV_ALGORITHM_ERROR;
}

static char t2_last_err[256];
const char *t2piv_util_import_last_error(void) { return t2_last_err; }
static void set_ossl_err(const char *what) {
  unsigned long e = ERR_peek_last_error();
  char b[160] = {0};
  if (e) ERR_error_string_n(e, b, sizeof(b));
  snprintf(t2_last_err, sizeof(t2_last_err), "%s%s%s", what, e ? ": " : "", b);
}

/* Parse a PKCS#12 / PEM blob into key + cert. Tries the given password, then
 * empty and NULL (files exported with no password differ in how they encode
 * that). */
static t2piv_rc t2_parse_blob(const unsigned char *data, size_t data_len, const char *password,
                              EVP_PKEY **pkey_out, X509 **cert_out) {
  EVP_PKEY *pkey = NULL;
  X509 *cert = NULL;
  PKCS12 *p12 = NULL;
  BIO *bio = NULL;
  t2piv_rc rc = T2PIV_PARSE_ERROR;
  t2_load_openssl_providers();
  t2_last_err[0] = 0;
  bio = BIO_new_mem_buf(data, (int)data_len);
  if (!bio) return T2PIV_MEMORY_ERROR;
  if (data_len > 5 && memcmp(data, "-----", 5) == 0) {
    pkey = PEM_read_bio_PrivateKey(bio, NULL, NULL, (void *)(password ? password : ""));
    if (!pkey) { set_ossl_err("PEM private key"); rc = T2PIV_PARSE_ERROR; goto out; }
    BIO_reset(bio);
    cert = PEM_read_bio_X509(bio, NULL, NULL, NULL);
    ERR_clear_error();
  } else {
    p12 = d2i_PKCS12_bio(bio, NULL);
    if (!p12) { set_ossl_err("not a DER PKCS#12 file"); rc = T2PIV_PARSE_ERROR; goto out; }
    const char *tries[3] = { password ? password : "", "", NULL };
    int ok = 0;
    for (int i = 0; i < 3 && !ok; i++) {
      ERR_clear_error();
      if (PKCS12_parse(p12, tries[i], &pkey, &cert, NULL) == 1 && pkey) ok = 1;
      else { if (pkey) { EVP_PKEY_free(pkey); pkey = NULL; } if (cert) { X509_free(cert); cert = NULL; } }
    }
    if (!ok) { set_ossl_err("PKCS12_parse"); rc = T2PIV_AUTHENTICATION_ERROR; goto out; }
  }
  *pkey_out = pkey; *cert_out = cert; pkey = NULL; cert = NULL;
  rc = T2PIV_OK;
out:
  if (pkey) EVP_PKEY_free(pkey);
  if (cert) X509_free(cert);
  return rc;
}

/* Parse only: reports the key algorithm and whether a certificate is bundled. */
t2piv_rc t2piv_util_probe_key_blob(const unsigned char *data, size_t data_len, const char *password,
                                   unsigned char *algorithm_out, int *has_cert) {
  EVP_PKEY *pkey = NULL;
  X509 *cert = NULL;
  t2piv_rc rc = t2_parse_blob(data, data_len, password, &pkey, &cert);
  if (rc != T2PIV_OK) return rc;
  if (algorithm_out) *algorithm_out = get_algorithm(pkey);
  if (has_cert) *has_cert = cert != NULL;
  if (algorithm_out && *algorithm_out == 0) { snprintf(t2_last_err, sizeof(t2_last_err), "unsupported key type"); rc = T2PIV_ALGORITHM_ERROR; }
  EVP_PKEY_free(pkey);
  if (cert) X509_free(cert);
  return rc;
}

t2piv_rc t2piv_util_import_key_blob(t2piv_state *state, unsigned char slot,
                                    const unsigned char *data, size_t data_len, const char *password,
                                    unsigned char pin_policy, unsigned char touch_policy,
                                    int write_cert, unsigned char *algorithm_out, int *cert_written) {
  EVP_PKEY *pkey = NULL;
  X509 *cert = NULL;
  t2piv_rc rc = T2PIV_GENERIC_ERROR;

  if (!state || !data || data_len == 0) return T2PIV_ARGUMENT_ERROR;
  if (cert_written) *cert_written = 0;

  rc = t2_parse_blob(data, data_len, password, &pkey, &cert);
  if (rc != T2PIV_OK) goto out;

  rc = import_evp_key(state, slot, pkey, pin_policy, touch_policy, algorithm_out);
  if (rc != T2PIV_OK) goto out;

  if (write_cert && cert) {
    unsigned char *der = NULL;
    int der_len = i2d_X509(cert, &der);
    if (der_len > 0) {
      rc = t2piv_util_write_cert(state, slot, der, (size_t)der_len, T2PIV_CERTINFO_UNCOMPRESSED);
      if (rc == T2PIV_OK && cert_written) *cert_written = 1;
      OPENSSL_free(der);
    }
  }

out:
  if (pkey) EVP_PKEY_free(pkey);
  if (cert) X509_free(cert);
  return rc;
}
