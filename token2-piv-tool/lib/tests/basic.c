/*
 * Copyright (c) 2014-2016 Yubico AB
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
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <check.h>

START_TEST(test_version_string) {
  if (strcmp(T2PIV_VERSION_STRING, t2piv_check_version(NULL)) != 0) {
    ck_abort_msg("version mismatch %s != %s\n", T2PIV_VERSION_STRING,
                 t2piv_check_version(NULL));
  }

  if (t2piv_check_version(T2PIV_VERSION_STRING) == NULL) {
    ck_abort_msg("version NULL?\n");
  }

  if (t2piv_check_version("99.99.99") != NULL) {
    ck_abort_msg("version not NULL?\n");
  }

  fprintf(stderr, "t2piv version: header %s library %s\n",
          T2PIV_VERSION_STRING, t2piv_check_version (NULL));
}
END_TEST

START_TEST(test_strerror) {
  const char *s;

  if (t2piv_strerror(T2PIV_OK) == NULL) {
    ck_abort_msg("t2piv_strerror NULL\n");
  }

  s = t2piv_strerror_name(T2PIV_OK);
  if (s == NULL || strcmp(s, "T2PIV_OK") != 0) {
    ck_abort_msg("t2piv_strerror_name %s\n", s);
  }
}
END_TEST

static Suite *basic_suite(void) {
  Suite *s;
  TCase *tc;

  s = suite_create("libt2piv basic");
  tc = tcase_create("basic");
  tcase_add_test(tc, test_version_string);
  tcase_add_test(tc, test_strerror);
  suite_add_tcase(s, tc);

  return s;
}

int main(void)
{
  int number_failed;
  Suite *s;
  SRunner *sr;

  s = basic_suite();
  sr = srunner_create(s);
  srunner_run_all(sr, CK_NORMAL);
  number_failed = srunner_ntests_failed(sr);
  srunner_free(sr);
  return (number_failed == 0) ? EXIT_SUCCESS : EXIT_FAILURE;
}
