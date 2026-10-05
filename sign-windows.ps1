<#
  sign-windows.ps1 — sign keyroost Windows .exe files with the EV cert on the hardware token

  Usage:
    .\sign-windows.ps1                      # signs keyroost.exe + keyroostctl.exe in the current dir
    .\sign-windows.ps1 -Files a.exe,b.exe   # sign specific files

  What it does automatically:
    - finds a MODERN signtool.exe from the Windows 10/11 SDK (ignores ancient v6.0A)
    - finds your code-signing cert in the store (the token cert appears here once
      the token middleware is installed and the token is plugged in)
    - signs with SHA256 + RFC3161 timestamp
    - verifies

  Requirements:
    - hardware token plugged in, middleware (e.g. SafeNet Authentication Client) running
    - you'll be prompted for the token PIN during signing
#>

param(
[string[]]$Files = @(
    ".\fido2-manage.exe",
    ".\token2-fido2-assert.exe",
    ".\token2-fido2-cred.exe",
    ".\token2-fido2-token.exe",
    ".\token2-key-manager.exe"
),
  

  [string]$TimestampUrl = "http://timestamp.digicert.com",
  [string]$Thumbprint = ""   # optional: force a specific cert; otherwise auto-detected
)

$ErrorActionPreference = "Stop"

function Die($msg) { Write-Host "ERROR: $msg" -ForegroundColor Red; exit 1 }
function Info($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }

# ---- 1. locate a modern signtool ---------------------------------------------
Info "Locating a modern signtool.exe…"
$candidates = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -match '\\x64\\' } |
  Sort-Object FullName -Descending

if (-not $candidates) {
  Die "No modern signtool found. Install it with:  winget install Microsoft.WindowsSDK.10.0.22621  (or the Windows SDK 'Signing Tools for Desktop Apps')."
}
$signtool = $candidates[0].FullName
Info "Using: $signtool"

# ---- 2. find the signing certificate -----------------------------------------
if (-not $Thumbprint) {
  Info "Auto-detecting a code-signing certificate in the store…"
  # code signing EKU = 1.3.6.1.5.5.7.3.3
  $certs = Get-ChildItem Cert:\CurrentUser\My, Cert:\LocalMachine\My -ErrorAction SilentlyContinue |
    Where-Object {
      $_.EnhancedKeyUsageList.ObjectId -contains "1.3.6.1.5.5.7.3.3" -and
      $_.NotAfter -gt (Get-Date) -and $_.NotBefore -lt (Get-Date)
    }

  if (-not $certs) { Die "No valid code-signing cert found. Is the token plugged in and its middleware running?" }

  if ($certs.Count -gt 1) {
    Info "Multiple code-signing certs found:"
    $i = 0
    $certs | ForEach-Object {
      Write-Host ("  [{0}] {1}  (expires {2})  {3}" -f $i, $_.Subject, $_.NotAfter.ToString("yyyy-MM-dd"), $_.Thumbprint)
      $i++
    }
    $choice = Read-Host "Pick index to use"
    $Thumbprint = $certs[[int]$choice].Thumbprint
  } else {
    $Thumbprint = $certs[0].Thumbprint
    Info ("Using cert: {0}" -f $certs[0].Subject)
  }
}
Info "Thumbprint: $Thumbprint"

# ---- 3. sign ------------------------------------------------------------------
foreach ($f in $Files) {
  if (-not (Test-Path $f)) { Die "file not found: $f" }
}

Info "Signing (you'll be prompted for the token PIN)…"
& $signtool sign /sha1 $Thumbprint /fd SHA256 /tr $TimestampUrl /td SHA256 $Files
if ($LASTEXITCODE -ne 0) { Die "signing failed (exit $LASTEXITCODE)" }

# ---- 4. verify ----------------------------------------------------------------
Info "Verifying…"
& $signtool verify /pa /v $Files
if ($LASTEXITCODE -ne 0) { Die "verification failed (exit $LASTEXITCODE)" }

Write-Host ""
Info "DONE. Signed: $($Files -join ', ')"
