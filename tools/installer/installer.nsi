; Constellations Windows Installer (NSIS Script, P9-01, EDGE_CASES §13).
; Builds a user-level installer (no admin rights needed), installs to %LOCALAPPDATA%\Programs\Constellations.

Unicode true
!define PRODUCT_NAME "Constellations"
!define PRODUCT_VERSION "0.1.0"
!define PRODUCT_PUBLISHER "Constellations Authors"
!define PRODUCT_WEB_SITE "https://github.com/13LA7E/Odys-Brows"
!define PRODUCT_DIR_REGKEY "Software\Microsoft\Windows\CurrentVersion\App Paths\constellations.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "..\..\out\installer\Constellations-Setup-${PRODUCT_VERSION}.exe"
InstallDir "$LOCALAPPDATA\Programs\Constellations"
InstallDirRegKey HKCU "${PRODUCT_DIR_REGKEY}" ""
RequestExecutionLevel user

!include "MUI2.nsh"

; Interface Settings
!define MUI_ABORTWARNING
!define MUI_ICON "constellations.ico"
!define MUI_UNICON "constellations.ico"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\constellations.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Start Constellations now"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "MainSection" SEC01
  SetOutPath "$INSTDIR"
  SetOverwrite try

  ; Payload staged in out\installer\payload
  File /r "..\..\out\installer\payload\*.*"

  ; Create shortcuts
  CreateDirectory "$SMPROGRAMS\Constellations"
  CreateShortcut "$SMPROGRAMS\Constellations\Constellations.lnk" "$INSTDIR\constellations.exe"
  CreateShortcut "$SMPROGRAMS\Constellations\Uninstall.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut "$DESKTOP\Constellations.lnk" "$INSTDIR\constellations.exe"

  ; Register in Add/Remove Programs
  WriteRegStr HKCU "${PRODUCT_DIR_REGKEY}" "" "$INSTDIR\constellations.exe"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\constellations.exe"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"

  WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\Constellations.lnk"
  Delete "$SMPROGRAMS\Constellations\Constellations.lnk"
  Delete "$SMPROGRAMS\Constellations\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Constellations"

  ; Ask user whether to remove personal profile data (EDGE_CASES §13)
  MessageBox MB_YESNO|MB_ICONQUESTION "Do you want to delete your browsing profile, bookmarks, and settings?" IDNO keep_profile
    RMDir /r "$LOCALAPPDATA\Constellations"
  keep_profile:

  RMDir /r "$INSTDIR"

  DeleteRegKey HKCU "${PRODUCT_UNINST_KEY}"
  DeleteRegKey HKCU "${PRODUCT_DIR_REGKEY}"
SectionEnd
