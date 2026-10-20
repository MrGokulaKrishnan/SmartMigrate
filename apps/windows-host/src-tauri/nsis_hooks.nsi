; Smart Migrate NSIS Installer Hooks
; Handles AppUserModelID registration, App Paths, shortcut icon binding, and shell change notification.

!macro NSIS_HOOK_POSTINSTALL
  ; Register AppUserModelId for Windows Shell, Start Menu, and Windows Search
  WriteRegStr SHCTX "Software\Classes\AppUserModelId\com.smartmigrate.host" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "Software\Classes\AppUserModelId\com.smartmigrate.host" "IconUri" "$INSTDIR\${MAINBINARYNAME}.exe,0"
  WriteRegStr SHCTX "Software\Classes\AppUserModelId\com.smartmigrate.host" "IconBackgroundColor" "0"
  WriteRegDWORD SHCTX "Software\Classes\AppUserModelId\com.smartmigrate.host" "ShowInNewInstall" 1

  ; Register App Paths so Windows Search, Run dialog, and Shell know the exact binary path
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\${MAINBINARYNAME}.exe" "Path" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\SmartMigrate.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\SmartMigrate.exe" "Path" "$INSTDIR"

  ; Ensure Start Menu and Desktop shortcuts have explicit icon location targeting the executable at index 0
  ; and ensure they are created or refreshed on both clean installs and in-place updates
  !if "${STARTMENUFOLDER}" != ""
    CreateDirectory "$SMPROGRAMS\$AppStartMenuFolder"
    CreateShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !else
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !endif

  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
  !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"

  ; Add Windows Firewall rules for Smart Migrate Host (TCP 7890 for SMP/1 and UDP 7889 for discovery)
  nsExec::Exec 'netsh advfirewall firewall add rule name="Smart Migrate TCP 7890" dir=in action=allow protocol=TCP localport=7890 profile=any'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Smart Migrate UDP 7889" dir=in action=allow protocol=UDP localport=7889 profile=any'

  ; Notify Windows Shell of association and icon changes (SHCNE_ASSOCCHANGED = 0x08000000, SHCNF_IDLIST = 0)
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Clean up Windows Firewall rules
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Smart Migrate TCP 7890"'
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Smart Migrate UDP 7889"'

  ; Clean up AppUserModelId and App Paths
  DeleteRegKey SHCTX "Software\Classes\AppUserModelId\com.smartmigrate.host"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\${MAINBINARYNAME}.exe"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\SmartMigrate.exe"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
