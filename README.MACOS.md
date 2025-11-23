# FIDO2-Manage Script for macOS

## Overview
Welcome to the early-stage version of the FIDO2-Manage Script adaptation for macOS! This tool is designed to manage FIDO2 security keys.  
## Features
- Manage your FIDO2 security keys via the command line. Refer to build_dmg.md for information about building the full application including GUI

## Installation

1. **Install xcode-select**

    Open terminal and run

    `xcode-select --install`

3. **Install other prerequisites using Homebrew**

    `brew install zlib`

    `brew install cmake`

    `brew install libcbor`

    `brew install libcblite`

    `brew install libsolv`

    `brew install libuv`

    `brew install pkg-config`
   
    `brew install openssl`

    `brew install grep`

    `brew install tcl-tk`

    `brew install python-tk`


5. **Clone the source code from GitHub and compile**

    `git clone https://github.com/Token2/fido2-manage.git`

    `cd fido2-manage`

    `rm -rf build && mkdir build && cd build && cmake -USE_PCSC=ON ../`

    `cd ..`

    `make -C build`

    `sudo make -C build install`

    `chmod +x fido2-manage-mac.sh`

6. **Test the script**  

Plug in your FIDO2 key(s) and run the command below:

    ./fido2-manage-mac.sh -list

The output should be similar to below:
```console
    MacBook-Air fido2-manage % ./fido2-manage-mac.sh -list
    Device [1] : TOKEN2 FIDO2 Security Key(0026
    Device [2] : TOKEN2 FIDO2 Security Key
```

### Usage ###
The syntax and command line parameters are similar to our  [fido2-manage.exe tool for Windows](https://www.token2.com/site/page/fido2-token-management-tool-fido2-manage-exe).
Make sure you replace `fido2-manage.exe` with `fido2-manage-mac.sh` or `fido2-manage` (where available) when issuing the commands.
