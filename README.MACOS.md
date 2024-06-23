# FIDO2-Manage Script for macOS

## Overview
Welcome to the early-stage version of the FIDO2-Manage Script adaptation for macOS! This tool is designed to manage FIDO2 security keys. Please note that the script is still under active development, and currently, **only the command-line interface (CLI) is available**. A graphical user interface (GUI) is planned for future releases.

## Features
- Manage your FIDO2 security keys via the command line.

## Installation

1. **Install xcode-select**
   Open terminal and run

    `xcode-select --install`

2. **Install other prerequisites using brew**

    `brew install zlibbrew`

    `brew install cmake`

    `brew install libcbor`

    `brew install libcblite`

    `brew install libsolv`

    `brew install libuv`

    `brew install pkg-config`

    `brew install grep`


3. **Clone and compile**

    `git clone https://github.com/Token2/fido2-manage.git`

    `cd fido2-manage`

    `rm -rf build && mkdir build && cd build && cmake -USE_PCSC=ON ..`

    `cd ..`

    `make -C build`

    `sudo make -C build install`

    `chmod 755 fido2-manage-mac.sh`

4. **Test the script**  

Plug in your FIDO2 key and run the command below:

    /fido2-manage-mac.sh -list

The output should be similar to below:
```console
    MacBook-Air fido2-manage % ./fido2-manage-mac.sh -list
    Device [1] : TOKEN2 FIDO2 Security Key(0026
    Device [2] : TOKEN2 FIDO2 Security Key
```

### Usage ###
    The syntax and command line parameters are similar to our  [fido2-manage.exe tool for Windows](https://www.token2.com/site/page/fido2-token-management-tool-fido2-manage-exe).

Make sure you replace `fido2-manage.exe` with `fido2-manage-mac.sh` when issuing the commands.
