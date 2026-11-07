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
!ifndef TEST_SETUP
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE DirectoryLeave
!endif
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
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "French"
!insertmacro MUI_LANGUAGE "PortugueseBR"
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "Ukrainian"
!insertmacro MUI_LANGUAGE "Japanese"
!insertmacro MUI_LANGUAGE "Korean"
!insertmacro MUI_LANGUAGE "SimpChinese"

; Sets $0 to 0 when $INSTDIR\lightcal.exe can be replaced, 1 while Lightcal
; holds its single-instance mutex, 2 while the exe stays in use, and 3 when
; this account cannot write it. Quit releases the mutex before the process has
; exited (ADR 0017), so the exe is also opened for writing, which Windows
; refuses with a sharing violation while a process runs it. Only that error is
; retried, for 5 s, so a closing Lightcal or a short antivirus or indexer lock
; passes. The probe changes nothing: a read-only exe counts as replaceable,
; since the install and uninstall sections clear the attribute themselves.
!macro LightcalRunning
  StrCpy $0 0
  System::Call 'kernel32::OpenMutexW(i 0x00100000, i 0, w "Local\Lightcal_SingleInstance_Mutex") p .r1'
  ${If} $1 != 0
    System::Call 'kernel32::CloseHandle(p r1)'
    StrCpy $0 1
  ${ElseIf} ${FileExists} "$INSTDIR\lightcal.exe"
    StrCpy $2 0
    ${Do}
      ; GENERIC_WRITE, share read/write/delete, OPEN_EXISTING.
      System::Call 'kernel32::CreateFileW(w "$INSTDIR\lightcal.exe", i 0x40000000, i 7, p 0, i 3, i 0, p 0) p .r1 ?e'
      Pop $3
      ${If} $1 <> -1
        System::Call 'kernel32::CloseHandle(p r1)'
        StrCpy $0 0
        ${Break}
      ${EndIf}
      ${If} $3 == 5
        ; ERROR_ACCESS_DENIED: a read-only attribute, or a folder this account cannot write.
        System::Call 'kernel32::GetFileAttributesW(w "$INSTDIR\lightcal.exe") i .r3'
        IntOp $3 $3 & 1
        ${If} $3 == 1
          StrCpy $0 0
        ${Else}
          StrCpy $0 3
        ${EndIf}
        ${Break}
      ${ElseIf} $3 != 32
        ; Anything but ERROR_SHARING_VIOLATION will not clear by waiting.
        StrCpy $0 3
        ${Break}
      ${EndIf}
      StrCpy $0 2
      IntOp $2 $2 + 1
      ${If} $2 >= 10
        ${Break}
      ${EndIf}
      Sleep 500
    ${Loop}
  ${EndIf}
!macroend

; Shows why Setup cannot continue when $0 is not 0.
!macro LightcalRunningMessage ACTION
  ${If} $0 == 1
    MessageBox MB_OK|MB_ICONINFORMATION "Please exit Lightcal using the tray menu, then ${ACTION}." /SD IDOK
  ${ElseIf} $0 == 2
    MessageBox MB_OK|MB_ICONINFORMATION "$INSTDIR\lightcal.exe is still in use. If Lightcal is closing, wait a moment, then ${ACTION}." /SD IDOK
  ${ElseIf} $0 == 3
    MessageBox MB_OK|MB_ICONSTOP "This account cannot change $INSTDIR\lightcal.exe. Choose another folder, or remove that installation with the account that installed it." /SD IDOK
  ${EndIf}
!macroend

!ifndef TEST_SETUP
; The Directory page can point Setup at another installation, so check again there.
Function DirectoryLeave
  !insertmacro LightcalRunning
  ${If} $0 != 0
    !insertmacro LightcalRunningMessage "continue"
    Abort
  ${EndIf}
FunctionEnd
!endif

Function .onInit
  SetShellVarContext current
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "Lightcal requires 64-bit Windows." /SD IDOK
    SetErrorLevel 1633
    Abort
  ${EndIf}
!ifndef TEST_SETUP
  !insertmacro LightcalRunning
  ${If} $0 != 0
    !insertmacro LightcalRunningMessage "run Setup again"
    SetErrorLevel 1603
    Abort
  ${EndIf}
!endif
FunctionEnd

Section "Lightcal (required)" AppFiles
  SectionIn RO
  SetOutPath "$INSTDIR"
  ; A read-only exe from an earlier copy would block the overwrite.
  ${If} ${FileExists} "$INSTDIR\lightcal.exe"
    SetFileAttributes "$INSTDIR\lightcal.exe" NORMAL
  ${EndIf}
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

; Runs last. An in-place upgrade keeps the same exe path, so Explorer and
; taskbar pins would keep showing the previous version's cached icon.
; SHCNE_UPDATEITEM (0x2000) with SHCNF_PATHW|SHCNF_FLUSH (0x1005) redraws each
; item; SHCNE_ASSOCCHANGED (0x08000000) then asks the shell to flush its
; icon cache. SHCNF_FLUSH waits until the shell has handled each event.
Section "-Refresh shell icons"
  System::Call 'shell32::SHChangeNotify(i 0x2000, i 0x1005, w "$INSTDIR\lightcal.exe", p 0)'
  System::Call 'shell32::SHChangeNotify(i 0x2000, i 0x1005, w "$SMPROGRAMS\Lightcal\Lightcal.lnk", p 0)'
  ${If} ${FileExists} "$DESKTOP\Lightcal.lnk"
    System::Call 'shell32::SHChangeNotify(i 0x2000, i 0x1005, w "$DESKTOP\Lightcal.lnk", p 0)'
  ${EndIf}
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, p 0, p 0)'
SectionEnd
!endif

Function un.onInit
  SetShellVarContext current
!ifndef TEST_SETUP
  !insertmacro LightcalRunning
  ${If} $0 != 0
    !insertmacro LightcalRunningMessage "uninstall again"
    SetErrorLevel 1603
    Abort
  ${EndIf}
!endif
FunctionEnd

Section "Uninstall"
  ; Delete only installer-owned files; never recurse over a user-selected directory.
  ${If} ${FileExists} "$INSTDIR\lightcal.exe"
    SetFileAttributes "$INSTDIR\lightcal.exe" NORMAL
  ${EndIf}
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
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Lightcal"
  ${If} $0 == '$\"$INSTDIR\lightcal.exe$\" --tray'
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Lightcal"
  ${EndIf}
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lightcal"
!endif
  ; Preferences, DPAPI credentials, and unrelated files are deliberately retained.
  RMDir "$INSTDIR"
SectionEnd
