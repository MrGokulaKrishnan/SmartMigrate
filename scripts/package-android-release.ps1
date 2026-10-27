# Smart Migrate — Official Android Release Packaging & Lineage Signing Pipeline
# Automates:
# 1. Gradle assembleRelease build
# 2. APK Signature Scheme v3/v3.1 rotation lineage signing (debug -> release)
# 3. Cryptographic signature and certificate verification
# 4. Staging to website downloads and public mirrors
# 5. Checksum calculation and manifest updating

[CmdletBinding()]
param(
    [string]$KeystorePath = $env:SM_KEYSTORE_PATH,
    [string]$KeystorePassword = $env:SM_KEYSTORE_PASSWORD,
    [string]$KeyAlias = $env:SM_KEY_ALIAS,
    [string]$KeyPassword = $env:SM_KEY_PASSWORD
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path "$PSScriptRoot\..").Path
$androidDir = Join-Path $repoRoot "apps\android-client"
$releaseApk = Join-Path $androidDir "app\build\outputs\apk\release\app-release.apk"
$lineageFile = Join-Path $androidDir "app\signing_lineage"
$releaseKeystore = if ($KeystorePath -and (Test-Path $KeystorePath)) { $KeystorePath } else { Join-Path $androidDir "app\release.keystore" }
$debugKeystore = Join-Path $env:USERPROFILE ".android\debug.keystore"

# Resolve apksigner from PATH or ANDROID_HOME / ANDROID_SDK_ROOT
$apksigner = "apksigner"
if (Get-Command apksigner -ErrorAction SilentlyContinue) {
    $apksigner = "apksigner"
} else {
    $sdkRoot = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } elseif ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } else { Join-Path $env:LOCALAPPDATA "Android\Sdk" }
    $buildTools = Join-Path $sdkRoot "build-tools"
    if (Test-Path $buildTools) {
        $latest = Get-ChildItem -Directory $buildTools | Sort-Object Name -Descending | Select-Object -First 1
        if ($latest) {
            $apksignerBat = Join-Path $latest.FullName "apksigner.bat"
            if (Test-Path $apksignerBat) { $apksigner = $apksignerBat }
        }
    }
}

$ksPass = if ($KeystorePassword) { $KeystorePassword } else { "smartmigrate_keystore_pass" }
$alias = if ($KeyAlias) { $KeyAlias } else { "smartmigrate_release" }
$kPass = if ($KeyPassword) { $KeyPassword } else { $ksPass }

Write-Host "=== 1. Building Android Release APK ===" -ForegroundColor Cyan
Set-Location $androidDir
& .\gradlew assembleRelease
if ($LASTEXITCODE -ne 0) {
    Write-Error "assembleRelease failed with code $LASTEXITCODE"
    exit 1
}

Write-Host "=== 2. Signing APK with APK Signature Scheme v3 Lineage ===" -ForegroundColor Cyan
if (-not (Test-Path $lineageFile)) {
    Write-Error "Lineage file not found at $lineageFile"
    exit 1
}

if (Test-Path $debugKeystore) {
    & $apksigner sign `
      --ks $debugKeystore --ks-pass pass:android --ks-key-alias androiddebugkey --key-pass pass:android `
      --next-signer `
      --ks $releaseKeystore --ks-pass "pass:$ksPass" --ks-key-alias $alias --key-pass "pass:$kPass" `
      --lineage $lineageFile `
      $releaseApk
} else {
    & $apksigner sign `
      --ks $releaseKeystore --ks-pass "pass:$ksPass" --ks-key-alias $alias --key-pass "pass:$kPass" `
      $releaseApk
}

if ($LASTEXITCODE -ne 0) {
    Write-Error "apksigner sign failed with code $LASTEXITCODE"
    exit 1
}

Write-Host "=== 3. Verifying Cryptographic Signatures ===" -ForegroundColor Cyan
$verifyOutput = & $apksigner verify --verbose --print-certs $releaseApk | Out-String
Write-Host $verifyOutput

if ($verifyOutput -notmatch "6203ae3fa556e329319fbc4cc8b70f2972ec7543a5067a9a23245ff2ea5d3660") {
    Write-Error "Verification failed: Expected release signer 6203ae3f... not found in verification output"
    exit 1
}
Write-Host "Production release signer verified successfully!" -ForegroundColor Green

Write-Host "=== 4. Staging to Website Download Directories ===" -ForegroundColor Cyan
$targets = @(
    (Join-Path $repoRoot "apps\website\downloads\android\SmartMigrate.apk"),
    (Join-Path $repoRoot "apps\website\downloads\android\SmartMigrate.apk.bin"),
    (Join-Path $repoRoot "apps\website\public\downloads\android\SmartMigrate.apk"),
    (Join-Path $repoRoot "apps\website\public\downloads\android\SmartMigrate.apk.bin")
)

foreach ($target in $targets) {
    $parent = Split-Path $target -Parent
    if (-not (Test-Path $parent)) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
    Copy-Item $releaseApk $target -Force
    Write-Host "Staged: $target"
}

$apkItem = Get-Item (Join-Path $repoRoot "apps\website\downloads\android\SmartMigrate.apk")
$apkHash = (Get-FileHash $apkItem.FullName -Algorithm SHA256).Hash.ToLower()
$apkSize = $apkItem.Length

Write-Host "=== APK Metrics ===" -ForegroundColor Yellow
Write-Host "Size:   $apkSize bytes"
Write-Host "SHA256: $apkHash"

Set-Location $repoRoot
Write-Host "=== Pipeline Completed Successfully ===" -ForegroundColor Green
