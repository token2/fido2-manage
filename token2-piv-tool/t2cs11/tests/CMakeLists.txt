# Copyright (c) 2020 Yubico AB
# All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions are
# met:
#
#     * Redistributions of source code must retain the above copyright
#       notice, this list of conditions and the following disclaimer.
#
#     * Redistributions in binary form must reproduce the above
#       copyright notice, this list of conditions and the following
#       disclaimer in the documentation and/or other materials provided
#       with the distribution.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
# "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
# LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
# A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
# OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
# SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
# LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
# DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
# THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
# (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

message("t2cs11/tests/CMakeList.txt")

include(check)
find_check()

if(NOT DEFINED SKIP_TESTS)
    if(ENABLE_HARDWARE_TESTS)
        if(NOT WIN32)
            add_test(
                NAME reset_tests
                COMMAND ${BASH_PROGRAM} ${CMAKE_CURRENT_SOURCE_DIR}/reset.sh
            )
            set_property(TEST reset_tests APPEND PROPERTY ENVIRONMENT "EXEEXT=${CMAKE_EXECUTABLE_SUFFIX}")
            set_property(TEST reset_tests APPEND PROPERTY ENVIRONMENT "T2PIV_ENV_HWTESTS_CONFIRMED=${HW_TESTS}")
        else(NOT WIN32)
            message("Running on Windows. Skipping basic tests...")
        endif(NOT WIN32)
    endif(ENABLE_HARDWARE_TESTS)

    set(SOURCE_T2CS11_TESTS
            t2cs11_tests.c
            t2cs11_tests_util.c
    )
    add_executable(test_t2cs11 ${SOURCE_T2CS11_TESTS})
    target_link_libraries(test_t2cs11 t2cs11_shared ${LIBCRYPTO_LDFLAGS})
    add_test(
        NAME test_t2cs11
        COMMAND test_t2cs11
        WORKING_DIRECTORY ${CMAKE_BINARY_DIR}/t2cs11/tests/
    )
    set_property(TEST test_t2cs11 APPEND PROPERTY ENVIRONMENT "T2PIV_ENV_HWTESTS_CONFIRMED=${HW_TESTS}")

    set(SOURCE_T2CS11_INTERFACES_TESTS t2cs11_interfaces_tests.c)
    add_executable(test_t2cs11_interfaces ${SOURCE_T2CS11_INTERFACES_TESTS})
    target_link_libraries(test_t2cs11_interfaces t2cs11_shared ${LIBCRYPTO_LDFLAGS})
    add_test(
        NAME test_t2cs11_interfaces
        COMMAND test_t2cs11_interfaces
        WORKING_DIRECTORY ${CMAKE_BINARY_DIR}/t2cs11/tests/
    )
    set_property(TEST test_t2cs11_interfaces APPEND PROPERTY ENVIRONMENT "T2PIV_ENV_HWTESTS_CONFIRMED=${HW_TESTS}")

    set(SOURCE_T2CS11_EDX_TESTS
            t2cs11_edx_test.c
            t2cs11_tests_util.c
            )
    add_executable(test_t2cs11_edx ${SOURCE_T2CS11_EDX_TESTS})
    target_link_libraries(test_t2cs11_edx t2cs11_shared ${LIBCRYPTO_LDFLAGS})
    add_test(
            NAME test_t2cs11_edx
            COMMAND test_t2cs11_edx
            WORKING_DIRECTORY ${CMAKE_BINARY_DIR}/t2cs11/tests/
    )
    set_property(TEST test_t2cs11_edx APPEND PROPERTY ENVIRONMENT "T2PIV_ENV_HWTESTS_CONFIRMED=${HW_TESTS}")

endif(NOT DEFINED SKIP_TESTS)