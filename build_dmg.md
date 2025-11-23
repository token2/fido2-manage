# FIDO2 Manager macOS Deployment Scripts

This folder contains the deployment scripts for building, packaging, and preparing the **FIDO2 Manager** application on macOS. These scripts automate the entire process from compiling the C++ CLI binary to creating a distributable DMG with the GUI.

> **Note:** These scripts are intended to be run on macOS (Apple Silicon preferred).

 

## ⚡ Prerequisites

Make sure you have the following installed:

* macOS (13.0 or later recommended)
* [Homebrew](https://brew.sh/)
* Xcode command line tools
* Python 3
* CMake

The deployment script will check for and install necessary Homebrew dependencies:

* pkg-config
* openssl@3
* libcbor
* zlib
* python-tk

---

## 🏗 Deployment Steps

1. **These script should be in  the project root**  
2. **Make the main deployment script executable**:

```bash
chmod +x deploy_macos.sh
```

3. **Run the deployment script**:

```bash
./deploy_macos.sh
```

This will:

* Set up a Python virtual environment
* Build the C++ CLI (`fido2-token2`)
* Bundle required libraries
* Build the macOS GUI app with PyInstaller
* Fix library linking for macOS
* Optionally code-sign the app if `sign_macos_app.sh` is present
* Create a DMG for distribution

---

## 🧪 Verification

After running the script:

* The final `.app` bundle will be in:

```
dist/fido2-manage.app
```

* The distributable DMG will be in:

```
dist/fido2-manage.dmg
```

* The script performs a self-contained check to ensure all required libraries are bundled and CLI commands work.

---

## ⚙ Customization

* **Icon:** Place your `icon.icns` in the project root to replace the placeholder icon.
* **Code signing:** If you have an Apple Developer ID, ensure `sign_macos_app.sh` exists and is executable. The deployment script will prompt to sign the app.

---

## 📝 Notes

* Avoid spaces in your project directory path. The build process handles them but may fail in some cases.
* The script assumes ARM64 architecture. Modify `CMAKE_OSX_ARCHITECTURES` in the script if you need x86_64 support.
* If you encounter missing libraries, check the `staging` folder and ensure `bundle_libs.sh` and `fix_macos_linking.sh` exist.

---

## ⚡ Quick Tips

* To rebuild from scratch, you can safely delete `build/`, `dist/`, and `.venv/` before running the script.
* Use the final DMG to distribute the app; it contains everything needed to run on other macOS machines.


 
