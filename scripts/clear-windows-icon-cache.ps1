<#
.SYNOPSIS
  Cleans and refreshes the Windows Shell, Start Menu, and Windows Search icon caches
  according to Microsoft troubleshooting procedures for Windows 10/11.

.DESCRIPTION
  This script safely stops the Shell Experience, Search Host, and File Explorer,
  purges stale icon caches (including Microsoft.Windows.Search AppIconCache entries,
  Explorer iconcache_*.db files, and legacy IconCache.db), calls ie4uinit to rebuild
  system icon caches, restarts File Explorer, and broadcasts SHCNE_ASSOCCHANGED.
#>

param(
  [switch]$Quiet
)

$ErrorActionPreference = "Continue"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Smart Migrate - Windows Shell & Search Icon Cache Cleaner" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Stop host application if running
Write-Host "[1/6] Stopping any running Smart Migrate instances..." -ForegroundColor Yellow
Get-Process -Name "*smart-migrate*", "*SmartMigrate*" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

# 2. Stop SearchHost and StartMenuExperienceHost
Write-Host "[2/6] Stopping SearchHost and Shell hosts..." -ForegroundColor Yellow
$shellProcesses = @("SearchHost", "StartMenuExperienceHost", "ShellExperienceHost", "SearchApp")
foreach ($p in $shellProcesses) {
  Stop-Process -Name $p -Force -ErrorAction SilentlyContinue
}

# 3. Stop File Explorer
Write-Host "[3/6] Stopping Windows Explorer..." -ForegroundColor Yellow
Stop-Process -Name "explorer" -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

try {
  # 4. Clear Windows Search AppIconCache
  Write-Host "[4/6] Purging stale AppIconCache entries..." -ForegroundColor Yellow
  $searchPackages = Get-ChildItem "$env:LOCALAPPDATA\Packages" -Filter "Microsoft.Windows.Search*" -ErrorAction SilentlyContinue
  foreach ($pkg in $searchPackages) {
    $appIconCachePath = Join-Path $pkg.FullName "LocalState\AppIconCache"
    if (Test-Path $appIconCachePath) {
      Get-ChildItem -Path $appIconCachePath -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object {
        if ($_.Name -like "*com_smartmigrate_host*" -or $_.Name -like "*smart*migrate*") {
          Write-Host "  Removing stale search icon cache: $($_.FullName)" -ForegroundColor DarkGray
          Remove-Item -LiteralPath $_.FullName -Force -ErrorAction SilentlyContinue
        }
      }
    }
  }

  # 5. Clear Explorer icon caches
  Write-Host "[5/6] Clearing Explorer icon cache databases..." -ForegroundColor Yellow
  $legacyCache = "$env:LOCALAPPDATA\IconCache.db"
  if (Test-Path $legacyCache) {
    Remove-Item -Path $legacyCache -Force -ErrorAction SilentlyContinue
  }

  $explorerDir = "$env:LOCALAPPDATA\Microsoft\Windows\Explorer"
  if (Test-Path $explorerDir) {
    Get-ChildItem -Path $explorerDir -Filter "iconcache_*.db" -File -ErrorAction SilentlyContinue | ForEach-Object {
      Remove-Item -LiteralPath $_.FullName -Force -ErrorAction SilentlyContinue
    }
  }

  # Run ie4uinit to rebuild system icon caches
  if (Test-Path "$env:SystemRoot\System32\ie4uinit.exe") {
    Start-Process -FilePath "$env:SystemRoot\System32\ie4uinit.exe" -ArgumentList "-show" -Wait -WindowStyle Hidden -ErrorAction SilentlyContinue
  }
}
finally {
  # 6. Restart File Explorer
  Write-Host "[6/6] Restarting Windows Explorer..." -ForegroundColor Green
  if (-not (Get-Process -Name "explorer" -ErrorAction SilentlyContinue)) {
    Start-Process "explorer.exe"
    Start-Sleep -Seconds 1
  }

  # Broadcast SHCNE_ASSOCCHANGED to all shell windows
  Add-Type -TypeDefinition @"
    using System;
    using System.Runtime.InteropServices;
    public class ShellNotifier {
      [DllImport("shell32.dll", CharSet = CharSet.Auto, SetLastError = true)]
      public static extern void SHChangeNotify(uint wEventId, uint uFlags, IntPtr dwItem1, IntPtr dwItem2);
    }
"@ -ErrorAction SilentlyContinue

  [ShellNotifier]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
  Write-Host "Shell change notification broadcasted (SHCNE_ASSOCCHANGED)." -ForegroundColor Green
}

Write-Host "Icon cache cleanup complete." -ForegroundColor Cyan
