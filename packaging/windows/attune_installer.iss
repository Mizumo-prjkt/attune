; Script generated for Attune Music Player (Native Rust Engine & Desktop Suite)
; Inno Setup 6.x Installer Script for Windows 64-bit

#ifndef MyAppVersion
#define MyAppVersion "0.1.1-rc1"
#endif

#ifndef MyAppLocalVersion
#define MyAppLocalVersion ""
#endif

#ifndef BuildMode
#define BuildMode "Release"
#endif

#define MyAppName "Attune"
#define MyAppPublisher "Mizumo-prjkt"
#define MyAppURL "https://github.com/mizproject/attune"
#define MyAppExeName "attune-desktop.exe"
#define LegacyFlutterExeName "attune_app.exe"

#ifndef SignToolName
#define SignToolName ""
#endif

; Force unconditional upgrade of all native DLLs and executables, even if read-only,
; lacking PE VERSIONINFO headers, or locked by a running background process.
#define DllUpgradeFlags "replacesameversion comparetimestamp overwritereadonly restartreplace uninsremovereadonly ignoreversion"
#define BinarySignFlags "replacesameversion comparetimestamp overwritereadonly restartreplace uninsremovereadonly ignoreversion"
#if SignToolName != ""
#define BinarySignFlags "replacesameversion comparetimestamp overwritereadonly restartreplace uninsremovereadonly ignoreversion signonce"
#endif

