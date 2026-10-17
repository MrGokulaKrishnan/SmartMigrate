# Locate the freshly built host exe regardless of the working directory or --target triple.
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$candidates = @(
    (Join-Path $repoRoot "target\release\smart-migrate-windows-host.exe"),
    (Join-Path $repoRoot "target\x86_64-pc-windows-gnu\release\smart-migrate-windows-host.exe"),
    (Join-Path $repoRoot "target\x86_64-pc-windows-msvc\release\smart-migrate-windows-host.exe"),
    (Join-Path $PSScriptRoot "target\release\smart-migrate-windows-host.exe")
) | Where-Object { Test-Path $_ }

if (-not $candidates) {
    Write-Error "Cannot locate smart-migrate-windows-host.exe"
    exit 1
}
$target = ($candidates | Sort-Object { (Get-Item $_).LastWriteTime } -Descending | Select-Object -First 1)

$fullPath = (Resolve-Path $target).Path
Write-Host "Embedding RT_MANIFEST into: $fullPath"

$signature = @"
[DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern IntPtr BeginUpdateResourceW(string pFileName, bool bDeleteExistingResources);

[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool UpdateResourceW(
    IntPtr hUpdate,
    IntPtr lpType,
    IntPtr lpName,
    ushort wLanguage,
    byte[] lpData,
    uint cb
);

[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool EndUpdateResourceW(IntPtr hUpdate, bool fDiscard);
"@

$type = Add-Type -MemberDefinition $signature -Name "WinResource" -Namespace "SmartMigrate" -PassThru

$manifest = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="1.4.2.0"
    processorArchitecture="*"
    name="SmartMigrate.WindowsHost"
    type="win32"
  />
  <description>Smart Migrate Windows Host</description>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">permonitorv2,permonitor</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>
"@

$bytes = [System.Text.Encoding]::UTF8.GetBytes($manifest)
$hUpdate = $type::BeginUpdateResourceW($fullPath, $false)
if ($hUpdate -eq [IntPtr]::Zero) {
    Write-Error "BeginUpdateResourceW failed: $([System.Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    exit 1
}

# RT_MANIFEST = 24, CREATEPROCESS_MANIFEST_RESOURCE_ID = 1, Neutral = 0
$res = $type::UpdateResourceW($hUpdate, [IntPtr]24, [IntPtr]1, 0, $bytes, [uint32]$bytes.Length)
if (-not $res) {
    $err = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
    $type::EndUpdateResourceW($hUpdate, $true) | Out-Null
    Write-Error "UpdateResourceW failed: $err"
    exit 1
}

# ── Application icon: RT_ICON (3) per image + RT_GROUP_ICON (14) ──
# The GNU/lld build does not carry tauri-build's icon resource into the exe, so Explorer, shortcuts and the
# taskbar showed the generic Windows icon. Embed icons\icon.ico directly.
$icoPath = Join-Path $PSScriptRoot "icons\icon.ico"
if (-not (Test-Path $icoPath)) {
    $type::EndUpdateResourceW($hUpdate, $true) | Out-Null
    Write-Error "Missing icon file: $icoPath"
    exit 1
}
$ico = [System.IO.File]::ReadAllBytes($icoPath)
$iconCount = [int][BitConverter]::ToUInt16($ico, 4)
$group = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($group)
$bw.Write([uint16]0)           # reserved
$bw.Write([uint16]1)           # type = icon
$bw.Write([uint16]$iconCount)
for ($i = 0; $i -lt $iconCount; $i++) {
    $entry  = 6 + (16 * $i)
    $size   = [int][BitConverter]::ToUInt32($ico, $entry + 8)
    $offset = [int][BitConverter]::ToUInt32($ico, $entry + 12)
    $img = New-Object byte[] $size
    [Array]::Copy($ico, $offset, $img, 0, $size)
    $iconId = $i + 1
    $ok = $type::UpdateResourceW($hUpdate, [IntPtr]3, [IntPtr]$iconId, 1033, $img, [uint32]$size)
    if (-not $ok) {
        $err = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
        $type::EndUpdateResourceW($hUpdate, $true) | Out-Null
        Write-Error "UpdateResourceW(RT_ICON $iconId) failed: $err"
        exit 1
    }
    $bw.Write([byte]$ico[$entry])         # width  (0 = 256)
    $bw.Write([byte]$ico[$entry + 1])     # height (0 = 256)
    $bw.Write([byte]$ico[$entry + 2])     # colour count
    $bw.Write([byte]0)                    # reserved
    $bw.Write([uint16][BitConverter]::ToUInt16($ico, $entry + 4))   # planes
    $bw.Write([uint16][BitConverter]::ToUInt16($ico, $entry + 6))   # bit count
    $bw.Write([uint32]$size)                                        # bytes in resource
    $bw.Write([uint16]$iconId)                                      # RT_ICON id
}
$bw.Flush()
$groupBytes = $group.ToArray()
$ok = $type::UpdateResourceW($hUpdate, [IntPtr]14, [IntPtr]1, 1033, $groupBytes, [uint32]$groupBytes.Length)
if (-not $ok) {
    $err = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
    $type::EndUpdateResourceW($hUpdate, $true) | Out-Null
    Write-Error "UpdateResourceW(RT_GROUP_ICON) failed: $err"
    exit 1
}
Write-Host "Embedded $iconCount icon image(s) from $icoPath"

$endRes = $type::EndUpdateResourceW($hUpdate, $false)
if (-not $endRes) {
    Write-Error "EndUpdateResourceW failed: $([System.Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    exit 1
}

Write-Host "Successfully embedded RT_MANIFEST + icon into $fullPath!"
