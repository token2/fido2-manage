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

message("t2cs11/CMakeList.txt")

link_directories(${PCSC_LIBRARY_DIRS})
add_definitions(-DCRYPTOKI_EXPORTS)
if(NOT WIN32)
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -fvisibility=hidden")
endif(NOT WIN32)

set(
        SOURCE
        t2cs11.c
        token.c
        mechanisms.c
        utils.c
        openssl_utils.c
        objects.c
        ../common/openssl-compat.c
        ../common/util.c
)

include_directories(
        ${CMAKE_SOURCE_DIR}/lib
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${LIBCRYPTO_INCLUDE_DIRS}
)

set(T2CS11_VERSION_MAJOR ${token2_piv_tool_VERSION_MAJOR})
set(T2CS11_VERSION_MINOR  ${token2_piv_tool_VERSION_MINOR})
set(T2CS11_VERSION_PATCH ${token2_piv_tool_VERSION_PATCH})
set(T2CS11_VERSION_STRING "${T2CS11_VERSION_MAJOR}.${T2CS11_VERSION_MINOR}.${T2CS11_VERSION_PATCH}")
set(T2CS11_VERSION_NUMBER ${T2CS11_VERSION_MAJOR}.${T2CS11_VERSION_MINOR}.${T2CS11_VERSION_PATCH})
if(${ENABLE_HARDWARE_TESTS})
    set(HW_TESTS 1)
endif(${ENABLE_HARDWARE_TESTS})

# static library
if(BUILD_STATIC_LIB)
    add_library(t2cs11 STATIC ${SOURCE})
    target_link_libraries(t2cs11 ${LIBCRYPTO_LDFLAGS} t2piv_static)
    set_target_properties (t2cs11 PROPERTIES COMPILE_FLAGS "-DSTATIC ")
    if(WIN32)
        set_target_properties(t2cs11 PROPERTIES OUTPUT_NAME t2cs11_static)
    endif()
    add_coverage(t2cs11)
endif(BUILD_STATIC_LIB)

# dynamic library
add_library(t2cs11_shared SHARED ${SOURCE})
target_link_libraries(t2cs11_shared ${LIBCRYPTO_LDFLAGS} t2piv_shared)
set_target_properties(t2cs11_shared PROPERTIES SOVERSION ${SO_VERSION} VERSION ${VERSION})
if (${CMAKE_SYSTEM_NAME} MATCHES "Darwin")
    set_target_properties(t2cs11_shared PROPERTIES INSTALL_RPATH "${T2PIV_INSTALL_LIB_DIR}")
endif()
if(WIN32)
    set_target_properties(t2cs11_shared PROPERTIES OUTPUT_NAME libt2cs11)
else(WIN32)
    set_target_properties(t2cs11_shared PROPERTIES OUTPUT_NAME t2cs11)
endif(WIN32)
add_coverage(t2cs11_shared)

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/t2cs11.pc.in ${CMAKE_CURRENT_SOURCE_DIR}/t2cs11.pc @ONLY)
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/t2cs11-config.h.in ${CMAKE_CURRENT_SOURCE_DIR}/t2cs11-config.h @ONLY)

install(
        TARGETS t2cs11_shared
        ARCHIVE DESTINATION "${T2PIV_INSTALL_LIB_DIR}"
        LIBRARY DESTINATION "${T2PIV_INSTALL_LIB_DIR}"
        RUNTIME DESTINATION "${T2PIV_INSTALL_BIN_DIR}")

if(BUILD_STATIC_LIB)
    install(
        TARGETS t2cs11
        ARCHIVE DESTINATION "${T2PIV_INSTALL_LIB_DIR}"
        LIBRARY DESTINATION "${T2PIV_INSTALL_LIB_DIR}"
        RUNTIME DESTINATION "${T2PIV_INSTALL_BIN_DIR}")
endif(BUILD_STATIC_LIB)

install(FILES t2cs11.pc DESTINATION ${T2PIV_INSTALL_PKGCONFIG_DIR})

add_subdirectory(tests)