[Setup]
; NOTE: The value of AppId uniquely identifies this application. Do not use the same AppId value in installers for other applications.
AppId={{5F8A2D39-6B3C-4E7B-9A5C-7E4A81E29F4C}
AppName={#MyAppName}
AppVersion={#MyAppVersion}{#MyAppLocalVersion}
AppVerName={#MyAppName} {#MyAppVersion}{#MyAppLocalVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
; Automatically close running instances so locked .dll components are cleanly upgraded
CloseApplications=force
CloseApplicationsFilter=*.exe,*.dll
RestartApplications=yes
#if SignToolName != ""
SignTool={#SignToolName}
SignedUninstaller=yes
#endif
; Display Patch Notes from commit log before install
#if FileExists("..\..\patch_notes.txt")
InfoBeforeFile=..\..\patch_notes.txt
#endif
; Open-source MIT License disclosure
#if FileExists("..\..\LICENSE")
LicenseFile=..\..\LICENSE
#endif
OutputDir=..\..\dist\windows
OutputBaseFilename=Attune-{#MyAppVersion}{#MyAppLocalVersion}-Windows-{#BuildMode}-Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#MyAppExeName}
#if FileExists("..\..\packaging\windows\app_icon.ico")
SetupIconFile=..\..\packaging\windows\app_icon.ico
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "cleanflutterdata"; Description: "Purge incompatible legacy Flutter cache && preferences data (Recommended when upgrading from Flutter version)"; GroupDescription: "Migration & Upgrade:"; Flags: checkedonce

[InstallDelete]
; Explicitly remove previous DLLs and manifests before unpacking to guarantee 100% fresh signed binaries on upgrade
Type: files; Name: "{app}\attune_engine.dll"
Type: files; Name: "{app}\attune_watchguard.dll"
Type: files; Name: "{app}\attune_procmgr.dll"
Type: files; Name: "{app}\attune_music_metadata.dll"
Type: files; Name: "{app}\attune_music_metadata_changer.dll"
Type: files; Name: "{app}\attune_integrity.json"
Type: files; Name: "{app}\SHA256SUMS.txt"
Type: files; Name: "{app}\lib\attune_engine.dll"
Type: files; Name: "{app}\lib\attune_watchguard.dll"
Type: files; Name: "{app}\lib\attune_procmgr.dll"
Type: files; Name: "{app}\lib\attune_music_metadata.dll"
Type: files; Name: "{app}\lib\attune_music_metadata_changer.dll"
Type: files; Name: "{app}\lib\attune_integrity.json"
Type: files; Name: "{app}\lib\SHA256SUMS.txt"
; Remove legacy Flutter desktop runner binaries when upgrading to Native Rust Desktop
Type: files; Name: "{app}\flutter_windows.dll"
Type: files; Name: "{app}\{#LegacyFlutterExeName}"
Type: filesandordirs; Name: "{app}\data"

[Files]
; ============================================================================
; 1. Native Rust Desktop Executable & Signed Dynamic C-ABI Components (Release)
; ============================================================================
#if BuildMode == "Release"
#if FileExists("..\..\target\release\attune-desktop.exe")
Source: "..\..\target\release\attune-desktop.exe"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: {#BinarySignFlags}
#endif
Source: "..\..\target\release\attune_engine.dll"; DestDir: "{app}\lib"; DestName: "attune_engine.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\release\attune_engine.dll"; DestDir: "{app}"; DestName: "attune_engine.dll"; Flags: {#BinarySignFlags}
#if FileExists("..\..\target\release\attune_watchguard.dll")
Source: "..\..\target\release\attune_watchguard.dll"; DestDir: "{app}\lib"; DestName: "attune_watchguard.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\release\attune_watchguard.dll"; DestDir: "{app}"; DestName: "attune_watchguard.dll"; Flags: {#BinarySignFlags}
#endif
#if FileExists("..\..\target\release\attune_procmgr.dll")
Source: "..\..\target\release\attune_procmgr.dll"; DestDir: "{app}\lib"; DestName: "attune_procmgr.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\release\attune_procmgr.dll"; DestDir: "{app}"; DestName: "attune_procmgr.dll"; Flags: {#BinarySignFlags}
#endif
#if FileExists("..\..\target\release\attune_music_metadata.dll")
Source: "..\..\target\release\attune_music_metadata.dll"; DestDir: "{app}\lib"; DestName: "attune_music_metadata.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\release\attune_music_metadata.dll"; DestDir: "{app}"; DestName: "attune_music_metadata.dll"; Flags: {#BinarySignFlags}
#endif
#if FileExists("..\..\target\release\attune_music_metadata_changer.dll")
Source: "..\..\target\release\attune_music_metadata_changer.dll"; DestDir: "{app}\lib"; DestName: "attune_music_metadata_changer.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\release\attune_music_metadata_changer.dll"; DestDir: "{app}"; DestName: "attune_music_metadata_changer.dll"; Flags: {#BinarySignFlags}
#endif
#if FileExists("..\..\target\release\attune_integrity.json")
Source: "..\..\target\release\attune_integrity.json"; DestDir: "{app}\lib"; Flags: {#DllUpgradeFlags}
Source: "..\..\target\release\attune_integrity.json"; DestDir: "{app}"; Flags: {#DllUpgradeFlags}
#endif
#else
; ============================================================================
; 2. Native Rust Desktop Executable & Signed Dynamic C-ABI Components (Debug)
; ============================================================================
#if FileExists("..\..\target\debug\attune-desktop.exe")
Source: "..\..\target\debug\attune-desktop.exe"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: {#BinarySignFlags}
#endif
Source: "..\..\target\debug\attune_engine.dll"; DestDir: "{app}\lib"; DestName: "attune_engine.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\debug\attune_engine.dll"; DestDir: "{app}"; DestName: "attune_engine.dll"; Flags: {#BinarySignFlags}
#if FileExists("..\..\target\debug\attune_watchguard.dll")
Source: "..\..\target\debug\attune_watchguard.dll"; DestDir: "{app}\lib"; DestName: "attune_watchguard.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\debug\attune_watchguard.dll"; DestDir: "{app}"; DestName: "attune_watchguard.dll"; Flags: {#BinarySignFlags}
#endif
#if FileExists("..\..\target\debug\attune_procmgr.dll")
Source: "..\..\target\debug\attune_procmgr.dll"; DestDir: "{app}\lib"; DestName: "attune_procmgr.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\debug\attune_procmgr.dll"; DestDir: "{app}"; DestName: "attune_procmgr.dll"; Flags: {#BinarySignFlags}
#endif
#if FileExists("..\..\target\debug\attune_music_metadata.dll")
Source: "..\..\target\debug\attune_music_metadata.dll"; DestDir: "{app}\lib"; DestName: "attune_music_metadata.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\debug\attune_music_metadata.dll"; DestDir: "{app}"; DestName: "attune_music_metadata.dll"; Flags: {#BinarySignFlags}
#endif
#if FileExists("..\..\target\debug\attune_music_metadata_changer.dll")
Source: "..\..\target\debug\attune_music_metadata_changer.dll"; DestDir: "{app}\lib"; DestName: "attune_music_metadata_changer.dll"; Flags: {#BinarySignFlags}
Source: "..\..\target\debug\attune_music_metadata_changer.dll"; DestDir: "{app}"; DestName: "attune_music_metadata_changer.dll"; Flags: {#BinarySignFlags}
#endif
#if FileExists("..\..\target\debug\attune_integrity.json")
Source: "..\..\target\debug\attune_integrity.json"; DestDir: "{app}\lib"; Flags: {#DllUpgradeFlags}
Source: "..\..\target\debug\attune_integrity.json"; DestDir: "{app}"; Flags: {#DllUpgradeFlags}
#endif
#endif

#if FileExists("..\..\dist\windows\attune_integrity.json")
Source: "..\..\dist\windows\attune_integrity.json"; DestDir: "{app}\lib"; Flags: {#DllUpgradeFlags}
Source: "..\..\dist\windows\attune_integrity.json"; DestDir: "{app}"; Flags: {#DllUpgradeFlags}
#endif

#if FileExists("..\..\dist\windows\SHA256SUMS.txt")
Source: "..\..\dist\windows\SHA256SUMS.txt"; DestDir: "{app}"; Flags: {#DllUpgradeFlags}
#endif

#if FileExists("..\..\packaging\windows\app_icon.ico")
Source: "..\..\packaging\windows\app_icon.ico"; DestDir: "{app}"; Flags: {#DllUpgradeFlags}
#endif

#if FileExists("..\..\apps\attune_app\assets\build_info.json")
Source: "..\..\apps\attune_app\assets\build_info.json"; DestDir: "{app}"; Flags: {#DllUpgradeFlags}
#endif

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app_icon.ico"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app_icon.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
var
  FlutterInstallWarned: Boolean;

function DetectPreviousFlutterInstall(TargetDir: String; var DetectedLocations: String): Boolean;
var
  CleanDir: String;
  AppDataExampleDir: String;
  AppDataFlutterPrefs: String;
  LocalAppDataFlutterPrefs: String;
begin
  Result := False;
  DetectedLocations := '';
  CleanDir := RemoveBackslashUnlessRoot(TargetDir);

  if FileExists(CleanDir + '\flutter_windows.dll') then
  begin
    Result := True;
    DetectedLocations := DetectedLocations + '  • ' + CleanDir + '\flutter_windows.dll' + #13#10;
  end;

  if FileExists(CleanDir + '\{#LegacyFlutterExeName}') then
  begin
    Result := True;
    DetectedLocations := DetectedLocations + '  • ' + CleanDir + '\{#LegacyFlutterExeName}' + #13#10;
  end;

  if DirExists(CleanDir + '\data\flutter_assets') then
  begin
    Result := True;
    DetectedLocations := DetectedLocations + '  • ' + CleanDir + '\data\flutter_assets' + #13#10;
  end;

  AppDataExampleDir := ExpandConstant('{userappdata}\com.example\attune_app');
  if DirExists(AppDataExampleDir) then
  begin
    Result := True;
    DetectedLocations := DetectedLocations + '  • ' + AppDataExampleDir + #13#10;
  end;

  AppDataFlutterPrefs := ExpandConstant('{userappdata}\attune_app\shared_preferences.json');
  if FileExists(AppDataFlutterPrefs) then
  begin
    Result := True;
    DetectedLocations := DetectedLocations + '  • ' + AppDataFlutterPrefs + #13#10;
  end;

  LocalAppDataFlutterPrefs := ExpandConstant('{localappdata}\attune_app\shared_preferences.json');
  if FileExists(LocalAppDataFlutterPrefs) then
  begin
    Result := True;
    DetectedLocations := DetectedLocations + '  • ' + LocalAppDataFlutterPrefs + #13#10;
  end;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var
  DetectedLocations: String;
  MsgText: String;
begin
  Result := True;

  if (CurPageID = wpSelectDir) and (not FlutterInstallWarned) then
  begin
    if DetectPreviousFlutterInstall(WizardDirValue, DetectedLocations) then
    begin
      MsgText :=
        'WARNING: Previous Flutter-Based Attune Installation Detected!' + #13#10 + #13#10 +
        'The installer detected artifacts from an older Flutter version of Attune:' + #13#10 +
        DetectedLocations + #13#10 +
        'IMPORTANT COMPATIBILITY NOTICE:' + #13#10 +
        'Application data, preferences, and database caches from the legacy Flutter version ' +
        'are NOT compatible with the new Native Rust version of Attune (attune-desktop).' + #13#10 + #13#10 +
        'Continuing will replace the legacy Flutter runtime binaries with the signed Native Rust engine. ' +
        'Do you wish to proceed with the installation?';

      if SuppressibleMsgBox(MsgText, mbConfirmation, MB_YESNO or MB_DEFBUTTON1, IDYES) = IDNO then
      begin
        Result := False;
        Exit;
      end;

      FlutterInstallWarned := True;
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  AppDataExampleDir: String;
  AppDataFlutterDir: String;
  LocalAppDataFlutterDir: String;
begin
  if CurStep = ssInstall then
  begin
    if WizardIsTaskSelected('cleanflutterdata') then
    begin
      AppDataExampleDir := ExpandConstant('{userappdata}\com.example\attune_app');
      if DirExists(AppDataExampleDir) then
        DelTree(AppDataExampleDir, True, True, True);

      AppDataFlutterDir := ExpandConstant('{userappdata}\attune_app');
      if FileExists(AppDataFlutterDir + '\shared_preferences.json') then
        DelTree(AppDataFlutterDir, True, True, True);

      LocalAppDataFlutterDir := ExpandConstant('{localappdata}\attune_app');
      if FileExists(LocalAppDataFlutterDir + '\shared_preferences.json') then
        DelTree(LocalAppDataFlutterDir, True, True, True);
    end;
  end;
end;
