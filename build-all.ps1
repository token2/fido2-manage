<#
  build-all.ps1 - builds token2-piv-tool (CLI + libt2piv + libt2cs11), libfido2 (upstream, PC/SC backend) and
  Token2 Key Manager (Tauri GUI) on Windows x64, in one go.

  Usage (PowerShell, from this folder):
      Set-ExecutionPolicy -Scope Process Bypass
      .\build-all.ps1                 # build everything, release
      .\build-all.ps1 -Dev            # build tool, then run the GUI in dev mode
      .\build-all.ps1 -ToolOnly       # only token2-piv-tool
      .\build-all.ps1 -HardwareTests  # also build the C test suites (needs 'check')

  Prerequisites (installed once):
    * Visual Studio 2022 (Community or Build Tools) with "Desktop development with C++"
    * CMake >= 3.16 (ships with VS; otherwise cmake.org)
    * Rust via rustup (MSVC toolchain)  https://rustup.rs
    * Git
  vcpkg and the Tauri CLI are fetched automatically if missing.
#>
[CmdletBinding()]
param(
    [switch]$FastDev,   # quick incremental debug build of the GUI exe only (no bundler)
    [string]$VcpkgPath = "C:\vcpkg",
    [switch]$Dev,
    [switch]$ToolOnly,
    [switch]$HardwareTests,
    [string]$LibFido2Tag = "1.15.0",
    [string]$DistPath = ""   # where to place build outputs; default: <repo>\dist (set to a path OUTSIDE the repo in CI so it is never committed)
)
$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
$Tool = Join-Path $Root "token2-piv-tool"
$Gui  = Join-Path $Root "token2-piv-gui\src-tauri"
if ($DistPath) { $Dist = $DistPath } else { $Dist = Join-Path $Root "dist" }
New-Item -ItemType Directory -Force $Dist | Out-Null

function Need($cmd, $hint) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) { throw "'$cmd' not found. $hint" }
}
function Step($m) { Write-Host "`n==> $m" -ForegroundColor Cyan }

Step "Checking prerequisites"
Need cmake "Install Visual Studio 2022 with C++ workload, or CMake from cmake.org."
Need git   "Install Git for Windows."
if (-not $ToolOnly) { Need cargo "Install Rust from https://rustup.rs (MSVC toolchain)." }
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere) -or -not (& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)) {
    throw "Visual Studio 2022 C++ build tools not found. Install the 'Desktop development with C++' workload."
}

