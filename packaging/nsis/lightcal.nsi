Unicode true
RequestExecutionLevel user
ManifestDPIAware true
SetCompressor /SOLID lzma
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

!ifndef VERSION
  !error "VERSION must be supplied by package-nsis.ps1"
!endif
!ifndef STAGE
  !error "STAGE must contain a staged release payload"
!endif
!ifndef OUTPUT
  !error "OUTPUT must name the installer"
!endif

Name "Lightcal"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Lightcal"
!ifndef TEST_SETUP
InstallDirRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "InstallLocation"
!endif
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Lightcal Setup"
VIAddVersionKey "FileDescription" "Lightcal per-user installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-only"
Icon "${STAGE}\lightcal.ico"
UninstallIcon "${STAGE}\lightcal.ico"

!define MUI_ABORTWARNING
!define MUI_ICON "${STAGE}\lightcal.ico"
!define MUI_UNICON "${STAGE}\lightcal.ico"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${STAGE}\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!ifndef TEST_SETUP
!define MUI_FINISHPAGE_RUN "$INSTDIR\lightcal.exe"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!endif
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "Japanese"

Function .onInit
  SetShellVarContext current
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "Lightcal requires 64-bit Windows." /SD IDOK
    SetErrorLevel 1633
    Abort
  ${EndIf}
!ifndef TEST_SETUP
  FindWindow $0 "" "Lightcal"
  ${If} $0 != 0
    MessageBox MB_OK|MB_ICONINFORMATION "Please exit Lightcal using the tray menu, then run Setup again." /SD IDOK
    SetErrorLevel 1603
    Abort
  ${EndIf}
!endif
FunctionEnd

Section "Lightcal (required)" AppFiles
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${STAGE}\lightcal.exe"
  File "${STAGE}\LICENSE"
  File "${STAGE}\THIRD_PARTY_NOTICES.md"
  File "${STAGE}\README.md"
  File "${STAGE}\NSIS-LICENSE.txt"
  WriteUninstaller "$INSTDIR\uninstall.exe"
!ifndef TEST_SETUP
  CreateDirectory "$SMPROGRAMS\Lightcal"
  CreateShortcut "$SMPROGRAMS\Lightcal\Lightcal.lnk" "$INSTDIR\lightcal.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "DisplayName" "Lightcal"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "Publisher" "Lightcal contributors"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "DisplayIcon" "$INSTDIR\lightcal.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "UninstallString" '$\"$INSTDIR\uninstall.exe$\"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "QuietUninstallString" '$\"$INSTDIR\uninstall.exe$\" /S'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "URLInfoAbout" "https://github.com/andmetoo/lightcal"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal" "NoRepair" 1
!endif
SectionEnd

!ifndef TEST_SETUP
Section /o "Desktop shortcut" DesktopShortcut
  CreateShortcut "$DESKTOP\Lightcal.lnk" "$INSTDIR\lightcal.exe"
SectionEnd
!endif

Function un.onInit
  SetShellVarContext current
!ifndef TEST_SETUP
  FindWindow $0 "" "Lightcal"
  ${If} $0 != 0
    MessageBox MB_OK|MB_ICONINFORMATION "Please exit Lightcal using the tray menu before uninstalling." /SD IDOK
    SetErrorLevel 1603
    Abort
  ${EndIf}
!endif
FunctionEnd

Section "Uninstall"
  ; Delete only installer-owned files; never recurse over a user-selected directory.
  Delete "$INSTDIR\lightcal.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\NSIS-LICENSE.txt"
  Delete "$INSTDIR\uninstall.exe"
!ifndef TEST_SETUP
  Delete "$SMPROGRAMS\Lightcal\Lightcal.lnk"
  RMDir "$SMPROGRAMS\Lightcal"
  Delete "$DESKTOP\Lightcal.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal"
!endif
  ; Preferences, DPAPI credentials, and unrelated files are deliberately retained.
  RMDir "$INSTDIR"
SectionEnd
