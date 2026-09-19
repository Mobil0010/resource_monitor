#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\..\target\x86_64-pc-windows-msvc\release\resource_monitor.exe"
#endif
#ifndef PresentMonDir
  #define PresentMonDir "..\..\vendor\presentmon"
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
UninstallDisplayIcon={app}\ResourceMonitor.exe

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[CustomMessages]
korean.PawnIOSetupNotice=설치 후 Resource Monitor를 처음 실행하면 하드웨어 온도 센서를 위해 PawnIO 설치 창이 나타날 수 있습니다. 관리자 권한 요청이 표시되면 허용하고 PawnIO 설치를 완료해 주세요. 설치하지 않아도 앱은 실행되지만 일부 온도 정보가 표시되지 않을 수 있습니다.
english.PawnIOSetupNotice=When Resource Monitor starts for the first time after installation, a PawnIO setup window may appear for hardware temperature sensors. Approve the administrator prompt and complete the PawnIO installation. The app can still run without it, but some temperature readings may be unavailable.
japanese.PawnIOSetupNotice=インストール後に Resource Monitor を初めて起動すると、ハードウェア温度センサーのために PawnIO のセットアップ画面が表示される場合があります。管理者権限の確認が表示されたら許可して、PawnIO のインストールを完了してください。インストールしなくてもアプリは起動できますが、一部の温度情報が表示されない場合があります。

[Messages]
WelcomeLabel2={cm:PawnIOSetupNotice}

[Tasks]
Name: "desktopicon"; Description: "바탕 화면에 바로 가기 만들기"; GroupDescription: "추가 바로 가기:"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "ResourceMonitor.exe"; Flags: ignoreversion
Source: "..\..\vendor\sensor-support\*"; DestDir: "{app}\sensor-support"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#PresentMonDir}\PresentMon.exe"; DestDir: "{app}\tools"; DestName: "PresentMon.exe"; Flags: ignoreversion
Source: "{#PresentMonDir}\LICENSE.txt"; DestDir: "{app}\tools"; DestName: "PresentMon-LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Resource Monitor"; Filename: "{app}\ResourceMonitor.exe"; IconFilename: "{app}\ResourceMonitor.exe"
Name: "{autodesktop}\Resource Monitor"; Filename: "{app}\ResourceMonitor.exe"; IconFilename: "{app}\ResourceMonitor.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\ResourceMonitor.exe"; Description: "Resource Monitor 실행"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