Step "vcpkg dependencies (openssl, getopt, zlib)"
if (-not (Test-Path "$VcpkgPath\vcpkg.exe")) {
    if (-not (Test-Path $VcpkgPath)) { git clone https://github.com/microsoft/vcpkg $VcpkgPath }
    & "$VcpkgPath\bootstrap-vcpkg.bat" -disableMetrics
}
$pkgs = @("openssl:x64-windows", "getopt:x64-windows", "zlib:x64-windows", "libcbor:x64-windows")
if ($HardwareTests) { $pkgs += "check:x64-windows" }
& "$VcpkgPath\vcpkg.exe" install @pkgs
if ($LASTEXITCODE) { throw "vcpkg install failed" }
# vcpkg puts everything under installed\x64-windows (packages\ is per-port and not always complete)
$I = "$VcpkgPath\installed\x64-windows"
$P = "$VcpkgPath\packages"
$env:OPENSSL_ROOT_DIR = $I
function FindLib($names) {
    foreach ($n in $names) {
        foreach ($dir in @("$I\lib", "$P\zlib_x64-windows\lib", "$P\getopt-win32_x64-windows\lib")) {
            $f = Join-Path $dir $n
            if (Test-Path $f) { return $f }
        }
    }
    throw "Could not find any of [$($names -join ', ')] under $I\lib - check 'dir $I\lib'"
}
$ZlibLib   = FindLib @("zlib.lib", "z.lib", "zlibstatic.lib")
$GetoptLib = FindLib @("getopt.lib")
Write-Host "zlib:   $ZlibLib"
Write-Host "getopt: $GetoptLib"

Step "Building token2-piv-tool"
$Build = Join-Path $Tool "build"
New-Item -ItemType Directory -Force $Build | Out-Null
# single portable folder holding the GUI, all CLI tools and every DLL together
$Portable = Join-Path $Dist "token2-key-manager"   # final flat portable folder
$ToolDist = Join-Path $Dist "_toolstage"           # CMake install prefix (staging)
New-Item -ItemType Directory -Force $Portable | Out-Null
$cmakeArgs = @(
    "-S", $Tool, "-B", $Build, "-A", "x64",
    "-DCMAKE_INSTALL_PREFIX=$ToolDist",
    "-DGETOPT=$GetoptLib",
    "-DGETOPT_LIB_DIR=$I\lib",
    "-DGETOPT_INCLUDE_DIR=$I\include",
    "-DZLIB=$ZlibLib",
    "-DZLIB_LIB_DIR=$I\lib",
    "-DZLIB_INCL_DIR=$I\include",
    "-DZLIB_ROOT=$I",
    "-DZLIB_LIBRARY=$ZlibLib",
    "-DZLIB_INCLUDE_DIR=$I\include"
)
if ($HardwareTests) { $cmakeArgs += "-DENABLE_HARDWARE_TESTS=ON" } else { $cmakeArgs += "-DSKIP_TESTS=1" }
cmake @cmakeArgs
if ($LASTEXITCODE) { throw "cmake configure failed" }
cmake --build $Build --config Release --target install
if ($LASTEXITCODE) { throw "token2-piv-tool build failed" }

function RuntimeDlls {
    $found = @()
    foreach ($pat in @("libcrypto*.dll", "z*.dll", "getopt*.dll")) {
        $m = Get-ChildItem "$I\bin\$pat" -ErrorAction SilentlyContinue | Where-Object Name -match '^(z|zlib)[0-9]*\.dll$|^libcrypto|^getopt'
        if (-not $m) {
            if ($pat -like "z*") { Write-Host "note: no zlib DLL (static zlib), nothing to copy"; continue }
            throw "No $pat in $I\bin - check 'dir $I\bin'"
        }
        $found += $m
    }
    return $found
}
$dlls = RuntimeDlls
foreach ($d in $dlls) { Copy-Item $d.FullName "$Portable\" -Force }
# OpenSSL legacy provider (RC2/DES PKCS#12 from Windows exports)
$legacy = Get-ChildItem "$I\bin\legacy.dll", "$I\lib\ossl-modules\legacy.dll", "$I\bin\ossl-modules\legacy.dll" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($legacy) { Copy-Item $legacy.FullName "$Portable\" -Force; Write-Host "openssl legacy provider: $($legacy.FullName)" } else { Write-Host "note: legacy.dll not found in vcpkg openssl; RC2-protected PFX files will not import" }
Write-Host ("runtime DLLs: " + (($dlls | ForEach-Object Name) -join ", "))
$LibDir = Join-Path $Build "lib\Release"
if (-not (Test-Path "$LibDir\libt2piv.lib")) { throw "libt2piv.lib not produced in $LibDir" }
Get-ChildItem "$ToolDist\bin\*.exe","$ToolDist\bin\*.dll" -ErrorAction SilentlyContinue | ForEach-Object { Copy-Item $_.FullName "$Portable\" -Force }
Copy-Item "$LibDir\libt2piv.dll" "$Portable\" -Force -ErrorAction SilentlyContinue
Write-Host "CLI tools ready: $Portable" -ForegroundColor Green

Step "Building libfido2 $LibFido2Tag (upstream, PC/SC backend on: USB CCID + NFC)"
$Fido = Join-Path $Root "deps\libfido2"
if (-not (Test-Path "$Fido\CMakeLists.txt")) {
    New-Item -ItemType Directory -Force (Join-Path $Root "deps") | Out-Null
    git clone --depth 1 --branch $LibFido2Tag https://github.com/Yubico/libfido2 $Fido
    if ($LASTEXITCODE) { throw "git clone of libfido2 failed" }
}
$FBuild = Join-Path $Fido "build"
New-Item -ItemType Directory -Force $FBuild | Out-Null
# Token2 CTAP-over-CCID tunnel + T=0 GET RESPONSE (FIDO management over the OTP
# applet, no admin needed). Always reset the clone to pristine upstream first,
# then apply the patch, so a partially-patched tree from a previous run can never
# make `git apply` conflict. The marker only records that this build patched it.
$patchMarker = Join-Path $Fido ".token2-tunnel-applied"
Push-Location $Fido
# revert any prior local changes (a stale/partial patch), keep the .git so we can reset
git checkout -- . 2>$null
git clean -fd 2>$null
Remove-Item -Force $patchMarker -ErrorAction SilentlyContinue
git apply --ignore-whitespace (Join-Path $Root "libfido2-token2.patch")
if ($LASTEXITCODE) {
    Pop-Location
    throw "applying libfido2-token2.patch failed (is deps\libfido2 a clean clone at tag $LibFido2Tag?)"
}
Pop-Location
New-Item -ItemType File -Force $patchMarker | Out-Null
# ship the library under the Token2 name (API unchanged)
$srcCmake = Join-Path $Fido "src\CMakeLists.txt"
(Get-Content $srcCmake -Raw) -replace 'OUTPUT_NAME fido2(\s)', 'OUTPUT_NAME t2fido2$1' | Set-Content $srcCmake -NoNewline
$null = FindLib @("cbor.lib")
$ZlibName = [System.IO.Path]::GetFileNameWithoutExtension($ZlibLib)
$fidoArgs = @(
    "-S", $Fido, "-B", $FBuild, "-A", "x64",
    "-DBUILD_TESTS=OFF", "-DBUILD_EXAMPLES=OFF", "-DBUILD_MANPAGES=OFF", "-DBUILD_TOOLS=ON", "-DBUILD_STATIC_LIBS=OFF",
    "-DUSE_PCSC=ON", "-DUSE_WINHELLO=OFF",
    "-DCBOR_INCLUDE_DIRS=$I\include", "-DCBOR_LIBRARY_DIRS=$I\lib", "-DCBOR_BIN_DIRS=$I\bin", "-DCBOR_LIBRARIES=cbor",
    "-DCRYPTO_INCLUDE_DIRS=$I\include", "-DCRYPTO_LIBRARY_DIRS=$I\lib", "-DCRYPTO_BIN_DIRS=$I\bin", "-DCRYPTO_LIBRARIES=libcrypto",
    # find_package(ZLIB) honors these (ZLIB_ROOT + singular INCLUDE_DIR/LIBRARY); the
    # plural *_DIRS/*_LIBRARIES names are NOT read by FindZLIB and left zlib undetected,
    # which disabled largeBlob compression. Pass the correct variables.
    "-DZLIB_ROOT=$I", "-DZLIB_INCLUDE_DIR=$I\include", "-DZLIB_LIBRARY=$ZlibLib",
    # keep the old plural ones too (harmless) in case the fork reads them
    "-DZLIB_INCLUDE_DIRS=$I\include", "-DZLIB_LIBRARY_DIRS=$I\lib", "-DZLIB_BIN_DIRS=$I\bin", "-DZLIB_LIBRARIES=$ZlibName"
)
cmake @fidoArgs
if ($LASTEXITCODE) { throw "libfido2 cmake configure failed" }
cmake --build $FBuild --config Release
if ($LASTEXITCODE) { throw "libfido2 build failed" }
$FidoLibDir = Join-Path $FBuild "src\Release"
if (-not (Test-Path "$FidoLibDir\t2fido2.lib")) { throw "t2fido2.lib not produced in $FidoLibDir" }
Copy-Item "$FidoLibDir\t2fido2.dll" "$Portable\" -Force
Get-ChildItem "$FBuild\tools\Release\*.exe" -ErrorAction SilentlyContinue | ForEach-Object { Copy-Item $_.FullName (Join-Path "$Portable" ("token2-" + $_.Name)) -Force }
Get-ChildItem "$I\bin\cbor*.dll" | ForEach-Object { Copy-Item $_.FullName "$Portable\" -Force }
Write-Host "libfido2 ready: $FidoLibDir\t2fido2.dll" -ForegroundColor Green
if ($ToolOnly) { return }

Step "Building Token2 Key Manager (Tauri)"
if (-not (Get-Command cargo-tauri -ErrorAction SilentlyContinue)) {
    cargo install tauri-cli --version "^2" --locked
    if ($LASTEXITCODE) { throw "tauri-cli install failed" }
}
$env:T2PIV_LIB_DIR  = $LibDir
$env:T2PIV_LIB_NAME = "libt2piv"
$env:FIDO2_LIB_DIR  = $FidoLibDir
$env:FIDO2_LIB_NAME = "t2fido2"
# runtime DLLs go next to the exe and into the installer
$GuiDlls = Join-Path $Gui "dlls"
New-Item -ItemType Directory -Force $GuiDlls | Out-Null
$GuiLibs = Join-Path $Gui "libs"
New-Item -ItemType Directory -Force $GuiLibs | Out-Null
Set-Content -Path (Join-Path $GuiLibs ".keep") -Value "" -NoNewline
Copy-Item "$LibDir\libt2piv.dll" $GuiDlls -Force
Copy-Item "$FidoLibDir\t2fido2.dll" $GuiDlls -Force
Get-ChildItem "$I\bin\cbor*.dll" | ForEach-Object { Copy-Item $_.FullName $GuiDlls -Force }
foreach ($d in (RuntimeDlls | Where-Object Name -notlike "getopt*")) { Copy-Item $d.FullName $GuiDlls -Force }
if ($legacy) { Copy-Item $legacy.FullName $GuiDlls -Force }

Push-Location $Gui
try {
    if ($FastDev) {
        New-Item -ItemType Directory -Force "target\debug" | Out-Null
        Copy-Item "$GuiDlls\*.dll" "target\debug\" -Force
        cargo build            # incremental debug; seconds after the first build
        if ($LASTEXITCODE) { throw "GUI build failed" }
        Write-Host "FastDev exe: target\debug\token2-key-manager.exe (run it directly; UI hot-reloads on restart)"
    } elseif ($Dev) {
        New-Item -ItemType Directory -Force "target\debug" | Out-Null
        Copy-Item "$GuiDlls\*.dll" "target\debug\" -Force
        cargo tauri dev
    } else {
        # portable build: compile the GUI (renamed binary) and the fido2-manage CLI,
        # no installer/bundle. Everything lands in the single $Portable folder next to
        # the CLI tools and all shared DLLs.
        cargo build --release --bin token2-key-manager
        if ($LASTEXITCODE) { throw "GUI build failed" }
        cargo build --release --bin fido2-manage
        if ($LASTEXITCODE) { throw "fido2-manage build failed" }
        Copy-Item "target\release\token2-key-manager.exe" $Portable -Force
        Copy-Item "target\release\fido2-manage.exe" $Portable -Force
        # fido2-manage is a CLI tool — remove the GUI icon so Explorer doesn't
        # make it look like the app. Best-effort via rcedit.
        try {
            $rcedit = Join-Path $env:TEMP "rcedit-x64.exe"
            if (-not (Test-Path $rcedit)) {
                Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/electron/rcedit/releases/download/v2.0.0/rcedit-x64.exe" -OutFile $rcedit -ErrorAction Stop
            }
            # a 1x1 transparent/blank ico removes the visible app icon
            $blank = Join-Path $env:TEMP "blank.ico"
            [IO.File]::WriteAllBytes($blank, [byte[]](0,0,1,0,1,0,1,1,0,0,1,0,32,0,48,0,0,0,22,0,0,0,40,0,0,0,1,0,0,0,2,0,0,0,1,0,32,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0))
            & $rcedit (Join-Path $Portable "fido2-manage.exe") --set-icon $blank 2>$null
            Write-Host "stripped icon from fido2-manage.exe"
        } catch { Write-Host "note: could not strip fido2-manage icon ($($_.Exception.Message)) — leaving default" }
        Copy-Item "$GuiDlls\*.dll" $Portable -Force
        Write-Host "`nPortable build ready: $Portable" -ForegroundColor Green
    }
} finally { Pop-Location }
