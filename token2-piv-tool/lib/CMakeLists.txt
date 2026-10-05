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

message("lib/CMakeList.txt")

include(${CMAKE_SOURCE_DIR}/cmake/pcscd.cmake)
find_pcscd()

set(T2PIV_VERSION_STRING "${token2_piv_tool_VERSION_MAJOR}.${token2_piv_tool_VERSION_MINOR}.${token2_piv_tool_VERSION_PATCH}")

set(CMAKE_WINDOWS_EXPORT_ALL_SYMBOLS TRUE)

set(SOURCE
        t2piv.c
        util.c
        t2import.c
        version.c
        error.c
        internal.c
        ecdh.c
        scp11_util.c
        ../aes_cmac/aes.c
        ../aes_cmac/aes_cmac.c
        ../common/openssl-compat.c
        ../common/util.c)

include_directories(
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/common
)

if (ENABLE_CERT_COMPRESS)
    add_definitions(-DUSE_CERT_COMPRESS="1")

    find_library(ZLIB zlib PATHS ${ZLIB_LIB_DIR})
    include_directories(${ZLIB_INCL_DIR})

    find_package(ZLIB REQUIRED)

    set(ZLIB_LIBS "ZLIB::ZLIB")
endif()

if(WIN32)
    set(ADDITIONAL_LIBRARY ws2_32)
endif ()

# static library
if(BUILD_STATIC_LIB)
    add_library(t2piv STATIC ${SOURCE})
    target_link_libraries(t2piv ${LIBCRYPTO_LIBRARIES} ${PCSC_LIBRARIES} ${ZLIB_LIBS} ${ADDITIONAL_LIBRARY})
    set_target_properties (t2piv PROPERTIES COMPILE_FLAGS "-DSTATIC ")
    if(WIN32)
        set_target_properties(t2piv PROPERTIES OUTPUT_NAME t2piv_static)
    endif()
    add_coverage(t2piv)
endif(BUILD_STATIC_LIB)

# dynamic library
add_library(t2piv_shared SHARED ${SOURCE})
target_link_libraries(t2piv_shared ${LIBCRYPTO_LIBRARIES} ${PCSC_LIBRARIES} ${ZLIB_LIBS} ${ADDITIONAL_LIBRARY})
set_target_properties(t2piv_shared PROPERTIES SOVERSION ${SO_VERSION} VERSION ${VERSION})
if (${CMAKE_SYSTEM_NAME} MATCHES "Darwin")
    set_target_properties(t2piv_shared PROPERTIES INSTALL_RPATH "${T2PIV_INSTALL_LIB_DIR}")
endif()
if(WIN32)
    set_target_properties(t2piv_shared PROPERTIES OUTPUT_NAME libt2piv)
else(WIN32)
    set_target_properties(t2piv_shared PROPERTIES OUTPUT_NAME t2piv)
endif(WIN32)
add_coverage(t2piv_shared)

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/t2piv.pc.in ${CMAKE_CURRENT_SOURCE_DIR}/t2piv.pc @ONLY)
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/t2piv-config.h.in ${CMAKE_CURRENT_SOURCE_DIR}/t2piv-config.h @ONLY)

install(
        TARGETS t2piv_shared
        ARCHIVE DESTINATION ${T2PIV_INSTALL_LIB_DIR}
        LIBRARY DESTINATION ${T2PIV_INSTALL_LIB_DIR}
        RUNTIME DESTINATION ${T2PIV_INSTALL_BIN_DIR})
if(BUILD_STATIC_LIB)
    install(
        TARGETS t2piv
        ARCHIVE DESTINATION ${T2PIV_INSTALL_LIB_DIR}
        LIBRARY DESTINATION ${T2PIV_INSTALL_LIB_DIR}
        RUNTIME DESTINATION ${T2PIV_INSTALL_BIN_DIR})
endif(BUILD_STATIC_LIB)

install(FILES t2piv.h DESTINATION ${T2PIV_INSTALL_INC_DIR}/t2piv)
install(FILES t2piv-config.h DESTINATION ${T2PIV_INSTALL_INC_DIR}/t2piv)

install(FILES t2piv.pc DESTINATION ${T2PIV_INSTALL_PKGCONFIG_DIR})

add_subdirectory(tests)
