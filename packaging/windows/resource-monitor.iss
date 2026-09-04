#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\..\target\x86_64-pc-windows-msvc\release\resource_monitor.exe"
#endif

[Setup]
AppId={{4C3601C9-C50B-48BA-959D-D9CE2B721D22}
AppName=Resource Monitor
AppVersion={#AppVersion}
AppPublisher=Mobil0010
AppPublisherURL=https://github.com/Mobil0010/resource_monitor
DefaultDirName={autopf}\Resource Monitor
DefaultGroupName=Resource Monitor
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename=ResourceMonitor-{#AppVersion}-Windows-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
UninstallDisplayName=Resource Monitor

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "바탕 화면에 바로 가기 만들기"; GroupDescription: "추가 바로 가기:"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "ResourceMonitor.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Resource Monitor"; Filename: "{app}\ResourceMonitor.exe"
Name: "{autodesktop}\Resource Monitor"; Filename: "{app}\ResourceMonitor.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\ResourceMonitor.exe"; Description: "Resource Monitor 실행"; Flags: nowait postinstall skipifsilent
