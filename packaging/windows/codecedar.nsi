Unicode true
!include "MUI2.nsh"
!include "Sections.nsh"
!ifndef PAYLOAD
!error "Pass /DPAYLOAD=absolute-payload-directory"
!endif
!ifndef OUTFILE
!define OUTFILE "CodeCedar-Setup.exe"
!endif
Name "CodeCedar"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Kedr\CodeCedar"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ICON "${PAYLOAD}\codecedar.ico"
!define MUI_UNICON "${PAYLOAD}\codecedar.ico"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "Welcome to CodeCedar Setup"
!define MUI_WELCOMEPAGE_TEXT "Install the free Kedr desktop editor preview for your account. Other products are not released. This unsigned preview does not claim JetBrains feature parity."
!define MUI_FINISHPAGE_RUN "$INSTDIR\kedr-studio.exe"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${PAYLOAD}\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Section "CodeCedar (required)" SEC_CORE
  SectionIn RO
  SetShellVarContext current
  !include "${PAYLOAD}\check-destination.nsh"
  SetOutPath "$INSTDIR"
  ClearErrors
  File /r "${PAYLOAD}\app\*"
  IfErrors install_failed installed_payload
  install_failed:
  SetErrorLevel 2
  Abort
  installed_payload:
  File "${PAYLOAD}\codecedar.ico"
  FileOpen $0 "$INSTDIR\kedr-owned.txt" w
  FileWrite $0 "CodeCedar preview installation"
  FileClose $0
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Kedr"
  CreateShortcut "$SMPROGRAMS\Kedr\CodeCedar.lnk" "$INSTDIR\kedr-studio.exe" "" "$INSTDIR\codecedar.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "DisplayName" "CodeCedar (Kedr preview)"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "DisplayVersion" "0.1.0-preview"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "Publisher" "Kedr"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "DisplayIcon" "$INSTDIR\codecedar.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar" "NoRepair" 1
SectionEnd
Section /o "Desktop shortcut" SEC_DESKTOP
  CreateShortcut "$DESKTOP\CodeCedar.lnk" "$INSTDIR\kedr-studio.exe" "" "$INSTDIR\codecedar.ico"
SectionEnd
Section /o "Other products (not released)" SEC_PLANNED
SectionEnd
Function .onInit
  SectionSetFlags ${SEC_PLANNED} ${SF_RO}
FunctionEnd
Section "Uninstall"
  SetShellVarContext current
  !include "${PAYLOAD}\uninstall-files.nsh"
  Delete "$INSTDIR\codecedar.ico"
  Delete "$INSTDIR\kedr-owned.txt"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$SMPROGRAMS\Kedr\CodeCedar.lnk"
  RMDir "$SMPROGRAMS\Kedr"
  Delete "$DESKTOP\CodeCedar.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar"
  RMDir "$INSTDIR"
SectionEnd
