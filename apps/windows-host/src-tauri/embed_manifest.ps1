$target = "..\..\target\release\smart-migrate-windows-host.exe"
if (-not (Test-Path $target)) {
    $target = "target\release\smart-migrate-windows-host.exe"
}
if (-not (Test-Path $target)) {
    $target = "..\..\..\target\release\smart-migrate-windows-host.exe"
}

if (-not (Test-Path $target)) {
    Write-Error "Cannot locate smart-migrate-windows-host.exe"
    exit 1
}

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
    version="1.4.0.0"
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

$endRes = $type::EndUpdateResourceW($hUpdate, $false)
if (-not $endRes) {
    Write-Error "EndUpdateResourceW failed: $([System.Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    exit 1
}

Write-Host "Successfully embedded RT_MANIFEST into $fullPath!"
