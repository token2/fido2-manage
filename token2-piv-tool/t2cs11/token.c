/*
 * Copyright (c) 2015-2016,2019-2020 Yubico AB
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

#include <string.h>
#include "utils.h"
#include "token.h"
#include "debug.h"
#include "objects.h"
#include "openssl_utils.h"

#include <stdbool.h>
#include "../common/util.h"

#define MIN_RSA_KEY_SIZE 1024
#define MAX_RSA_KEY_SIZE 4096
#define MIN_ECC_KEY_SIZE 256
#define MAX_ECC_KEY_SIZE 384

static const char *token_model = "Token2 PIV";

static const token_mechanism token_mechanisms[] = {
  CKM_RSA_PKCS_KEY_PAIR_GEN, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_GENERATE_KEY_PAIR},
  CKM_RSA_PKCS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_ENCRYPT | CKF_DECRYPT | CKF_SIGN | CKF_VERIFY},
  CKM_RSA_PKCS_PSS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_RSA_PKCS_OAEP, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_ENCRYPT | CKF_DECRYPT},
  CKM_RSA_X_509, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_ENCRYPT | CKF_DECRYPT | CKF_SIGN | CKF_VERIFY},
  CKM_SHA1_RSA_PKCS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_SHA256_RSA_PKCS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_SHA384_RSA_PKCS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_SHA512_RSA_PKCS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_SHA1_RSA_PKCS_PSS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_SHA256_RSA_PKCS_PSS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_SHA384_RSA_PKCS_PSS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_SHA512_RSA_PKCS_PSS, {MIN_RSA_KEY_SIZE, MAX_RSA_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY},
  CKM_EC_KEY_PAIR_GEN, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_GENERATE_KEY_PAIR | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  //CKM_ECDSA_KEY_PAIR_GEN, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_GENERATE_KEY_PAIR | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS}, //Same as CKM_EC_KEY_PAIR_GEN, deprecated in 2.11
  CKM_ECDSA, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_ECDSA_SHA1, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_ECDSA_SHA224, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_ECDSA_SHA256, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_ECDSA_SHA384, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_ECDSA_SHA512, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_SIGN | CKF_VERIFY | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_ECDH1_DERIVE, {MIN_ECC_KEY_SIZE, MAX_ECC_KEY_SIZE, CKF_HW | CKF_DERIVE | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_SHA_1, {0, 0, CKF_DIGEST},
  CKM_SHA256, {0, 0, CKF_DIGEST},
  CKM_SHA384, {0, 0, CKF_DIGEST},
  CKM_SHA512, {0, 0, CKF_DIGEST},
  CKM_EC_EDWARDS_KEY_PAIR_GEN, {255, 255, CKF_HW | CKF_GENERATE_KEY_PAIR | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_EC_MONTGOMERY_KEY_PAIR_GEN, {255, 255, CKF_HW | CKF_GENERATE_KEY_PAIR | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS},
  CKM_EDDSA, {255, 255, CKF_HW | CKF_SIGN | CKF_VERIFY | CKF_EC_F_P | CKF_EC_NAMEDCURVE | CKF_EC_UNCOMPRESS}
};

// The commented out objects below are either not supported (PIV_DATA_OBJ_BITGT) or requires authentication.
static const piv_obj_id_t token_objects[] = { // TODO: is there a way to get this from the token?
    PIV_DATA_OBJ_X509_PIV_AUTH,   // PIV authentication
    PIV_DATA_OBJ_X509_DS,         // digital signature
    PIV_DATA_OBJ_X509_KM,         // key management
    PIV_DATA_OBJ_X509_CARD_AUTH,  // card authentication
    PIV_DATA_OBJ_X509_RETIRED1,   // Retired key 1
    PIV_DATA_OBJ_X509_RETIRED2,   // Retired key 2
    PIV_DATA_OBJ_X509_RETIRED3,   // Retired key 3
    PIV_DATA_OBJ_X509_RETIRED4,   // Retired key 4
    PIV_DATA_OBJ_X509_RETIRED5,   // Retired key 5
    PIV_DATA_OBJ_X509_RETIRED6,   // Retired key 6
    PIV_DATA_OBJ_X509_RETIRED7,   // Retired key 7
    PIV_DATA_OBJ_X509_RETIRED8,   // Retired key 8
    PIV_DATA_OBJ_X509_RETIRED9,   // Retired key 9
    PIV_DATA_OBJ_X509_RETIRED10,  // Retired key 10
    PIV_DATA_OBJ_X509_RETIRED11,  // Retired key 11
    PIV_DATA_OBJ_X509_RETIRED12,  // Retired key 12
    PIV_DATA_OBJ_X509_RETIRED13,  // Retired key 13
    PIV_DATA_OBJ_X509_RETIRED14,  // Retired key 14
    PIV_DATA_OBJ_X509_RETIRED15,  // Retired key 15
    PIV_DATA_OBJ_X509_RETIRED16,  // Retired key 16
    PIV_DATA_OBJ_X509_RETIRED17,  // Retired key 17
    PIV_DATA_OBJ_X509_RETIRED18,  // Retired key 18
    PIV_DATA_OBJ_X509_RETIRED19,  // Retired key 19
    PIV_DATA_OBJ_X509_RETIRED20,  // Retired key 20
    PIV_DATA_OBJ_X509_ATTESTATION,// Attestation key
    PIV_DATA_OBJ_CCC,             // Card capability container
    PIV_DATA_OBJ_CHUI,            // Cardholder unique id
    //PIV_DATA_OBJ_CHF,           // Cardholder fingerprints
    PIV_DATA_OBJ_SEC_OBJ,         // Security object
    //PIV_DATA_OBJ_CHFI,          // Cardholder facial images
    //PIV_DATA_OBJ_PI,            // Cardholder printed information
    PIV_DATA_OBJ_DISCOVERY,       // Discovery object
    PIV_DATA_OBJ_HISTORY,         // History object
    //PIV_DATA_OBJ_IRIS_IMAGE,    // Cardholder iris images
    //PIV_DATA_OBJ_BITGT,         // Biometric information templates group template
    PIV_DATA_OBJ_SM_SIGNER,       // Secure messaging signer
    PIV_DATA_OBJ_PC_REF_DATA,     // Pairing code reference data
};

CK_RV get_token_model(t2piv_state *state, CK_UTF8CHAR_PTR str, CK_ULONG len) {

  (void)state;
  if (strlen(token_model) > len)
    return CKR_BUFFER_TOO_SMALL;
  memstrcpy(str, len, token_model);
  return CKR_OK;
}

CK_RV get_token_version(t2piv_state *state, CK_VERSION_PTR version) {

  char buf[16] = {0};
  t2piv_rc rc;

  if (version == NULL)
    return CKR_ARGUMENTS_BAD;

  if ((rc = t2piv_get_version(state, buf, sizeof(buf))) != T2PIV_OK) {
    version->major = 0;
    version->minor = 0;
    return yrc_to_rv(rc);
  }

  version->major = (buf[0] - '0');
  version->minor = (buf[2] - '0') * 10 + (buf[4] - '0');

  return CKR_OK;
}

static t2piv_rc token_serial_text(t2piv_state *state, char *out, size_t out_len) {
  uint32_t serial = 0;
  if (t2piv_get_serial_str(state, out, out_len) == T2PIV_OK) return T2PIV_OK;
  t2piv_rc rc = t2piv_get_serial(state, &serial);
  snprintf(out, out_len, "%u", serial);
  return rc;
}

CK_RV get_token_serial(t2piv_state *state, CK_CHAR_PTR str, CK_ULONG len) {

  char serial[32] = {0};
  char buf[64] = {0};

  t2piv_rc rc = token_serial_text(state, serial, sizeof(serial));

  int actual = snprintf(buf, sizeof(buf), "%s", serial);

  if(actual < 0)
    return CKR_FUNCTION_FAILED;

  if((CK_ULONG)actual >= len)
    return CKR_BUFFER_TOO_SMALL;

  memstrcpy(str, len, buf);

  return yrc_to_rv(rc);
}

CK_RV get_token_label(t2piv_state *state, CK_CHAR_PTR str, CK_ULONG len) {

  char serial[32] = {0};
  char buf[64] = {0};

  t2piv_rc rc = token_serial_text(state, serial, sizeof(serial));

  int actual = snprintf(buf, sizeof(buf), "Token2 PIV #%s", serial);

  if(actual < 0)
    return CKR_FUNCTION_FAILED;

  if((CK_ULONG)actual >= len)
    return CKR_BUFFER_TOO_SMALL;

  memstrcpy(str, len, buf);

  return yrc_to_rv(rc);
}

CK_RV get_token_mechanism_list(CK_MECHANISM_TYPE_PTR mec, CK_ULONG_PTR num) {

  if(mec) {
    if (*num < sizeof(token_mechanisms) / sizeof(token_mechanisms[0])) {
      return CKR_BUFFER_TOO_SMALL;
    }

    for (CK_ULONG i = 0; i < sizeof(token_mechanisms) / sizeof(token_mechanisms[0]); i++) {
      mec[i] = token_mechanisms[i].type;
    }
  }

  *num = sizeof(token_mechanisms) / sizeof(token_mechanisms[0]);
  return CKR_OK;
}

CK_RV get_token_mechanism_info(CK_MECHANISM_TYPE mec, CK_MECHANISM_INFO_PTR info) {

  for (CK_ULONG i = 0; i < sizeof(token_mechanisms) / sizeof(token_mechanisms[0]); i++) {
    if (token_mechanisms[i].type == mec) {
      memcpy(info, &token_mechanisms[i].info, sizeof(CK_MECHANISM_INFO));
      return CKR_OK;
    }
  }

  return CKR_MECHANISM_INVALID;
}

CK_RV get_token_object_ids(const piv_obj_id_t **obj, CK_ULONG_PTR len) {

  *obj = token_objects;
  *len = sizeof(token_objects) / sizeof(token_objects[0]);

  return CKR_OK;
}

CK_RV token_change_pin(t2piv_state *state, CK_USER_TYPE user_type, CK_UTF8CHAR_PTR pOldPin, CK_ULONG ulOldLen, CK_UTF8CHAR_PTR pNewPin, CK_ULONG ulNewLen) {
  int tries;
  t2piv_rc res;

  switch(user_type){
    case CKU_SO:{
      t2piv_mgm new_key = {0};
      new_key.len = sizeof(new_key.data);
      if(t2piv_hex_decode((const char*)pNewPin, ulNewLen, new_key.data, &new_key.len) != T2PIV_OK) {
        DBG("Failed to decode new pin");
        return CKR_PIN_INVALID;
      }
      DBG("Changing SO PIN");
      // Set new mgm key (SO PIN) with the same algorithm and touch policy the old one had
      res = t2piv_set_mgmkey3(state, new_key.data, new_key.len, T2PIV_ALGO_AUTO, T2PIV_TOUCHPOLICY_AUTO);
      if(res == T2PIV_OK) {
        t2piv_config config = {0};
        res = t2piv_util_get_config(state, &config);
        if(res == T2PIV_OK && config.mgm_type == T2PIV_CONFIG_MGM_PROTECTED) {
          res = t2piv_util_update_protected_mgm(state, &new_key);
          if(res != T2PIV_OK) {
            DBG("Failed to update pin protected management key metadata");
          }
        }
      } else {
        DBG("Failed to set new management key: %s", t2piv_strerror(res));
      }
      OPENSSL_cleanse(new_key.data, sizeof(new_key.data));
      break;
    }
    case CKU_USER:
      if(ulOldLen >= 4 && strncmp((const char*)pOldPin, "puk:", 4) == 0){
        if(ulNewLen >= 4 && strncmp((const char*)pNewPin, "pin:", 4) == 0) {
          DBG("Unblocking PIN with PUK");
          res = t2piv_unblock_pin(state, (const char*)pOldPin + 4, ulOldLen - 4, (const char*)pNewPin + 4, ulNewLen - 4, &tries);
        } else {
          DBG("Changing PUK");
          if(ulNewLen >= 4 && strncmp((const char*)pNewPin, "puk:", 4) == 0) {
            res = t2piv_change_puk(state, (const char*)pOldPin + 4, ulOldLen - 4, (const char*)pNewPin + 4, ulNewLen - 4, &tries);
          } else {
            res = t2piv_change_puk(state, (const char*)pOldPin + 4, ulOldLen - 4, (const char*)pNewPin, ulNewLen, &tries);
          }
        }
      }else{
        DBG("Changing PIN");
        res = t2piv_change_pin(state, (const char*)pOldPin, ulOldLen, (const char*)pNewPin, ulNewLen, &tries);
      }
      break;
    default:
      DBG("TODO implement other context specific pin change");
      return CKR_FUNCTION_FAILED;
  }

  switch (res) {
    case T2PIV_SIZE_ERROR:
      return CKR_PIN_LEN_RANGE;
    default:
      return yrc_to_rv(res);
  }
}

#define T2CS11_VERIFY_BIO "VERIFY_BIO"
#define T2CS11_VERIFY_NONE "VERIFY_NONE"

CK_RV token_login(t2piv_state *state, CK_USER_TYPE user, CK_UTF8CHAR_PTR pin, CK_ULONG pin_len) {

  t2piv_rc res;
  int tries = 0;

  if (pin == NULL || pin_len == 0 || strcmp(pin, T2CS11_VERIFY_BIO) == 0) {
    if ((res = t2piv_verify_bio(state, NULL, NULL, &tries, false)) != T2PIV_OK) {
      DBG("Failed to login: %s, %d tries left", t2piv_strerror(res), tries);
      return yrc_to_rv(res);
    }
  } else if(strcmp(pin, T2CS11_VERIFY_NONE) == 0) {
    DBG("Skipping PIN verification");
    return CKR_OK;
  } else if (pin_len >= T2PIV_MIN_PIN_LEN && pin_len <= T2PIV_MAX_PIN_LEN) {
    char term_pin[T2PIV_MAX_PIN_LEN + 1] = {0};

    memcpy(term_pin, pin, pin_len);
    term_pin[pin_len] = 0;

    res = t2piv_verify(state, term_pin, &tries);

    OPENSSL_cleanse(term_pin, pin_len);

    if (res != T2PIV_OK) {
      DBG("Failed to login: %s, %d tries left", t2piv_strerror(res), tries);
      return yrc_to_rv(res);
    }
  } else if(pin_len < T2PIV_MIN_MGM_KEY_LEN || pin_len > T2PIV_MAX_MGM_KEY_LEN || user != CKU_SO) {
    DBG("PIN is wrong length");
    return CKR_ARGUMENTS_BAD;
  }

  if (user == CKU_SO) {
    t2piv_config cfg = {0};

    if (pin_len >= T2PIV_MIN_MGM_KEY_LEN && pin_len <= T2PIV_MAX_MGM_KEY_LEN) {
      cfg.mgm_len = sizeof(cfg.mgm_key);
      if((res = t2piv_hex_decode((char *)pin, pin_len, cfg.mgm_key, &cfg.mgm_len)) != T2PIV_OK) {
        DBG("Failed decoding key");
        OPENSSL_cleanse(cfg.mgm_key, sizeof(cfg.mgm_key));
        return yrc_to_rv(res);
      }
    } else {
      res = t2piv_util_get_config(state, &cfg);
      if(res != T2PIV_OK) {
        DBG("Failed to get device configuration: %s", t2piv_strerror(res));
        OPENSSL_cleanse(cfg.mgm_key, sizeof(cfg.mgm_key));
        return yrc_to_rv(res);
      }

      if(cfg.mgm_type != T2PIV_CONFIG_MGM_PROTECTED) {
        DBG("Device configuration invalid, no PIN-protected MGM key available");
        OPENSSL_cleanse(cfg.mgm_key, sizeof(cfg.mgm_key));
        return CKR_USER_PIN_NOT_INITIALIZED;
      }
    }

    if((res = t2piv_authenticate2(state, cfg.mgm_key, cfg.mgm_len)) != T2PIV_OK) {
      DBG("Failed to authenticate: %s", t2piv_strerror(res));
      OPENSSL_cleanse(cfg.mgm_key, sizeof(cfg.mgm_key));

      if(res == T2PIV_AUTHENTICATION_ERROR)
        return CKR_PIN_INCORRECT;

      return yrc_to_rv(res);
    }

    OPENSSL_cleanse(cfg.mgm_key, sizeof(cfg.mgm_key));
  }

  return CKR_OK;
}

CK_RV token_generate_key(t2piv_state *state, gen_info_t *gen, CK_BYTE key, CK_BYTE_PTR cert_data, CK_ULONG_PTR cert_len) {
  // TODO: make a function in t2piv for this
  unsigned char in_data[11] = {0};
  unsigned char *in_ptr = in_data;
  unsigned char data[1024] = {0};
  unsigned char templ[] = {0, T2PIV_INS_GENERATE_ASYMMETRIC, 0, 0};
  uint8_t certdata[T2PIV_OBJ_MAX_SIZE + 16] = {0};
  size_t certdata_len = sizeof(certdata);
  unsigned long len, offs, recv_len = sizeof(data);
  char label[32] = {0};
  t2piv_rc res;
  int sw;

  switch(gen->algorithm) {
    case T2PIV_ALGO_RSA1024:
    case T2PIV_ALGO_RSA2048:
    case T2PIV_ALGO_RSA3072:
    case T2PIV_ALGO_RSA4096:
      if (!is_version_compatible(state, 4, 3, 5)) {
        DBG("On-chip RSA key generation on this Token2 key has been blocked.");
        DBG("See the ROCA advisory (CVE-2017-15361).");
        return CKR_FUNCTION_FAILED;
      }
      if (!is_version_compatible(state, 5, 7, 0) &&
         (gen->algorithm == T2PIV_ALGO_RSA3072 || gen->algorithm == T2PIV_ALGO_RSA4096)) {
        DBG("RSA3072 and RSA4096 key types are only available with applet version 5.7.0 or later");
        return CKR_FUNCTION_NOT_SUPPORTED;
      }
      break;

    case T2PIV_ALGO_ECCP256:
    case T2PIV_ALGO_ECCP384:
      break;

    case T2PIV_ALGO_ED25519:
    case T2PIV_ALGO_X25519:
      if (!is_version_compatible(state, 5, 7, 0)) {
        DBG("ED25519 and X25519 key types are only available with applet version 5.7.0 or later");
        return CKR_FUNCTION_NOT_SUPPORTED;
      }
      break;

    default:
      return CKR_FUNCTION_FAILED;
  }

  switch (gen->touch_policy) {
    case T2PIV_TOUCHPOLICY_DEFAULT:
    case T2PIV_TOUCHPOLICY_ALWAYS:
    case T2PIV_TOUCHPOLICY_CACHED:
    case T2PIV_TOUCHPOLICY_NEVER:
        break;
    default: 
        return CKR_FUNCTION_FAILED;
  }

  switch (gen->pin_policy) {
    case T2PIV_PINPOLICY_DEFAULT:
    case T2PIV_PINPOLICY_ALWAYS:
    case T2PIV_PINPOLICY_ONCE:
    case T2PIV_PINPOLICY_NEVER:
    case T2PIV_PINPOLICY_MATCH_ONCE:
    case T2PIV_PINPOLICY_MATCH_ALWAYS:
        break;
    default: 
        return CKR_FUNCTION_FAILED;
  }

  templ[3] = key;

  *in_ptr++ = 0xac;
  *in_ptr++ = 3;
  *in_ptr++ = T2PIV_ALGO_TAG;
  *in_ptr++ = 1;
  *in_ptr++ = gen->algorithm;

  if (gen->touch_policy != T2PIV_TOUCHPOLICY_DEFAULT) {
      in_data[1] += 3;
      *in_ptr++ = T2PIV_TOUCHPOLICY_TAG;
      *in_ptr++ = 0x01;
      *in_ptr++ = gen->touch_policy;
  }

  if (gen->pin_policy != T2PIV_PINPOLICY_DEFAULT) {
      in_data[1] += 3;
      *in_ptr++ = T2PIV_PINPOLICY_TAG;
      *in_ptr++ = 0x01;
      *in_ptr++ = gen->pin_policy;
  }

  if((res = t2piv_transfer_data(state, templ, in_data, in_ptr - in_data, data, &recv_len, &sw)) != T2PIV_OK) {
    return yrc_to_rv(res);
  }
  if((res = t2piv_translate_sw_ex(__FUNCTION__, sw)) != T2PIV_OK) {
    return yrc_to_rv(res);
  }

  snprintf(label, sizeof(label), "Token2 PIV Slot %x", key);

  // Create a new empty certificate for the key
  offs = 2 + get_length(data + 2, data + recv_len, &len);
  if(offs == 2)
    return CKR_DEVICE_ERROR;

  len = recv_len;
  recv_len = sizeof(data);
  CK_RV rv = do_create_empty_cert(data + offs, len - offs, gen->algorithm, label, data, &recv_len);
  if(rv != CKR_OK)
    return rv;

  if ((res = t2piv_util_write_certdata(data, recv_len, T2PIV_CERTINFO_UNCOMPRESSED, certdata, &certdata_len)) != T2PIV_OK) {
    return yrc_to_rv(res);
  }

  if(*cert_len < (CK_ULONG)certdata_len) {
    DBG("Certificate buffer too small.");
    return CKR_BUFFER_TOO_SMALL;
  }

  // Store the certificate into the token
  if ((res = t2piv_save_object(state, t2piv_util_slot_object(key), certdata, certdata_len)) != T2PIV_OK)
    return yrc_to_rv(res);

  memcpy(cert_data, data, (unsigned long) certdata_len);
  *cert_len = certdata_len;

  return CKR_OK;
}

CK_RV token_import_cert(t2piv_state *state, CK_ULONG cert_id, CK_BYTE_PTR in, CK_ULONG in_len) {

  unsigned char certdata[T2PIV_OBJ_MAX_SIZE + 16] = {0};
  size_t certdata_len = sizeof(certdata);
  CK_ULONG cert_len;
  t2piv_rc res;
  CK_RV rv;

  // Check whether or not we have a valid cert
  if ((rv = do_check_cert(in, in_len, &cert_len)) != CKR_OK) {
    DBG("Certificate not valid.");
    return rv;
  }

  if ((res = t2piv_util_write_certdata(in, cert_len, T2PIV_CERTINFO_UNCOMPRESSED, certdata, &certdata_len)) != T2PIV_OK) {
    return yrc_to_rv(res);
  }

  // Store the certificate into the token
  if ((res = t2piv_save_object(state, cert_id, certdata, certdata_len)) != T2PIV_OK)
    return yrc_to_rv(res);

  return CKR_OK;
}

CK_RV token_delete_cert(t2piv_state *state, CK_ULONG cert_id) {
  t2piv_rc res;

  if ((res = t2piv_save_object(state, cert_id, NULL, 0)) != T2PIV_OK)
    return yrc_to_rv(res);

  return CKR_OK;
}
