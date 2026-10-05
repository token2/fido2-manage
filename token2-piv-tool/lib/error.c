/*
 * Copyright (c) 2014-2016,2019-2020 Yubico AB
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

#include "t2piv.h"

#include <stddef.h>

#define ERR(name, desc) { name, #name, desc }

typedef struct
{
  t2piv_rc rc;
  const char *name;
  const char *description;
} err_t;

static const err_t errors[] = {
  ERR (T2PIV_OK, "Successful return"),
  ERR (T2PIV_MEMORY_ERROR, "Error allocating memory"),
  ERR (T2PIV_PCSC_ERROR, "Error in PCSC call"),
  ERR (T2PIV_SIZE_ERROR, "Wrong buffer size"),
  ERR (T2PIV_APPLET_ERROR, "No PIV application found"),
  ERR (T2PIV_AUTHENTICATION_ERROR, "Authentication error"),
  ERR (T2PIV_RANDOMNESS_ERROR, "Error getting randomness"),
  ERR (T2PIV_GENERIC_ERROR, "Something went wrong."),
  ERR (T2PIV_KEY_ERROR, "Key error"),
  ERR (T2PIV_PARSE_ERROR, "Parse error"),
  ERR (T2PIV_WRONG_PIN, "Wrong PIN code"),
  ERR (T2PIV_INVALID_OBJECT, "Invalid object"),
  ERR (T2PIV_ALGORITHM_ERROR, "Algorithm error"),
  ERR (T2PIV_PIN_LOCKED, "PIN locked"),
  ERR (T2PIV_ARGUMENT_ERROR, "Argument error"),
  ERR (T2PIV_RANGE_ERROR, "Range error"),
  ERR (T2PIV_NOT_SUPPORTED, "Not supported"),
  ERR (T2PIV_PCSC_SERVICE_ERROR, "PCSC service not available"),
  ERR (T2PIV_CONDITION_ERROR, "Conditions not met to use command"),
};

/**
 * t2piv_strerror:
 * @err: error code
 *
 * Convert return code to human readable string explanation of the
 * reason for the particular error code.
 *
 * This string can be used to output a diagnostic message to the user.
 *
 * Return value: Returns a pointer to a statically allocated string
 *   containing an explanation of the error code @err.
 **/
const char *t2piv_strerror(t2piv_rc err) {
  static const char *unknown = "Unknown t2piv error";
  const char *p;

  if (-err < 0 || -err >= (int) (sizeof (errors) / sizeof (errors[0])))
    return unknown;

  p = errors[-err].description;
  if (!p)
    p = unknown;

  return p;
}


/**
 * t2piv_strerror_name:
 * @err: error code
 *
 * Convert return code to human readable string representing the error
 * code symbol itself.  For example, t2piv_strerror_name(%T2PIV_OK)
 * returns the string "T2PIV_OK".
 *
 * This string can be used to output a diagnostic message to the user.
 *
 * Return value: Returns a pointer to a statically allocated string
 *   containing a string version of the error code @err, or NULL if
 *   the error code is not known.
 **/
const char *t2piv_strerror_name(t2piv_rc err) {
  if (-err < 0 || -err >= (int) (sizeof (errors) / sizeof (errors[0])))
    return NULL;

  return errors[-err].name;
}
