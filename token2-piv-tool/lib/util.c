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

#include <string.h>
#include <stdio.h>
#include <time.h>
 
#ifdef USE_CERT_COMPRESS
#include <zlib.h>
#endif

#include "internal.h"
#include "t2piv.h"

#define MAX(a,b) (a) > (b) ? (a) : (b)
#define MIN(a,b) (a) < (b) ? (a) : (b)

/*
 * Format defined in SP-800-73-4, Appendix A, Table 9
 *
 * FASC-N containing S9999F9999F999999F0F1F0000000000300001E encoded in
 * 4-bit BCD with 1 bit parity. run through the tools/fasc.pl script to get
 * bytes. This CHUID has an expiry of 2030-01-01.
 *
 * Defined fields:
 *  - 0x30: FASC-N (hard-coded)
 *  - 0x34: Card UUID / GUID (settable)
 *  - 0x35: Exp. Date (hard-coded)
 *  - 0x3e: Signature (hard-coded, empty)
 *  - 0xfe: Error Detection Code (hard-coded)
 */
const uint8_t CHUID_TMPL[] = {
  0x30, 0x19, 0xd4, 0xe7, 0x39, 0xda, 0x73, 0x9c, 0xed, 0x39, 0xce, 0x73, 0x9d,
  0x83, 0x68, 0x58, 0x21, 0x08, 0x42, 0x10, 0x84, 0x21, 0xc8, 0x42, 0x10, 0xc3,
  0xeb, 0x34, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x35, 0x08, 0x32, 0x30, 0x33, 0x30, 0x30,
  0x31, 0x30, 0x31, 0x3e, 0x00, 0xfe, 0x00,
};
#define CHUID_GUID_OFFS 29
#define TAG_CHUID_UUID 0x34

// f0: Card Identifier
//  - 0xa000000116 == GSC-IS RID
//  - 0xff == Manufacturer ID (dummy)
//  - 0x02 == Card type (javaCard)
//  - next 14 bytes: card ID
const uint8_t CCC_TMPL[] = {
  0xf0, 0x15, 0xa0, 0x00, 0x00, 0x01, 0x16, 0xff, 0x02, 0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf1, 0x01, 0x21,
  0xf2, 0x01, 0x21, 0xf3, 0x00, 0xf4, 0x01, 0x00, 0xf5, 0x01, 0x10, 0xf6, 0x00,
  0xf7, 0x00, 0xfa, 0x00, 0xfb, 0x00, 0xfc, 0x00, 0xfd, 0x00, 0xfe, 0x00
};
#define CCC_ID_OFFS 9

static t2piv_rc _read_certificate(t2piv_state *state, uint8_t slot, uint8_t *buf, size_t *buf_len);
static t2piv_rc _write_certificate(t2piv_state *state, uint8_t slot, uint8_t *data, size_t data_len, uint8_t certinfo);

static t2piv_rc _read_metadata(t2piv_state *state, uint8_t tag, uint8_t* data, size_t* pcb_data);
static t2piv_rc _write_metadata(t2piv_state *state, uint8_t tag, uint8_t *data, size_t cb_data);
static t2piv_rc _get_metadata_item(uint8_t *data, size_t cb_data, uint8_t tag, uint8_t **pp_item, size_t *pcb_item);
static t2piv_rc _set_metadata_item(uint8_t *data, size_t *pcb_data, size_t cb_data_max, uint8_t tag, uint8_t *p_item, size_t cb_item);

static size_t _obj_size_max(t2piv_state *state) {
  return (state && state->model == DEVTYPE_NEOr3) ? CB_OBJ_MAX_NEO : CB_OBJ_MAX;
}

static unsigned long get_length_size(unsigned long length) {
  if (length < 0x80) {
    return 1;
  }
  else if (length < 0x100) {
    return 2;
  }
  else {
    return 3;
  }
}

/*
** T2PIV Utility API - aggregate functions and slightly nicer interface
*/

