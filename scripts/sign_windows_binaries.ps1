<#
.SYNOPSIS
    Automates Authenticode code signing and verification for Smart Migrate Windows binaries.
.DESCRIPTION
    Signs SmartMigrate executables and MSI installers using SHA-256 Authenticode
    with an RFC 3161 compliant timestamp server (DigiCert / Sectigo).
    Supports production EV/OV certificates via Thumbprint or PFX, and generates
    a dedicated local test certificate for CI/development validation if none is provided.
.PARAMETER BinaryPath
    Specific binary file or directory containing binaries to sign. Defaults to website downloads.
.PARAMETER Thumbprint
    Certificate thumbprint in Cert:\CurrentUser\My or Cert:\LocalMachine\My.
.PARAMETER PfxPath
    Path to a .pfx or .p12 certificate file.
.PARAMETER PfxPassword
    Secure password for the PFX file.
.PARAMETER CreateTestCertIfMissing
    Generates a trusted local code-signing cert if no production cert is provided.
#>
[CmdletBinding()]
param(
    [string]$BinaryPath,
    [string]$Thumbprint = $env:SM_CERT_THUMBPRINT,
    [string]$PfxPath = $env:SM_PFX_PATH,
    [string]$PfxPassword = $env:SM_PFX_PASSWORD,
    [string]$TimestampServer = "http://timestamp.digicert.com",
    [switch]$CreateTestCertIfMissing
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Smart Migrate Authenticode Code Signing Tool (SEC-01)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Resolve Target Files
$TargetFiles = @()
if ($BinaryPath -and (Test-Path $BinaryPath)) {
    if ((Get-Item $BinaryPath).PSIsContainer) {
        $TargetFiles = Get-ChildItem -Path $BinaryPath -Include *.exe, *.msi, *.dll -Recurse
    } else {
        $TargetFiles = @(Get-Item $BinaryPath)
    }
} else {
    $DefaultPaths = @(
        "apps/website/downloads/windows/SmartMigrate-Setup-x64.exe",
        "apps/website/downloads/windows/SmartMigrate-x64.msi",
        "apps/website/public/downloads/windows/SmartMigrate-Setup-x64.exe",
        "apps/website/public/downloads/windows/SmartMigrate-x64.msi"
    )
    foreach ($p in $DefaultPaths) {
        if (Test-Path $p) {
            $TargetFiles += (Get-Item $p)
        }
    }
}

if ($TargetFiles.Count -eq 0) {
    Write-Warning "No target binaries found to sign."
    return
}

# 2. Resolve Certificate
$Cert = $null

if ($PfxPath -and (Test-Path $PfxPath)) {
    Write-Host "[+] Loading certificate from PFX: $PfxPath" -ForegroundColor Green
    $SecurePass = if ($PfxPassword) { ConvertTo-SecureString $PfxPassword -AsPlainText -Force } else { New-Object System.Security.SecureString }
    $Cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($PfxPath, $SecurePass)
} elseif ($Thumbprint) {
    Write-Host "[+] Resolving certificate by thumbprint: $Thumbprint" -ForegroundColor Green
    $Cert = Get-Item "Cert:\CurrentUser\My\$Thumbprint" -ErrorAction SilentlyContinue
    if (-not $Cert) {
        $Cert = Get-Item "Cert:\LocalMachine\My\$Thumbprint" -ErrorAction SilentlyContinue
    }
}

if (-not $Cert -and $CreateTestCertIfMissing) {
    Write-Host "[!] No certificate provided. Generating Smart Migrate local test certificate..." -ForegroundColor Yellow
    $Cert = New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject "CN=Smart Migrate Development Authority, O=Smart Migrate, C=US" `
        -KeyAlgorithm RSA `
        -KeyLength 3072 `
        -HashAlgorithm SHA256 `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -NotAfter (Get-Date).AddYears(3)
    Write-Host "[+] Generated test certificate with thumbprint: $($Cert.Thumbprint)" -ForegroundColor Green
}

if (-not $Cert) {
    Write-Error "No valid code signing certificate found. Provide -Thumbprint, -PfxPath, or -CreateTestCertIfMissing."
    return
}

Write-Host "[+] Certificate Subject: $($Cert.Subject)" -ForegroundColor Gray
Write-Host "[+] Certificate Thumbprint: $($Cert.Thumbprint)" -ForegroundColor Gray
Write-Host "[+] Timestamp Server: $TimestampServer" -ForegroundColor Gray

# 3. Sign Binaries
$SuccessCount = 0
$FailCount = 0

foreach ($File in $TargetFiles) {
    Write-Host "`n---> Signing: $($File.FullName)" -ForegroundColor Cyan
    try {
        $Sig = Set-AuthenticodeSignature `
            -FilePath $File.FullName `
            -Certificate $Cert `
            -TimestampServer $TimestampServer `
            -HashAlgorithm SHA256

        Write-Host "     Signature Status: $($Sig.Status)" -ForegroundColor $(if ($Sig.Status -eq 'Valid' -or $Sig.Status -eq 'UnknownError') { 'Green' } else { 'Yellow' })
        Write-Host "     Signer: $($Sig.SignerCertificate.Subject)" -ForegroundColor Gray

        # Verify
        $Verify = Get-AuthenticodeSignature -FilePath $File.FullName
        Write-Host "     Verified: $($Verify.Status) ($($Verify.StatusMessage))" -ForegroundColor Gray
        $SuccessCount++
    } catch {
        Write-Host "     Signing Error: $_" -ForegroundColor Red
        $FailCount++
    }
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host " Signing Summary: $SuccessCount succeeded, $FailCount failed." -ForegroundColor $(if ($FailCount -eq 0) { 'Green' } else { 'Red' })
Write-Host "==========================================================" -ForegroundColor Cyan
