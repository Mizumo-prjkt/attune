; Script generated for Attune Music Player
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
#define MyAppExeName "attune_app.exe"

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

[Files]
; Main Flutter Windows Runner Bundle
Source: "..\..\apps\attune_app\build\windows\x64\runner\{#BuildMode}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; Attune Native Rust DSP Engine DLL bundled relative to application in lib\ and app root
#if BuildMode == "Release"
Source: "..\..\target\release\attune_engine.dll"; DestDir: "{app}\lib"; Flags: ignoreversion; DestName: "attune_engine.dll"
Source: "..\..\target\release\attune_engine.dll"; DestDir: "{app}"; Flags: ignoreversion; DestName: "attune_engine.dll"
#if FileExists("..\..\target\release\attune_watchguard.dll")
Source: "..\..\target\release\attune_watchguard.dll"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\release\attune_watchguard.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#if FileExists("..\..\target\release\attune_procmgr.dll")
Source: "..\..\target\release\attune_procmgr.dll"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\release\attune_procmgr.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#if FileExists("..\..\target\release\attune_music_metadata.dll")
Source: "..\..\target\release\attune_music_metadata.dll"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\release\attune_music_metadata.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#if FileExists("..\..\target\release\attune_music_metadata_changer.dll")
Source: "..\..\target\release\attune_music_metadata_changer.dll"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\release\attune_music_metadata_changer.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#if FileExists("..\..\target\release\attune_integrity.json")
Source: "..\..\target\release\attune_integrity.json"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\release\attune_integrity.json"; DestDir: "{app}"; Flags: ignoreversion
#endif
#else
Source: "..\..\target\debug\attune_engine.dll"; DestDir: "{app}\lib"; Flags: ignoreversion; DestName: "attune_engine.dll"
Source: "..\..\target\debug\attune_engine.dll"; DestDir: "{app}"; Flags: ignoreversion; DestName: "attune_engine.dll"
#if FileExists("..\..\target\debug\attune_watchguard.dll")
Source: "..\..\target\debug\attune_watchguard.dll"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\debug\attune_watchguard.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#if FileExists("..\..\target\debug\attune_procmgr.dll")
Source: "..\..\target\debug\attune_procmgr.dll"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\debug\attune_procmgr.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#if FileExists("..\..\target\debug\attune_music_metadata.dll")
Source: "..\..\target\debug\attune_music_metadata.dll"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\debug\attune_music_metadata.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#if FileExists("..\..\target\debug\attune_music_metadata_changer.dll")
Source: "..\..\target\debug\attune_music_metadata_changer.dll"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\debug\attune_music_metadata_changer.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#if FileExists("..\..\target\debug\attune_integrity.json")
Source: "..\..\target\debug\attune_integrity.json"; DestDir: "{app}\lib"; Flags: ignoreversion
Source: "..\..\target\debug\attune_integrity.json"; DestDir: "{app}"; Flags: ignoreversion
#endif
#endif

#if FileExists("..\..\packaging\windows\app_icon.ico")
Source: "..\..\packaging\windows\app_icon.ico"; DestDir: "{app}"; Flags: ignoreversion
#endif

#if FileExists("..\..\apps\attune_app\assets\build_info.json")
Source: "..\..\apps\attune_app\assets\build_info.json"; DestDir: "{app}\data\flutter_assets\assets"; Flags: ignoreversion
#endif

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