t2piv_rc t2piv_util_get_cardid(t2piv_state *state, t2piv_cardid *cardid) {
  t2piv_rc res = T2PIV_OK;
  uint8_t buf[CB_OBJ_MAX] = {0};
  unsigned long len = sizeof(buf);
  uint8_t *p_temp = NULL;
  size_t offs, cb_temp = 0;
  uint8_t tag = 0;

  if (!cardid) return T2PIV_ARGUMENT_ERROR;
   uint8_t scp11 = state->scp11_state.security_level;

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  if ((res = _t2piv_fetch_object(state, T2PIV_OBJ_CHUID, buf, &len)) == T2PIV_OK) {
    p_temp = buf;

    while (p_temp < (buf + len)) {
      tag = *p_temp++;

      offs = _t2piv_get_length(p_temp, buf + len, &cb_temp);
      if (!offs) {
        res = T2PIV_PARSE_ERROR;
        goto Cleanup;
      }

      p_temp += offs;

      if (tag == TAG_CHUID_UUID) {
        /* found card uuid */
        if (cb_temp < T2PIV_CARDID_SIZE || p_temp + T2PIV_CARDID_SIZE > buf + len) {
          res = T2PIV_SIZE_ERROR;
          goto Cleanup;
        }

        res = T2PIV_OK;
        memcpy(cardid->data, p_temp, T2PIV_CARDID_SIZE);
        goto Cleanup;
      }

      p_temp += cb_temp;
    }

    /* not found, not malformed */
    res = T2PIV_GENERIC_ERROR;
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_set_cardid(t2piv_state *state, const t2piv_cardid *cardid) {
  t2piv_rc res = T2PIV_OK;
  uint8_t id[T2PIV_CARDID_SIZE] = {0};
  uint8_t buf[sizeof(CHUID_TMPL)] = {0};
  size_t len = 0;

  if (!state) return T2PIV_ARGUMENT_ERROR;
  uint8_t scp11 = state->scp11_state.security_level;

  if (!cardid) {
    if (PRNG_OK != _t2piv_prng_generate(id, sizeof(id))) {
      return T2PIV_RANDOMNESS_ERROR;
    }
  }
  else {
    memcpy(id, cardid->data, sizeof(id));
  }

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  memcpy(buf, CHUID_TMPL, sizeof(CHUID_TMPL));
  memcpy(buf + CHUID_GUID_OFFS, id, sizeof(id));
  len = sizeof(CHUID_TMPL);

  res = _t2piv_save_object(state, T2PIV_OBJ_CHUID, buf, len);

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_get_cccid(t2piv_state *state, t2piv_cccid *ccc) {
  t2piv_rc res = T2PIV_OK;
  uint8_t buf[CB_OBJ_MAX] = {0};
  unsigned long len = sizeof(buf);

  if (!ccc) return T2PIV_ARGUMENT_ERROR;

  uint8_t scp11 = state->scp11_state.security_level;
  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  res = _t2piv_fetch_object(state, T2PIV_OBJ_CAPABILITY, buf, &len);
  if (T2PIV_OK == res) {
    if (len != sizeof(CCC_TMPL)) {
      res = T2PIV_GENERIC_ERROR;
    }
    else {
      memcpy(ccc->data, buf + CCC_ID_OFFS, T2PIV_CCCID_SIZE);
    }
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_set_cccid(t2piv_state *state, const t2piv_cccid *ccc) {
  t2piv_rc res = T2PIV_OK;
  uint8_t id[T2PIV_CCCID_SIZE] = {0};
  uint8_t buf[sizeof(CCC_TMPL)] = {0};
  size_t len = 0;

  if (!state) return T2PIV_ARGUMENT_ERROR;
  uint8_t scp11 = state->scp11_state.security_level;

  if (!ccc) {
    if (PRNG_OK != _t2piv_prng_generate(id, sizeof(id))) {
      return T2PIV_RANDOMNESS_ERROR;
    }
  }
  else {
    memcpy(id, ccc->data, sizeof(id));
  }

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  len = sizeof(CCC_TMPL);
  memcpy(buf, CCC_TMPL, len);
  memcpy(buf + CCC_ID_OFFS, id, T2PIV_CCCID_SIZE);
  res = _t2piv_save_object(state, T2PIV_OBJ_CAPABILITY, buf, len);

Cleanup:
  _t2piv_end_transaction(state);
  return res;
}

t2piv_devmodel t2piv_util_devicemodel(t2piv_state *state) {
  if (!state || !state->context || (state->context == (SCARDCONTEXT)-1)) {
    return DEVTYPE_UNKNOWN;
  }
  return state->model;
}

t2piv_rc t2piv_util_list_keys(t2piv_state *state, uint8_t *key_count, t2piv_key **data, size_t *data_len) {
  t2piv_rc res = T2PIV_OK;
  t2piv_key *pKey = NULL;
  uint8_t *pData = NULL;
  uint8_t *pTemp = NULL;
  size_t cbData = 0;
  size_t offset = 0;
  uint8_t buf[CB_BUF_MAX] = {0};
  size_t cbBuf = 0;
  size_t i = 0;
  size_t cbRealloc = 0;

  const size_t CB_PAGE = 4096;

  const uint8_t SLOTS[] = {
    T2PIV_KEY_AUTHENTICATION,
    T2PIV_KEY_SIGNATURE,
    T2PIV_KEY_KEYMGM,
    T2PIV_KEY_RETIRED1,
    T2PIV_KEY_RETIRED2,
    T2PIV_KEY_RETIRED3,
    T2PIV_KEY_RETIRED4,
    T2PIV_KEY_RETIRED5,
    T2PIV_KEY_RETIRED6,
    T2PIV_KEY_RETIRED7,
    T2PIV_KEY_RETIRED8,
    T2PIV_KEY_RETIRED9,
    T2PIV_KEY_RETIRED10,
    T2PIV_KEY_RETIRED11,
    T2PIV_KEY_RETIRED12,
    T2PIV_KEY_RETIRED13,
    T2PIV_KEY_RETIRED14,
    T2PIV_KEY_RETIRED15,
    T2PIV_KEY_RETIRED16,
    T2PIV_KEY_RETIRED17,
    T2PIV_KEY_RETIRED18,
    T2PIV_KEY_RETIRED19,
    T2PIV_KEY_RETIRED20,
    T2PIV_KEY_CARDAUTH
  };

  if ((NULL == data) || (NULL == data_len) || (NULL == key_count)) { return T2PIV_ARGUMENT_ERROR; }

  uint8_t scp11 = state->scp11_state.security_level;
  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  // init return parameters
  *key_count = 0;
  *data = NULL;
  *data_len = 0;

  // allocate initial page of buffer
  if (NULL == (pData = _t2piv_alloc(state, CB_PAGE))) {
    res = T2PIV_MEMORY_ERROR;
    goto Cleanup;
  }

  cbData = CB_PAGE;

  for (i = 0; i < sizeof(SLOTS); i++) {
    cbBuf = sizeof(buf);
    res = _read_certificate(state, SLOTS[i], buf, &cbBuf);

    if ((res == T2PIV_OK) && (cbBuf > 0)) {
      // add current slot to result, grow result buffer if necessary

      cbRealloc = (sizeof(t2piv_key) + cbBuf - 1) > (cbData - offset) ? MAX((sizeof(t2piv_key) + cbBuf - 1) - (cbData - offset), CB_PAGE) : 0;

      if (0 != cbRealloc) {
        if (!(pTemp = _t2piv_realloc(state, pData, cbData + cbRealloc))) {
          /* realloc failed, pData will be freed in cleanup */
          res = T2PIV_MEMORY_ERROR;
          goto Cleanup;
        }
        yc_memzero(pTemp + cbData, cbRealloc); // clear newly allocated memory
        pData = pTemp;
        pTemp = NULL;
      }

      cbData += cbRealloc;

      // If t2piv_key is misaligned or results in padding, this causes problems
      // in the array we return.  If this becomes a problem, we'll probably want
      // to go with a flat byte array.

      pKey = (t2piv_key*)(pData + offset);

      pKey->slot = SLOTS[i];
      pKey->cert_len = (uint16_t)cbBuf;
      memcpy(pKey->cert, buf, cbBuf);

      offset += sizeof(t2piv_key) + cbBuf - 1;
      (*key_count)++;
    }
  }

  *data = (t2piv_key*)pData;
  pData = NULL;

  if (data_len) {
    *data_len = offset;
  }

  res = T2PIV_OK;

Cleanup:

  if (pData) { _t2piv_free(state, pData); }

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_free(t2piv_state *state, void *data) {
  if (!data) return T2PIV_OK;
  if (!state || (!(state->allocator.pfn_free))) return T2PIV_ARGUMENT_ERROR;

  _t2piv_free(state, data);

  return T2PIV_OK;
}

t2piv_rc t2piv_util_read_cert(t2piv_state *state, uint8_t slot, uint8_t **data, size_t *data_len) {
  t2piv_rc res = T2PIV_OK;
  uint8_t buf[CB_BUF_MAX] = {0};
  size_t cbBuf = sizeof(buf);

  if ((NULL == data )|| (NULL == data_len)) return T2PIV_ARGUMENT_ERROR;

  uint8_t scp11 = state->scp11_state.security_level;
  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  *data = 0;
  *data_len = 0;

  if (T2PIV_OK == (res = _read_certificate(state, slot, buf, &cbBuf))) {

    /* handle those who write empty certificate blobs to PIV objects */
    if (cbBuf == 0) {
      *data = NULL;
      *data_len = 0;
      goto Cleanup;
    }

    if (!(*data = _t2piv_alloc(state, cbBuf))) {
      res = T2PIV_MEMORY_ERROR;
      goto Cleanup;
    }

    memcpy(*data, buf, cbBuf);

    *data_len = cbBuf;
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_write_cert(t2piv_state *state, uint8_t slot, uint8_t *data, size_t data_len, uint8_t certinfo) {
  t2piv_rc res = T2PIV_OK;
  uint8_t scp11 = state->scp11_state.security_level;
  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  res = _write_certificate(state, slot, data, data_len, certinfo);

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_delete_cert(t2piv_state *state, uint8_t slot) {
  return t2piv_util_write_cert(state, slot, NULL, 0, T2PIV_CERTINFO_UNCOMPRESSED);
}

t2piv_rc t2piv_util_block_puk(t2piv_state *state) {
  t2piv_rc res = T2PIV_OK;
  uint8_t puk[] = { 0x30, 0x42, 0x41, 0x44, 0x46, 0x30, 0x30, 0x44 };
  int tries = -1;
  uint8_t data[CB_BUF_MAX] = {0};
  size_t  cb_data = sizeof(data);
  uint8_t *p_item = NULL;
  size_t  cb_item = 0;
  uint8_t flags = 0;

  if (!state) return T2PIV_ARGUMENT_ERROR;
  uint8_t scp11 = state->scp11_state.security_level;

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  while (tries != 0) {
    if (T2PIV_OK == (res = t2piv_change_puk(state, (const char*)puk, sizeof(puk), (const char*)puk, sizeof(puk), &tries))) {
      /* did we accidentally choose the correct PUK?, change our puk and try again */
      puk[0]++;
    }
    else {
      /* depending on the firmware, tries may not be set to zero when the PUK is blocked, */
      /* instead, the return code will be PIN_LOCKED and tries will be unset */
      if (T2PIV_PIN_LOCKED == res) {
        tries = 0;
        res = T2PIV_OK;
      }
    }
  }

  /* attempt to set the puk blocked flag in admin data */

  if (T2PIV_OK == _read_metadata(state, TAG_ADMIN, data, &cb_data)) {
    if (T2PIV_OK == _get_metadata_item(data, cb_data, TAG_ADMIN_FLAGS_1, &p_item, &cb_item)) {
      if (sizeof(flags) == cb_item) {
        memcpy(&flags, p_item, cb_item);
      }
      else {
        DBG("admin flags exist, but are incorrect size = %lu", (unsigned long)cb_item);
      }
    }
  }

  flags |= ADMIN_FLAGS_1_PUK_BLOCKED;

  if (T2PIV_OK != _set_metadata_item(data, &cb_data, CB_OBJ_MAX, TAG_ADMIN_FLAGS_1, (uint8_t*)&flags, sizeof(flags))) {
    DBG("could not set admin flags");
  }
  else {
    if (T2PIV_OK != _write_metadata(state, TAG_ADMIN, data, cb_data)) {
      DBG("could not write admin metadata");
    }
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_read_mscmap(t2piv_state *state, t2piv_container **containers, size_t *n_containers) {
  t2piv_rc res = T2PIV_OK;
  uint8_t buf[CB_BUF_MAX] = {0};
  unsigned long cbBuf = sizeof(buf);
  size_t offs, len = 0;
  uint8_t *ptr = NULL;

  if ((NULL == containers) || (NULL == n_containers)) { res = T2PIV_ARGUMENT_ERROR; goto Cleanup; }

  uint8_t scp11 = state->scp11_state.security_level;
  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  *containers = 0;
  *n_containers = 0;

  if (T2PIV_OK == (res = _t2piv_fetch_object(state, T2PIV_OBJ_MSCMAP, buf, &cbBuf))) {
    ptr = buf;

    /* check that object contents are at least large enough to read the header */
    if (cbBuf < CB_OBJ_TAG_MIN) {
      res = T2PIV_OK;
      goto Cleanup;
    }

    if (*ptr++ == TAG_MSCMAP) {
      offs = _t2piv_get_length(ptr, buf + cbBuf, &len);
      if(!offs) {
        res = T2PIV_OK;
        goto Cleanup;
      }
      ptr += offs;

      if (NULL == (*containers = _t2piv_alloc(state, len))) {
        res = T2PIV_MEMORY_ERROR;
        goto Cleanup;
      }

      /* should check if container map isn't corrupt */

      memcpy(*containers, ptr, len);
      *n_containers = len / sizeof(t2piv_container);
    }
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_write_mscmap(t2piv_state *state, t2piv_container *containers, size_t n_containers) {
  t2piv_rc res = T2PIV_OK;
  uint8_t buf[CB_OBJ_MAX] = {0};
  size_t offset = 0;
  size_t req_len = 0;
  size_t data_len = n_containers * sizeof(t2piv_container);
  uint8_t scp11 = state->scp11_state.security_level;

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  // check if data and data_len are zero, this means that
  // we intend to delete the object
  if ((NULL == containers) || (0 == n_containers)) {

    // if either containers or n_containers are non-zero, return an error,
    // that we only delete strictly when both are set properly
    if ((NULL != containers) || (0 != n_containers)) {
      res = T2PIV_ARGUMENT_ERROR;
    }
    else {
      res = _t2piv_save_object(state, T2PIV_OBJ_MSCMAP, NULL, 0);
    }

    goto Cleanup;
  }

  // encode object data for storage

  // calculate the required length of the encoded object
  req_len = 1 /* data tag */ + (unsigned long)_t2piv_set_length(buf, data_len) + data_len;

  if (req_len > _obj_size_max(state)) {
    res = T2PIV_SIZE_ERROR;
    goto Cleanup;
  }

  buf[offset++] = TAG_MSCMAP;
  offset += _t2piv_set_length(buf + offset, data_len);
  memcpy(buf + offset, (uint8_t*)containers, data_len);
  offset += data_len;

  // write onto device
  res = _t2piv_save_object(state, T2PIV_OBJ_MSCMAP, buf, offset);

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_read_msroots(t2piv_state *state, uint8_t **data, size_t *data_len) {
  t2piv_rc res = T2PIV_OK;
  uint8_t buf[CB_BUF_MAX] = {0};
  unsigned long cbBuf = sizeof(buf);
  size_t offs, len = 0;
  uint8_t *ptr = NULL;
  int object_id = 0;
  uint8_t tag = 0;
  uint8_t *pData = NULL;
  uint8_t *pTemp = NULL;
  size_t cbData = 0;
  size_t cbRealloc = 0;
  size_t offset = 0;

  if (!data || !data_len) return T2PIV_ARGUMENT_ERROR;

  uint8_t scp11 = state->scp11_state.security_level;
  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  *data = 0;
  *data_len = 0;

  // allocate first page
  cbData = _obj_size_max(state);
  if (NULL == (pData = _t2piv_alloc(state, cbData))) { res = T2PIV_MEMORY_ERROR; goto Cleanup; }

  for (object_id = T2PIV_OBJ_MSROOTS1; object_id <= T2PIV_OBJ_MSROOTS5; object_id++) {
    cbBuf = sizeof(buf);

    if (T2PIV_OK != (res = _t2piv_fetch_object(state, object_id, buf, &cbBuf))) {
      goto Cleanup;
    }

    ptr = buf;

    if (cbBuf < CB_OBJ_TAG_MIN) {
      res = T2PIV_OK;
      goto Cleanup;
    }

    tag = *ptr++;

    if (((TAG_MSROOTS_MID != tag) && (TAG_MSROOTS_END != tag)) ||
        ((T2PIV_OBJ_MSROOTS5 == object_id) && (TAG_MSROOTS_END != tag))) {
      // the current object doesn't contain a valid part of a msroots file
      res = T2PIV_OK; // treat condition as object isn't found
      goto Cleanup;
    }

    offs = _t2piv_get_length(ptr, buf + cbBuf, &len);
    if(!offs) {
      res = T2PIV_OK;
      goto Cleanup;
    }
    ptr += offs;

    cbRealloc = len > (cbData - offset) ? len - (cbData - offset) : 0;

    if (0 != cbRealloc) {
      if (!(pTemp = _t2piv_realloc(state, pData, cbData + cbRealloc))) {
        /* realloc failed, pData will be freed in cleanup */
        res = T2PIV_MEMORY_ERROR;
        goto Cleanup;
      }
      pData = pTemp;
      pTemp = NULL;
    }

    cbData += cbRealloc;

    memcpy(pData + offset, ptr, len);
    offset += len;

    if (TAG_MSROOTS_END == tag) {
      break;
    }
  }

  // return data
  *data = pData;
  pData = NULL;
  *data_len = offset;

  res = T2PIV_OK;

Cleanup:

  if (pData) { _t2piv_free(state, pData); }

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_write_msroots(t2piv_state *state, uint8_t *data, size_t data_len) {
  t2piv_rc res = T2PIV_OK;
  uint8_t buf[CB_OBJ_MAX] = {0};
  size_t offset = 0;
  size_t data_offset = 0;
  size_t data_chunk = 0;
  size_t n_objs = 0;
  unsigned int i = 0;
  size_t cb_obj_max = _obj_size_max(state);
  uint8_t scp11 = state->scp11_state.security_level;

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  // check if either data and data_len are zero, this means that
  // we intend to delete the object
  if ((NULL == data) || (0 == data_len)) {

    // if either data or data_len are non-zero, return an error,
    // that we only delete strictly when both are set properly
    if ((NULL != data) || (0 != data_len)) {
      res = T2PIV_ARGUMENT_ERROR;
    }
    else {
      // it should be sufficient to just delete the first object, though
      // to be complete we should erase all of the MSROOTS objects
      res = _t2piv_save_object(state, T2PIV_OBJ_MSROOTS1, NULL, 0);
    }

    goto Cleanup;
  }

  // calculate number of objects required to store blob
  n_objs = (data_len / (cb_obj_max - CB_OBJ_TAG_MAX)) + 1;

  // we're allowing 5 objects to be used to span the msroots file
  if (n_objs > 5) {
    res = T2PIV_SIZE_ERROR;
    goto Cleanup;
  }

  for (i = 0; i < n_objs; i++) {
    offset = 0;
    data_chunk = MIN(cb_obj_max - CB_OBJ_TAG_MAX, data_len - data_offset);

    /* encode object data for storage */
    buf[offset++] = (i == (n_objs - 1)) ? TAG_MSROOTS_END : TAG_MSROOTS_MID;
    offset += _t2piv_set_length(buf + offset, data_chunk);
    memcpy(buf + offset, data + data_offset, data_chunk);
    offset += data_chunk;

    /* write onto device */
    res = _t2piv_save_object(state, (int)(T2PIV_OBJ_MSROOTS1 + i), buf, offset);

    if (T2PIV_OK != res) {
      goto Cleanup;
    }

    data_offset += data_chunk;
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_generate_key(t2piv_state *state, uint8_t slot, uint8_t algorithm, uint8_t pin_policy, uint8_t touch_policy, uint8_t **modulus, size_t *modulus_len, uint8_t **exp, size_t *exp_len, uint8_t **point, size_t *point_len) {
  t2piv_rc res = T2PIV_OK;
  unsigned char in_data[11] = {0};
  unsigned char *in_ptr = in_data;
  unsigned char data[1024] = {0};
  unsigned char templ[] = { 0, T2PIV_INS_GENERATE_ASYMMETRIC, 0, 0 };
  unsigned long recv_len = sizeof(data);
  int sw = 0;
  uint8_t *ptr_modulus = NULL;
  size_t  cb_modulus = 0;
  uint8_t *ptr_exp = NULL;
  size_t  cb_exp = 0;
  uint8_t *ptr_point = NULL;
  size_t  cb_point = 0;
  size_t offs;

  setting_bool_t setting_roca = { 0 };
  const char sz_setting_roca[] = "Enable_Unsafe_Keygen_ROCA";
  const char sz_roca_format[] = "Token2 key serial number %u is affected by vulnerability "
    "CVE-2017-15361 (ROCA) and should be replaced. On-chip key generation %s  "
    "See the ROCA advisory (CVE-2017-15361) "
    "for additional information on device replacement and mitigation assistance.\n";
  const char sz_roca_allow_user[] = "was permitted by an end-user configuration setting, but is not recommended.";
  const char sz_roca_allow_admin[] = "was permitted by an administrator configuration setting, but is not recommended.";
  const char sz_roca_block_user[] = "was blocked due to an end-user configuration setting.";
  const char sz_roca_block_admin[] = "was blocked due to an administrator configuration setting.";
  const char sz_roca_default[] = "was permitted by default, but is not recommended.  "
    "The default behavior may change in a future release.";

  if (!state) return T2PIV_ARGUMENT_ERROR;
  uint8_t scp11 = state->scp11_state.security_level;

  if ((algorithm == T2PIV_ALGO_RSA3072 || algorithm == T2PIV_ALGO_RSA4096 || T2PIV_IS_25519(algorithm))
       && !is_version_compatible(state, 5, 7, 0)) {
    DBG("RSA3072, RSA4096, ED25519 and X25519 keys are only supported by applet version 5.7.0 and newer");
    return T2PIV_NOT_SUPPORTED;
  }
  if ((algorithm == T2PIV_ALGO_RSA1024 || algorithm == T2PIV_ALGO_RSA2048) && !is_version_compatible(state, 4, 3, 5)) {
    const char *psz_msg = NULL;
    setting_roca = setting_get_bool(sz_setting_roca, true);

    switch (setting_roca.source) {
      case SETTING_SOURCE_ADMIN:
        psz_msg = setting_roca.value ? sz_roca_allow_admin : sz_roca_block_admin;
        break;

      case SETTING_SOURCE_USER:
        psz_msg = setting_roca.value ? sz_roca_allow_user : sz_roca_block_user;
        break;

      default:
      case SETTING_SOURCE_DEFAULT:
        psz_msg = sz_roca_default;
        break;
    }

    DBG(sz_roca_format, state->serial, psz_msg);
    yc_log_event("Token2 PIV Library", 1, setting_roca.value ? YC_LOG_LEVEL_WARN : YC_LOG_LEVEL_ERROR, sz_roca_format,
                 state->serial, psz_msg);

    if (!setting_roca.value) {
      return T2PIV_NOT_SUPPORTED;
    }
  }

  switch (algorithm) {
  case T2PIV_ALGO_RSA1024:
  case T2PIV_ALGO_RSA2048:
  case T2PIV_ALGO_RSA3072:
  case T2PIV_ALGO_RSA4096:
    if (!modulus || !modulus_len || !exp || !exp_len) {
      DBG("Invalid output parameter for RSA algorithm");
      return T2PIV_ARGUMENT_ERROR;
    }
    *modulus = NULL;
    *modulus_len = 0;
    *exp = NULL;
    *exp_len = 0;
    break;

  case T2PIV_ALGO_ECCP256:
  case T2PIV_ALGO_ECCP384:
  case T2PIV_ALGO_ED25519:
  case T2PIV_ALGO_X25519:
    if (!point || !point_len) {
      DBG("Invalid output parameter for ECC algorithm");
      return T2PIV_ARGUMENT_ERROR;
    }
    *point = NULL;
    *point_len = 0;
    break;

  default:
    DBG("Invalid algorithm specified");
    return T2PIV_GENERIC_ERROR;
  }

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  templ[3] = slot;

  *in_ptr++ = 0xac;
  *in_ptr++ = 3;
  *in_ptr++ = T2PIV_ALGO_TAG;
  *in_ptr++ = 1;
  *in_ptr++ = algorithm;

  if (in_data[4] == 0) {
    res = T2PIV_ALGORITHM_ERROR;
    DBG("Unexpected algorithm");
    goto Cleanup;
  }

  if (pin_policy != T2PIV_PINPOLICY_DEFAULT) {
    in_data[1] += 3;
    *in_ptr++ = T2PIV_PINPOLICY_TAG;
    *in_ptr++ = 1;
    *in_ptr++ = pin_policy;
  }

  if (touch_policy != T2PIV_TOUCHPOLICY_DEFAULT) {
    in_data[1] += 3;
    *in_ptr++ = T2PIV_TOUCHPOLICY_TAG;
    *in_ptr++ = 1;
    *in_ptr++ = touch_policy;
  }

  if (T2PIV_OK != (res = _t2piv_transfer_data(state, templ, in_data, (unsigned long)(in_ptr - in_data), data, &recv_len, &sw))) {
    goto Cleanup;
  }
  res = t2piv_translate_sw_ex(__FUNCTION__, sw);
  if (res != T2PIV_OK) {
    DBG("Failed to generate new key");
    goto Cleanup;
  }

  if (T2PIV_IS_RSA(algorithm)) {
    size_t len;
    unsigned char *data_ptr = data + 2 + _t2piv_get_length(data + 2, data + recv_len, &len);

    if (*data_ptr != TAG_RSA_MODULUS) {
      DBG("Failed to parse public key structure (modulus).");
      res = T2PIV_PARSE_ERROR;
      goto Cleanup;
    }

    data_ptr++;
    offs = _t2piv_get_length(data_ptr, data + recv_len, &len);
    if(!offs) {
      DBG("Failed to parse public key structure (modulus length).");
      res = T2PIV_PARSE_ERROR;
      goto Cleanup;
    }
    data_ptr += offs;

    cb_modulus = len;
    if (NULL == (ptr_modulus = _t2piv_alloc(state, cb_modulus))) {
      DBG("Failed to allocate memory for modulus.");
      res = T2PIV_MEMORY_ERROR;
      goto Cleanup;
    }

    memcpy(ptr_modulus, data_ptr, cb_modulus);

    data_ptr += len;

    if (*data_ptr != TAG_RSA_EXP) {
      DBG("Failed to parse public key structure (public exponent).");
      res = T2PIV_PARSE_ERROR;
      goto Cleanup;
    }

    data_ptr++;
    offs = _t2piv_get_length(data_ptr, data + recv_len, &len);
    if(!offs) {
      DBG("Failed to parse public key structure (public exponent length).");
      res = T2PIV_PARSE_ERROR;
      goto Cleanup;
    }
    data_ptr += offs;

    cb_exp = len;
    if (NULL == (ptr_exp = _t2piv_alloc(state, cb_exp))) {
      DBG("Failed to allocate memory for public exponent.");
      res = T2PIV_MEMORY_ERROR;
      goto Cleanup;
    }

    memcpy(ptr_exp, data_ptr, cb_exp);

    // set output parameters

    *modulus = ptr_modulus;
    ptr_modulus = NULL;
    *modulus_len = cb_modulus;
    *exp = ptr_exp;
    ptr_exp = NULL;
    *exp_len = cb_exp;
  }
  else if (T2PIV_IS_EC(algorithm) || T2PIV_IS_25519(algorithm)) {
    unsigned char *data_ptr = data + 3;
    size_t len = 0;

    if (T2PIV_ALGO_ECCP256 == algorithm) {
      len = CB_ECC_POINTP256;
    } else if (T2PIV_ALGO_ECCP384 == algorithm) {
      len = CB_ECC_POINTP384;
    } else if (T2PIV_IS_25519(algorithm)) {
      len = CB_ECC_POINT25519;
    }

    if (*data_ptr++ != TAG_ECC_POINT) {
      DBG("Failed to parse public key structure.");
      res = T2PIV_PARSE_ERROR;
      goto Cleanup;
    }

    if (*data_ptr++ != len) { /* the curve point should always be determined by the curve */
      DBG("Unexpected length.");
      res = T2PIV_ALGORITHM_ERROR;
      goto Cleanup;
    }

    cb_point = len;
    if (NULL == (ptr_point = _t2piv_alloc(state, cb_point))) {
      DBG("Failed to allocate memory for public point.");
      res = T2PIV_MEMORY_ERROR;
      goto Cleanup;
    }

    memcpy(ptr_point, data_ptr, cb_point);

    // set output parameters

    *point = ptr_point;
    ptr_point = NULL;
    *point_len = cb_point;
  }
  else {
    DBG("Wrong algorithm.");
    res = T2PIV_ALGORITHM_ERROR;
    goto Cleanup;
  }

Cleanup:

  if (ptr_modulus) { _t2piv_free(state, ptr_modulus); }
  if (ptr_exp) { _t2piv_free(state, ptr_exp); }
  if (ptr_point) { _t2piv_free(state, ptr_point); }

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_get_config(t2piv_state *state, t2piv_config *config) {
  t2piv_rc res = T2PIV_OK;
  uint8_t data[CB_BUF_MAX] = { 0 };
  size_t cb_data = sizeof(data);
  uint8_t *p_item = NULL;
  size_t cb_item = 0;

  if (NULL == state) return T2PIV_ARGUMENT_ERROR;
  if (NULL == config) return T2PIV_ARGUMENT_ERROR;

  uint8_t scp11 = state->scp11_state.security_level;

  // initialize default values

  config->puk_blocked = false;
  config->puk_noblock_on_upgrade = false;
  config->pin_last_changed = 0;
  config->mgm_type = T2PIV_CONFIG_MGM_MANUAL;

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  /* recover admin data */
  if (T2PIV_OK == _read_metadata(state, TAG_ADMIN, data, &cb_data)) {
    if (T2PIV_OK == _get_metadata_item(data, cb_data, TAG_ADMIN_FLAGS_1, &p_item, &cb_item)) {
      if (*p_item & ADMIN_FLAGS_1_PUK_BLOCKED) config->puk_blocked = true;
      if (*p_item & ADMIN_FLAGS_1_PROTECTED_MGM) config->mgm_type = T2PIV_CONFIG_MGM_PROTECTED;
    }

    if (T2PIV_OK == _get_metadata_item(data, cb_data, TAG_ADMIN_SALT, &p_item, &cb_item)) {
      if (config->mgm_type != T2PIV_CONFIG_MGM_MANUAL) {
        DBG("conflicting types of mgm key administration configured");
        config->mgm_type = T2PIV_CONFIG_MGM_INVALID;
      }
      else {
        config->mgm_type = T2PIV_CONFIG_MGM_DERIVED;
      }
    }

    if (T2PIV_OK == _get_metadata_item(data, cb_data, TAG_ADMIN_TIMESTAMP, &p_item, &cb_item)) {
      if (CB_ADMIN_TIMESTAMP != cb_item)  {
        DBG("pin timestamp in admin metadata is an invalid size");
      }
      else {
        memcpy(&(config->pin_last_changed), p_item, cb_item);
      }
    }
  }

  /* recover protected data */
  cb_data = sizeof(data);

  if (T2PIV_OK == _read_metadata(state, TAG_PROTECTED, data, &cb_data)) {

    if (T2PIV_OK == _get_metadata_item(data, cb_data, TAG_PROTECTED_FLAGS_1, &p_item, &cb_item)) {
      if (*p_item & PROTECTED_FLAGS_1_PUK_NOBLOCK) config->puk_noblock_on_upgrade = true;
    }

    if (T2PIV_OK == _get_metadata_item(data, cb_data, TAG_PROTECTED_MGM, &p_item, &cb_item)) {
      if(sizeof(config->mgm_key) >= cb_item) {
        memcpy(config->mgm_key, p_item, cb_item);
        config->mgm_len = cb_item;
        if (config->mgm_type != T2PIV_CONFIG_MGM_PROTECTED) {
          DBG("conflicting types of mgm key administration configured - protected mgm exists");
          config->mgm_type = T2PIV_CONFIG_MGM_INVALID;
        }
      } else {
        DBG("protected data contains mgm, but is the wrong size = %lu", (unsigned long)cb_item);
        config->mgm_type = T2PIV_CONFIG_MGM_INVALID;
      }
    }
  }
  else {
    if (config->mgm_type == T2PIV_CONFIG_MGM_PROTECTED) {
      DBG("admin data indicates protected mgm present, but the object cannot be read");
      config->mgm_type = T2PIV_CONFIG_MGM_INVALID;
    }
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_set_pin_last_changed(t2piv_state *state) {
  t2piv_rc res = T2PIV_OK;
  t2piv_rc t2rc = T2PIV_OK;
  uint8_t  data[CB_BUF_MAX] = { 0 };
  size_t   cb_data = sizeof(data);
  time_t   tnow = 0;

  if (NULL == state) return T2PIV_ARGUMENT_ERROR;
  uint8_t scp11 = state->scp11_state.security_level;

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  /* recover admin data */
  if (T2PIV_OK != (t2rc = _read_metadata(state, TAG_ADMIN, data, &cb_data))) {
    cb_data = 0; /* set current metadata blob size to zero, we'll add the timestamp to the blank blob */
  }

  tnow = time(NULL);

  if (T2PIV_OK != (res = _set_metadata_item(data, &cb_data, CB_OBJ_MAX, TAG_ADMIN_TIMESTAMP, (uint8_t*)&tnow, CB_ADMIN_TIMESTAMP))) {
    DBG("could not set pin timestamp, err = %d", res);
  }
  else {
    if (T2PIV_OK != (res = _write_metadata(state, TAG_ADMIN, data, cb_data))) {
      /* Note: this can fail if authenticate() wasn't called previously - expected behavior */
      DBG("could not write admin data, err = %d", res);
    }
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_get_derived_mgm(t2piv_state *state, const uint8_t *pin, const size_t pin_len, t2piv_mgm *mgm) {
  t2piv_rc res = T2PIV_OK;
  pkcs5_rc p5rc = PKCS5_OK;
  uint8_t  data[CB_BUF_MAX] = { 0 };
  size_t   cb_data = sizeof(data);
  uint8_t  *p_item = NULL;
  size_t   cb_item = 0;

  if (NULL == state) return T2PIV_ARGUMENT_ERROR;
  if ((NULL == pin) || (0 == pin_len) || (NULL == mgm)) return T2PIV_ARGUMENT_ERROR;

  uint8_t scp11 = state->scp11_state.security_level;
  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  /* recover management key */
  if (T2PIV_OK == (res = _read_metadata(state, TAG_ADMIN, data, &cb_data))) {
    if (T2PIV_OK == (res = _get_metadata_item(data, cb_data, TAG_ADMIN_SALT, &p_item, &cb_item))) {
      if (cb_item != CB_ADMIN_SALT) {
        DBG("derived mgm salt exists, but is incorrect size = %lu", (unsigned long)cb_item);
        res = T2PIV_GENERIC_ERROR;
        goto Cleanup;
      }
      mgm->len = DES_LEN_3DES;
      if (PKCS5_OK != (p5rc = pkcs5_pbkdf2_sha1(pin, pin_len, p_item, cb_item, ITER_MGM_PBKDF2, mgm->data, mgm->len))) {
        DBG("pbkdf2 failure, err = %d", p5rc);
        res = T2PIV_GENERIC_ERROR;
        goto Cleanup;
      }
    }
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_get_protected_mgm(t2piv_state *state, t2piv_mgm *mgm) {
  t2piv_rc res = T2PIV_OK;
  uint8_t  data[CB_BUF_MAX] = { 0 };
  size_t   cb_data = sizeof(data);
  uint8_t  *p_item = NULL;
  size_t   cb_item = 0;

  if (NULL == state) return T2PIV_ARGUMENT_ERROR;
  if (NULL == mgm) return T2PIV_ARGUMENT_ERROR;
  uint8_t scp11 = state->scp11_state.security_level;

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) return res;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  if (T2PIV_OK != (res = _read_metadata(state, TAG_PROTECTED, data, &cb_data))) {
    DBG("could not read protected data, err = %d", res);
    goto Cleanup;
  }

  if (T2PIV_OK != (res = _get_metadata_item(data, cb_data, TAG_PROTECTED_MGM, &p_item, &cb_item))) {
    DBG("could not read protected mgm from metadata, err = %d", res);
    goto Cleanup;
  }

  if (cb_item > sizeof(mgm->data)) {
    DBG("protected data contains mgm, but is the wrong size = %lu", (unsigned long)cb_item);
    res = T2PIV_AUTHENTICATION_ERROR;
    goto Cleanup;
  }

  mgm->len = cb_item;
  memcpy(mgm->data, p_item, cb_item);

Cleanup:

  yc_memzero(data, sizeof(data));

  _t2piv_end_transaction(state);
  return res;

}

t2piv_rc t2piv_util_update_protected_mgm(t2piv_state *state, t2piv_mgm *mgm) {
  t2piv_rc res = T2PIV_OK;
  uint8_t data[CB_BUF_MAX] = {0};
  size_t cb_data = sizeof(data);
  uint8_t scp11 = state->scp11_state.security_level;

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) goto Cleanup;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  if (T2PIV_OK != (res = _read_metadata(state, TAG_PROTECTED, data, &cb_data))) {
    cb_data = 0; /* set current metadata blob size to zero, we'll add to the blank blob */
  }

  if (T2PIV_OK != (res = _set_metadata_item(data, &cb_data, CB_OBJ_MAX, TAG_PROTECTED_MGM, mgm->data, mgm->len))) {
    DBG("could not set protected mgm item, err = %d", res);
  }
  else {
    if (T2PIV_OK != (res = _write_metadata(state, TAG_PROTECTED, data, cb_data))) {
      DBG("could not write protected data, err = %d", res);
      goto Cleanup;
    }
  }

Cleanup:

  _t2piv_end_transaction(state);
  return res;
}

/* to set a generated mgm, pass NULL for mgm, or set mgm.data to all zeroes */
t2piv_rc t2piv_util_set_protected_mgm(t2piv_state *state, t2piv_mgm *mgm) {
  t2piv_rc res = T2PIV_OK;
  t2piv_rc t2rc = T2PIV_OK;
  prng_rc  prngrc = PRNG_OK;
  bool     fGenerate = false;
  size_t   mgm_len = DES_LEN_3DES;
  uint8_t  mgm_key[sizeof(mgm->data)] = { 0 };
  size_t   i = 0;
  uint8_t  data[CB_BUF_MAX] = { 0 };
  size_t   cb_data = sizeof(data);
  uint8_t  *p_item = NULL;
  size_t   cb_item = 0;
  uint8_t  flags_1 = 0;

  if (NULL == state) return T2PIV_ARGUMENT_ERROR;
   uint8_t scp11 = state->scp11_state.security_level;

  fGenerate = true;
  if (mgm) {
    mgm_len = mgm->len;
    memcpy(mgm_key, mgm->data, mgm->len);

    for (i = 0; i < mgm_len; i++) {
      if (mgm_key[i] != 0) {
        fGenerate = false;
        break;
      }
    }
  }

  if (T2PIV_OK != (res = _t2piv_begin_transaction(state))) goto Cleanup;
  if (T2PIV_OK != (res = _t2piv_ensure_application_selected(state, scp11))) goto Cleanup;

  /* try to set the mgm key as long as we don't encounter a fatal error */
  do {
    if (fGenerate) {
      /* generate a new mgm key */
      if (PRNG_OK != (prngrc = _t2piv_prng_generate(mgm_key, mgm_len))) {
        DBG("could not generate new mgm, err = %d", prngrc);
        res = T2PIV_RANDOMNESS_ERROR;
        goto Cleanup;
      }
    }

    if (T2PIV_OK != (t2rc = t2piv_set_mgmkey3(state, mgm_key, mgm_len, T2PIV_ALGO_AUTO, T2PIV_TOUCHPOLICY_AUTO))) {
      /*
      ** if _set_mgmkey fails with T2PIV_KEY_ERROR, it means the generated key is weak
      ** otherwise, log a warning, since the device mgm key is corrupt or we're in
      ** a state where we can't set the mgm key
      */
      if (T2PIV_KEY_ERROR != t2rc) {
        DBG("could not set new derived mgm key, err = %d", t2rc);
        res = t2rc;
        goto Cleanup;
      }
    }
    else {
      /* _set_mgmkey succeeded, stop generating */
      fGenerate = false;
    }
  } while (fGenerate);

  /* set output mgm */
  if (mgm) {
    memcpy(mgm->data, mgm_key, mgm_len);
  }

  /* after this point, we've set the mgm key, so the function should succeed, regardless of being able to set the metadata */

  /* set the new mgm key in protected data */
  if (T2PIV_OK != (t2rc = _read_metadata(state, TAG_PROTECTED, data, &cb_data))) {
    cb_data = 0; /* set current metadata blob size to zero, we'll add to the blank blob */
  }

  if (T2PIV_OK != (t2rc = _set_metadata_item(data, &cb_data, CB_OBJ_MAX, TAG_PROTECTED_MGM, mgm_key, mgm_len))) {
    DBG("could not set protected mgm item, err = %d", t2rc);
  }
  else {
    if (T2PIV_OK != (t2rc = _write_metadata(state, TAG_PROTECTED, data, cb_data))) {
      DBG("could not write protected data, err = %d", t2rc);
      goto Cleanup;
    }
  }

  /* set the protected mgm flag in admin data */
  cb_data = sizeof(data);

  if (T2PIV_OK != (t2rc = _read_metadata(state, TAG_ADMIN, data, &cb_data))) {
    cb_data = 0;
  }
  else {

    if (T2PIV_OK != (t2rc = _get_metadata_item(data, cb_data, TAG_ADMIN_FLAGS_1, &p_item, &cb_item))) {
      /* flags are not set */
      DBG("admin data exists, but flags are not present");
    }

    if (cb_item == sizeof(flags_1)) {
      memcpy(&flags_1, p_item, cb_item);
    }
    else {
      DBG("admin data flags are an incorrect size = %lu", (unsigned long)cb_item);
    }

    /* remove any existing salt */
    if (T2PIV_OK != (t2rc = _set_metadata_item(data, &cb_data, CB_OBJ_MAX, TAG_ADMIN_SALT, NULL, 0))) {
      DBG("could not unset derived mgm salt, err = %d", t2rc);
    }
  }

  flags_1 |= ADMIN_FLAGS_1_PROTECTED_MGM;

  if (T2PIV_OK != (t2rc = _set_metadata_item(data, &cb_data, CB_OBJ_MAX, TAG_ADMIN_FLAGS_1, &flags_1, sizeof(flags_1)))) {
    DBG("could not set admin flags item, err = %d", t2rc);
  }
  else {
    if (T2PIV_OK != (t2rc = _write_metadata(state, TAG_ADMIN, data, cb_data))) {
      DBG("could not write admin data, err = %d", t2rc);
      goto Cleanup;
    }
  }


Cleanup:

  yc_memzero(data, sizeof(data));
  yc_memzero(mgm_key, sizeof(mgm_key));

  _t2piv_end_transaction(state);
  return res;
}

t2piv_rc t2piv_util_reset(t2piv_state *state) {
  unsigned char templ[] = {0, T2PIV_INS_RESET, 0, 0};
  unsigned char data[256] = {0};
  unsigned long recv_len = sizeof(data);
  t2piv_rc res;
  int sw;

  /* note: the reset function is only available when both pins are blocked. */
  res = t2piv_transfer_data(state, templ, NULL, 0, data, &recv_len, &sw);
  if(res != T2PIV_OK) {
    return res;
  }
  return t2piv_translate_sw_ex(__FUNCTION__, sw);
}

uint32_t t2piv_util_slot_object(uint8_t slot) {
  int object_id = -1;

  switch (slot) {
  case T2PIV_KEY_AUTHENTICATION:
    object_id = T2PIV_OBJ_AUTHENTICATION;
    break;

  case T2PIV_KEY_SIGNATURE:
    object_id = T2PIV_OBJ_SIGNATURE;
    break;

  case T2PIV_KEY_KEYMGM:
    object_id = T2PIV_OBJ_KEY_MANAGEMENT;
    break;

  case T2PIV_KEY_CARDAUTH:
    object_id = T2PIV_OBJ_CARD_AUTH;
    break;

  case T2PIV_KEY_ATTESTATION:
    object_id = T2PIV_OBJ_ATTESTATION;
    break;

  default:
    if ((slot >= T2PIV_KEY_RETIRED1) && (slot <= T2PIV_KEY_RETIRED20)) {
      object_id = T2PIV_OBJ_RETIRED1 + (slot - T2PIV_KEY_RETIRED1);
    }
    break;
  }

  return (uint32_t) object_id;
}

static t2piv_rc
decompress_data(const uint8_t *compressed_data, size_t compressed_len, uint8_t *output_data, size_t *output_len) {
#ifdef USE_CERT_COMPRESS
  size_t expected_len = SIZE_MAX; // Safe since it is out of range of a 16 bit unsigned integer

  if (compressed_len >= 4 && compressed_data[0] == 0x01 && compressed_data[1] == 0x00) { // NETiD zlib compression
    // Compression format: 0x01 0x00 + 2-byte little-endian length + zlib compressed data
    expected_len = compressed_data[2] | (compressed_data[3] << 8);
    compressed_data += 4; // Skip the 4-byte header
    compressed_len -= 4;
  }

  z_stream zs;
  zs.zalloc = Z_NULL;
  zs.zfree = Z_NULL;
  zs.opaque = Z_NULL;
  zs.avail_in = (uInt) compressed_len;
  zs.next_in = (Bytef *) compressed_data;
  zs.avail_out = (uInt) * output_len;
  zs.next_out = (Bytef *) output_data;

  // 'MAX_WBITS' is the window bits. '0x20' tells zlib to use gzip or zlib format for decompression
  if (inflateInit2(&zs, (MAX_WBITS | 0x20)) != Z_OK) {
    DBG("Failed to initialize decompression");
    return T2PIV_GENERIC_ERROR;
  }

  int res = inflate(&zs, Z_FINISH);
  if (res != Z_STREAM_END) {
   inflateEnd(&zs);
    if (res == Z_BUF_ERROR) {
      DBG("Decompression failed: Allocated output buffer too small");
      return T2PIV_SIZE_ERROR;
    }
    DBG("Decompression failed with error code: %d", res);
    return T2PIV_INVALID_OBJECT;
  }

  if (inflateEnd(&zs) != Z_OK) {
    DBG("Failed to finalize decompression");
    return T2PIV_INVALID_OBJECT;
  }

  if (expected_len != SIZE_MAX && zs.total_out != expected_len) {
    DBG("Decompressed data length mismatch. Expected %zu, got %lu", expected_len, zs.total_out);
    return T2PIV_INVALID_OBJECT;
  }

  *output_len = zs.total_out;
  return T2PIV_OK;
#else
  DBG("Decompressing certificate not supported");
  return T2PIV_NOT_SUPPORTED;
#endif
}

t2piv_rc t2piv_util_get_certdata(uint8_t *buf, size_t buf_len, uint8_t* certdata, size_t *certdata_len) {
   uint8_t compress_info = T2PIV_CERTINFO_UNCOMPRESSED;
   uint8_t *certptr = 0;
   size_t cert_len = 0;
   uint8_t *ptr = buf;

   while (ptr < buf + buf_len) {
     uint8_t tag = *ptr++;
     size_t len = 0;
     size_t offs = _t2piv_get_length(ptr, buf + buf_len, &len);
     if(!offs) {
       DBG("Found invalid length for tag 0x%02x.", tag);
       goto invalid_tlv;
     }
     ptr += offs; // move to after length bytes

     switch (tag) {
       case TAG_CERT:
         certptr = ptr;
         cert_len = len;
         DBG("Found TAG_CERT with length %zu", cert_len);
         break;
       case TAG_CERT_COMPRESS:
         if(len != 1) {
           DBG("Found TAG_CERT_COMPRESS with invalid length %zu", len);
           goto invalid_tlv;
         }
         compress_info = *ptr;
         DBG("Found TAG_CERT_COMPRESS with length %zu value 0x%02x", len, compress_info);
         break;
       case TAG_CERT_LRC: {
         // basically ignore it
         DBG("Found TAG_CERT_LRC with length %zu", len);
         break;
       }
       default:
         DBG("Unknown cert tag 0x%02x", tag);
         goto invalid_tlv;
         break;
     }
     ptr += len; // move to after value bytes
   }

invalid_tlv:
   if(certptr == 0 || cert_len == 0 || ptr != buf + buf_len || compress_info > T2PIV_CERTINFO_GZIP) {
     DBG("Invalid TLV encoding, treating as a raw certificate");
     certptr = buf;
     cert_len = buf_len;
   }

   if (compress_info == T2PIV_CERTINFO_GZIP) {
     DBG("Found compressed certificate");
     t2piv_rc res = decompress_data(certptr, cert_len, certdata, certdata_len);
     if (res != T2PIV_OK) {
       DBG("Failed to decompress certificate data");
       *certdata_len = 0;
       return res;
     }
   } else {
     if (*certdata_len < cert_len) {
       DBG("Buffer too small");
       *certdata_len = 0;
       return T2PIV_SIZE_ERROR;
     }
     memmove(certdata, certptr, cert_len);
     *certdata_len = cert_len;
   }
   return T2PIV_OK;
}

t2piv_rc t2piv_util_write_certdata(uint8_t *rawdata, size_t rawdata_len, uint8_t compress_info, uint8_t* certdata, size_t *certdata_len) {
  size_t offset = 0;
  size_t buf_len = 0;

  unsigned long len_bytes = get_length_size((unsigned long)rawdata_len);

   // calculate the required length of the encoded object
   buf_len = 1 /* cert tag */ + 3 /* compression tag + data*/ + 2 /* lrc */;
   buf_len += len_bytes + rawdata_len;

   if (buf_len > *certdata_len) {
     DBG("Buffer too small");
     *certdata_len = 0;
     return T2PIV_SIZE_ERROR;
   }

  memmove(certdata + len_bytes + 1, rawdata, rawdata_len);

  certdata[offset++] = TAG_CERT;
  offset += _t2piv_set_length(certdata+offset, rawdata_len);
  offset += rawdata_len;
  certdata[offset++] = TAG_CERT_COMPRESS;
  certdata[offset++] = 1;
  certdata[offset++] = compress_info;
  certdata[offset++] = TAG_CERT_LRC;
  certdata[offset++] = 0;
  *certdata_len = offset;
  return T2PIV_OK;
}

static t2piv_rc _read_certificate(t2piv_state *state, uint8_t slot, uint8_t *buf, size_t *buf_len) {
  t2piv_rc res = T2PIV_OK;
  int object_id = (int)t2piv_util_slot_object(slot);

  if (-1 == object_id) return T2PIV_INVALID_OBJECT;

   unsigned char data[T2PIV_OBJ_MAX_SIZE] = {0};
   unsigned long data_len = sizeof (data);

  if (T2PIV_OK == (res = _t2piv_fetch_object(state, object_id, data, &data_len))) {
    if ((res = t2piv_util_get_certdata(data, data_len, buf, buf_len)) != T2PIV_OK) {
      DBG("Failed to get certificate data");
      return res;
    }
  } else {
    *buf_len = 0;
  }

  return res;
}

static t2piv_rc _write_certificate(t2piv_state *state, uint8_t slot, uint8_t *data, size_t data_len, uint8_t certinfo) {
  uint8_t buf[CB_OBJ_MAX] = {0};
  size_t buf_len = sizeof(buf);
  int object_id = (int)t2piv_util_slot_object(slot);


  if (-1 == object_id) return T2PIV_INVALID_OBJECT;

  // check if data or data_len are zero, this means that we intend to delete the object
  if ((NULL == data) || (0 == data_len)) {

    // if either data or data_len are non-zero, return an error,
    // that we only delete strictly when both are set properly
    if ((NULL != data) || (0 != data_len)) {
      return T2PIV_ARGUMENT_ERROR;
    }

    return _t2piv_save_object(state, object_id, NULL, 0);
  }

  // encode certificate data for storage
  t2piv_rc res = T2PIV_OK;
  if ( (res=t2piv_util_write_certdata(data, data_len, certinfo, buf, &buf_len)) != T2PIV_OK) {
    return res;
  }

  // write onto device
  return _t2piv_save_object(state, object_id, buf, buf_len);
}

/*
** PIV Manager data helper functions
**
** These functions allow the PIV Manager to extend the T2PIV_OBJ_ADMIN_DATA object without having to change
** this implementation.  New items may be added without modifying these functions.  Data items are picked
** from the pivman_data buffer by tag, and replaced either in place if length allows or the data object is
** expanded to fit a new/updated data item.
*/

/*
** _get_metadata_item
**
** Parses the metadata blob, specified by data, looking for the specified tag.  If found, the item is
** returned in pp_item and its size in pcb_item.
**
** If the item is not found, this function returns T2PIV_GENERIC_ERROR.
*/
static t2piv_rc _get_metadata_item(uint8_t *data, size_t cb_data, uint8_t tag, uint8_t **pp_item, size_t *pcb_item) {
  uint8_t *p_temp = data;
  size_t  offs, cb_temp = 0;
  uint8_t tag_temp = 0;
  bool found = false;

  if (!data || !pp_item || !pcb_item) return T2PIV_ARGUMENT_ERROR;

  *pp_item = NULL;
  *pcb_item = 0;

  while (p_temp < (data + cb_data)) {
    tag_temp = *p_temp++;

    offs = _t2piv_get_length(p_temp, data + cb_data, &cb_temp);
    if (!offs) {
      return T2PIV_PARSE_ERROR;
    }

    p_temp += offs;

    if (tag_temp == tag) {
      // found tag
      found = true;
      break;
    }

    p_temp += cb_temp;
  }

  if (found) {
    *pp_item = p_temp;
    *pcb_item = cb_temp;
  }

  return found ? T2PIV_OK : T2PIV_GENERIC_ERROR;
}

t2piv_rc t2piv_util_parse_metadata(uint8_t *data, size_t data_len, t2piv_metadata *metadata) {
  uint8_t *p = 0;
  size_t cb = 0;
  uint32_t cnt = 0;

  t2piv_rc rc = _get_metadata_item(data, data_len, T2PIV_METADATA_ALGORITHM_TAG, &p, &cb);
  if(rc == T2PIV_OK && cb == 1) {
    metadata->algorithm = p[0];
    cnt++;
  }

  rc = _get_metadata_item(data, data_len, T2PIV_METADATA_POLICY_TAG, &p, &cb);
  if(rc == T2PIV_OK && cb == 2) {
    metadata->pin_policy = p[0];
    metadata->touch_policy = p[1];
    cnt++;
  }

  rc = _get_metadata_item(data, data_len, T2PIV_METADATA_ORIGIN_TAG, &p, &cb);
  if(rc == T2PIV_OK && cb == 1) {
    metadata->origin = p[0];
    cnt++;
  }

  rc = _get_metadata_item(data, data_len, T2PIV_METADATA_PUBKEY_TAG, &p, &cb);
  if(rc == T2PIV_OK && cb > 0 && cb <= sizeof(metadata->pubkey)) {
    metadata->pubkey_len = cb;
    memcpy(metadata->pubkey, p, cb);
    cnt++;
  }

  return cnt ? T2PIV_OK : T2PIV_PARSE_ERROR;
}

/*
** _set_metadata_item
**
** Adds or replaces a data item encoded in a metadata blob, specified by tag to the existing
** metadata blob (data) until it reaches the a maximum buffer size (cb_data_max).
**
** If adding/replacing the item would exceed cb_data_max, this function returns T2PIV_GENERIC_ERROR.
**
** The new size of the blob is returned in pcb_data.
*/
static t2piv_rc _set_metadata_item(uint8_t *data, size_t *pcb_data, size_t cb_data_max, uint8_t tag, uint8_t *p_item, size_t cb_item) {
  uint8_t *p_temp = data;
  size_t  cb_temp = 0;
  uint8_t tag_temp = 0;
  size_t  cb_len = 0;
  uint8_t *p_next = NULL;
  long    cb_moved = 0; /* must be signed to have negative offsets */

  if (!data || !pcb_data) return T2PIV_ARGUMENT_ERROR;

  while (p_temp < (data + *pcb_data)) {
    tag_temp = *p_temp++;
    cb_len = _t2piv_get_length(p_temp, data + *pcb_data, &cb_temp);
    if(!cb_len) {
        return T2PIV_PARSE_ERROR;
    }
    p_temp += cb_len;

    if (tag_temp == tag) {
      /* found tag */

      /* check length, if it matches, overwrite */
      if (cb_temp == cb_item) {
        memcpy(p_temp, p_item, cb_item);
        return T2PIV_OK;
      }

      /* length doesn't match, expand/shrink to fit */
      p_next = p_temp + cb_temp;
      cb_moved = (long)cb_item - (long)cb_temp +
        ((long)(cb_item != 0 ? (long)_t2piv_get_length_size(cb_item) : -1l /* for tag, if deleting */) -
        (long)cb_len); /* accounts for different length encoding */

      /* length would cause buffer overflow, return error */
      if ((size_t)(*pcb_data + cb_moved) > cb_data_max) {
        return T2PIV_GENERIC_ERROR;
      }

      /* move remaining data */
      memmove(p_next + cb_moved, p_next, *pcb_data - (size_t)(p_next - data));
      *pcb_data += cb_moved;

      /* re-encode item and insert */
      if (cb_item != 0) {
        p_temp -= cb_len;
        p_temp += _t2piv_set_length(p_temp, cb_item);
        memcpy(p_temp, p_item, cb_item);
      }

      return T2PIV_OK;
    } /* if tag found */

    p_temp += cb_temp;
  }

  if (cb_item == 0) {
    /* we've been asked to delete an existing item that isn't in the blob */
    return T2PIV_OK;
  }

  // we did not find an existing tag, append
  p_temp = data + *pcb_data;
  cb_len = _t2piv_get_length_size(cb_item);

  // length would cause buffer overflow, return error
  if (*pcb_data + 1 + cb_len + cb_item > cb_data_max) {
    return T2PIV_GENERIC_ERROR;
  }

  *p_temp++ = tag;
  p_temp += _t2piv_set_length(p_temp, cb_item);
  memcpy(p_temp, p_item, cb_item);
  *pcb_data += 1 + cb_len + cb_item;

  return T2PIV_OK;
}

/*
** _read_metadata
**
** Reads admin or protected data (specified by tag) from its associated object.
**
** The data stored in the object is parsed to ensure it has the correct tag and valid length.
**
** data must point to a buffer of at least CB_BUF_MAX bytes, and pcb_data should point to
** the size of data.
**
** To read from protected data, the pin must be verified prior to calling this function.
*/
static t2piv_rc _read_metadata(t2piv_state *state, uint8_t tag, uint8_t* data, size_t* pcb_data) {
  t2piv_rc res = T2PIV_OK;
  uint8_t *p_temp = NULL;
  unsigned long cb_temp = 0;
  size_t offs;
  int obj_id = 0;

  if (!data || !pcb_data || (CB_BUF_MAX > *pcb_data)) return T2PIV_ARGUMENT_ERROR;

  switch (tag) {
  case TAG_ADMIN: obj_id = T2PIV_OBJ_ADMIN_DATA; break;
  case TAG_PROTECTED: obj_id = T2PIV_OBJ_PRINTED; break;
  default: return T2PIV_INVALID_OBJECT;
  }

  cb_temp = (unsigned long)*pcb_data;
  *pcb_data = 0;

  if (T2PIV_OK != (res = _t2piv_fetch_object(state, obj_id, data, &cb_temp))) {
    return res;
  }

  if (cb_temp < CB_OBJ_TAG_MIN) return T2PIV_PARSE_ERROR;

  p_temp = data;

  if (tag != *p_temp++) return T2PIV_PARSE_ERROR;

  offs = _t2piv_get_length(p_temp, data + cb_temp, pcb_data);
  if (!offs) {
    *pcb_data = 0;
    return T2PIV_PARSE_ERROR;
  }

  p_temp += offs;

  memmove(data, p_temp, *pcb_data);

  return T2PIV_OK;
}

/*
** _write_metadata
**
** Writes admin/protected data, specified by tag to its associated object.
**
** To delete the metadata, set data to NULL and cb_data to 0.
**
** To write protected data, the pin must be verified prior to calling this function.
*/
static t2piv_rc _write_metadata(t2piv_state *state, uint8_t tag, uint8_t *data, size_t cb_data) {
  t2piv_rc res = T2PIV_OK;
  uint8_t buf[CB_OBJ_MAX] = { 0 };
  uint8_t *pTemp = buf;
  int obj_id = 0;

  if (cb_data > (_obj_size_max(state) - CB_OBJ_TAG_MAX)) {
    return T2PIV_GENERIC_ERROR;
  }

  switch (tag) {
  case TAG_ADMIN: obj_id = T2PIV_OBJ_ADMIN_DATA; break;
  case TAG_PROTECTED: obj_id = T2PIV_OBJ_PRINTED; break;
  default: return T2PIV_INVALID_OBJECT;
  }

  if (!data || (0 == cb_data)) {
    // deleting metadata
    res = _t2piv_save_object(state, obj_id, NULL, 0);
  }
  else {
    *pTemp++ = tag;
    pTemp += _t2piv_set_length(pTemp, cb_data);

    memcpy(pTemp, data, cb_data);
    pTemp += cb_data;

    res = _t2piv_save_object(state, obj_id, buf, (size_t)(pTemp - buf));
  }

  return res;
}