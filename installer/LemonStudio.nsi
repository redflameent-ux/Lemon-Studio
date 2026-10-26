; Lemon Studio - Windows Setup
; Unofficial build of https://github.com/ItsAlphaNeon/Lemon-Studio (GPL-3.0-or-later)

!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.1"
!endif
!ifndef SRC
  !define SRC "..\stage"
!endif

Name "Lemon Studio"
OutFile "LemonStudio-Setup-${VERSION}.exe"
Unicode True
RequestExecutionLevel admin
InstallDir "$PROGRAMFILES64\Lemon Studio"
InstallDirRegKey HKLM "Software\Lemon Studio" "InstallDir"
SetCompressor /SOLID lzma

!define MUI_ICON "${SRC}\lemonstudio.ico"
!define MUI_UNICON "${SRC}\lemonstudio.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\lemonstudio.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Open Lemon Studio"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${SRC}\LICENSE.txt"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Lemon Studio (required)" SecApp
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${SRC}\lemonstudio.exe"
  File "${SRC}\lemonstudio.ico"
  File "${SRC}\LICENSE.txt"
  File "${SRC}\THIRD-PARTY-NOTICES.txt"

  CreateShortcut "$SMPROGRAMS\Lemon Studio.lnk" "$INSTDIR\lemonstudio.exe" "" "$INSTDIR\lemonstudio.ico"
  CreateShortcut "$DESKTOP\Lemon Studio.lnk" "$INSTDIR\lemonstudio.exe" "" "$INSTDIR\lemonstudio.ico"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKLM "Software\Lemon Studio" "InstallDir" "$INSTDIR"
  !define UNINST "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lemon Studio"
  WriteRegStr HKLM "${UNINST}" "DisplayName" "Lemon Studio"
  WriteRegStr HKLM "${UNINST}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST}" "Publisher" "ItsAlphaNeon (unofficial build)"
  WriteRegStr HKLM "${UNINST}" "DisplayIcon" "$INSTDIR\lemonstudio.ico"
  WriteRegStr HKLM "${UNINST}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKLM "${UNINST}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST}" "NoRepair" 1
SectionEnd

Section "Lemon plugins for FL Studio and other DAWs (VST3 + CLAP)" SecPlugins
  SetOutPath "$COMMONFILES64\VST3"
  File /r "${SRC}\plugins\*.vst3"
  SetOutPath "$COMMONFILES64\CLAP"
  File /r "${SRC}\plugins\*.clap"
  ; Remember which plugins we installed so the uninstaller only removes those
  SetOutPath "$INSTDIR"
  File "${SRC}\plugins.txt"
SectionEnd

LangString DESC_App ${LANG_ENGLISH} "The Lemon Studio program, with Start menu and desktop shortcuts."
LangString DESC_Plugins ${LANG_ENGLISH} "Installs Lemon Studio's own effects and instruments so FL Studio and other DAWs can use them. Lemon Studio itself doesn't need these."
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} $(DESC_App)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecPlugins} $(DESC_Plugins)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section "Uninstall"
  ; Remove only the plugin bundles this installer put in place
  !include "${SRC}\uninstall-plugins.nsh"
  Delete "$INSTDIR\plugins.txt"
  Delete "$INSTDIR\lemonstudio.exe"
  Delete "$INSTDIR\lemonstudio.ico"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\THIRD-PARTY-NOTICES.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Lemon Studio.lnk"
  Delete "$DESKTOP\Lemon Studio.lnk"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lemon Studio"
  DeleteRegKey HKLM "Software\Lemon Studio"
SectionEnd
