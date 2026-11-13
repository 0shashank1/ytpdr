#define MyAppName "YTPDR"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Shashank"
#define MyAppURL "https://github.com/0shashank1/ytpdr"
#define MyAppExeName "ytpdr.exe"

[Setup]
AppId={{C8E64D92-230F-4720-B8A6-3D8D7E3035A1}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=ytpdr-Setup-v1.0.0
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "addtopath"; Description: "Add YTPDR to your User PATH (recommended so you can use 'ytpdr' in any terminal)"; GroupDescription: "Environment:"; Flags: checkedonce

[Files]
Source: "..\target\release\ytpdr.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "bundle\yt-dlp.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "bundle\ffmpeg.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; DestName: "README.txt"; Flags: ignoreversion

[Icons]
Name: "{group}\YTPDR Command Prompt"; Filename: "{cmd}"; Parameters: "/K ""title YTPDR && echo Welcome to YTPDR! Type 'ytpdr --help' to get started. && echo."""
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"

[Code]
const
  EnvironmentKey = 'Environment';

procedure AddToPath();
var
  PathStr: string;
  AppDir: string;
begin
  AppDir := ExpandConstant('{app}');
  if RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', PathStr) then
  begin
    if Pos(AppDir, PathStr) = 0 then
    begin
      if (Length(PathStr) > 0) and (PathStr[Length(PathStr)] <> ';') then
        PathStr := PathStr + ';';
      PathStr := PathStr + AppDir;
      RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', PathStr);
    end;
  end
  else
  begin
    RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', AppDir);
  end;
end;

procedure RemoveFromPath();
var
  PathStr, AppDir: string;
  P, L: Integer;
begin
  AppDir := ExpandConstant('{app}');
  if RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', PathStr) then
  begin
    P := Pos(AppDir, PathStr);
    if P > 0 then
    begin
      L := Length(AppDir);
      if (P + L <= Length(PathStr)) and (PathStr[P + L] = ';') then
        L := L + 1
      else if (P > 1) and (PathStr[P - 1] = ';') then
      begin
        Dec(P);
        L := L + 1;
      end;
      Delete(PathStr, P, L);
      RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', PathStr);
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
  begin
    AddToPath();
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    RemoveFromPath();
  end;
end;
