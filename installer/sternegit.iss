#ifndef AppVersion
  #error Pass the version with /DAppVersion=X.Y.Z
#endif
#ifndef NumericVersion
  #error Pass the version without its pre-release part with /DNumericVersion=X.Y.Z
#endif
#ifndef Staging
  #error Pass the folder holding sternegit.exe and connectors with /DStaging=...
#endif
#ifndef SetupIcon
  #error Pass the icon build.rs wrote with /DSetupIcon=...
#endif

#define AppName "SterneGit"
#define AppExe "sternegit.exe"
#define Repository "https://github.com/Sulexa/SterneGit"

[Setup]
AppId={{DA4AA123-3721-4E0A-811C-254241C68EAB}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppName}
AppPublisherURL={#Repository}
AppSupportURL={#Repository}/issues
AppUpdatesURL={#Repository}/releases
VersionInfoVersion={#NumericVersion}
PrivilegesRequired=lowest
DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
UsedUserAreasWarning=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17134
WizardStyle=modern
SetupIconFile={#SetupIcon}
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Compression=lzma2/ultra64
SolidCompression=yes
OutputBaseFilename={#AppName}-Setup-{#AppVersion}
ChangesEnvironment=yes
CloseApplications=force
RestartApplications=no
SetupMutex=SterneGitSetup

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "openin"; Description: "Add ""Open in SterneGit"" to folders in Explorer"; GroupDescription: "Explorer and terminal:"
Name: "path"; Description: "Add SterneGit to PATH, to start it from a terminal"; GroupDescription: "Explorer and terminal:"; Flags: unchecked

[InstallDelete]
Type: filesandordirs; Name: "{app}\connectors"

[Files]
Source: "{#Staging}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\Directory\shell\SterneGit"; ValueType: string; ValueName: ""; ValueData: "Open in SterneGit"; Flags: uninsdeletekey; Tasks: openin
Root: HKCU; Subkey: "Software\Classes\Directory\shell\SterneGit"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExe}"",0"; Tasks: openin
Root: HKCU; Subkey: "Software\Classes\Directory\shell\SterneGit\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%V"""; Tasks: openin
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\SterneGit"; ValueType: string; ValueName: ""; ValueData: "Open in SterneGit"; Flags: uninsdeletekey; Tasks: openin
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\SterneGit"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExe}"",0"; Tasks: openin
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\SterneGit\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%V"""; Tasks: openin
Root: HKCU; Subkey: "Software\Classes\Directory\shell\SterneGit"; Flags: deletekey; Tasks: not openin
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\SterneGit"; Flags: deletekey; Tasks: not openin

[Run]
Filename: "{app}\{#AppExe}"; Flags: nowait; Check: RelaunchAfterUpdate
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{localappdata}\SterneGit\updates"

[Code]
const
  EnvironmentKey = 'Environment';
  PathValue = 'Path';
  GitDownloadPage = 'https://git-scm.com/download/win';

function RelaunchAfterUpdate: Boolean;
begin
  Result := WizardSilent and (ExpandConstant('{param:relaunch|no}') = 'yes');
end;

function GitOnPath: Boolean;
begin
  Result := FileSearch('git.exe', GetEnv('PATH')) <> '';
end;

procedure OpenGitDownloadPage(Sender: TObject);
var
  ErrorCode: Integer;
begin
  ShellExecAsOriginalUser('open', GitDownloadPage, '', '', SW_SHOWNORMAL, ewNoWait, ErrorCode);
end;

procedure AddGitPage;
var
  Page: TOutputMsgWizardPage;
  Button: TNewButton;
begin
  Page := CreateOutputMsgPage(wpWelcome,
    'Git is needed',
    'SterneGit uses Git for diffs, staging, fetching and pushing.',
    'Git wasn''t found on this computer''s PATH. SterneGit still installs and shows history without it, ' +
    'but install Git 2.26 or newer to use everything else. You can continue now and install Git afterwards.');
  Button := TNewButton.Create(Page);
  Button.Parent := Page.Surface;
  Button.Caption := 'Open git-scm.com';
  Button.Left := 0;
  Button.Top := Page.MsgLabel.Top + Page.MsgLabel.Height + ScaleY(16);
  Button.Width := ScaleX(140);
  Button.Height := WizardForm.NextButton.Height;
  Button.OnClick := @OpenGitDownloadPage;
end;

procedure InitializeWizard;
begin
  if not GitOnPath then
    AddGitPage;
end;

function PathWithout(Path, Folder: string): string;
var
  Rest, Entry: string;
  Separator: Integer;
begin
  Result := '';
  Rest := Path;
  while Rest <> '' do
  begin
    Separator := Pos(';', Rest);
    if Separator = 0 then
    begin
      Entry := Rest;
      Rest := '';
    end
    else
    begin
      Entry := Copy(Rest, 1, Separator - 1);
      Delete(Rest, 1, Separator);
    end;
    if (Entry <> '') and (CompareText(RemoveBackslashUnlessRoot(Entry), RemoveBackslashUnlessRoot(Folder)) <> 0) then
    begin
      if Result <> '' then
        Result := Result + ';';
      Result := Result + Entry;
    end;
  end;
end;

procedure KeepOnPath(Wanted: Boolean);
var
  Path, Folder, Changed: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, PathValue, Path) then
    Path := '';
  Folder := ExpandConstant('{app}');
  Changed := PathWithout(Path, Folder);
  if Wanted then
  begin
    if Changed <> '' then
      Changed := Changed + ';';
    Changed := Changed + Folder;
  end;
  if Changed <> Path then
    RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, PathValue, Changed);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    KeepOnPath(WizardIsTaskSelected('path'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    KeepOnPath(False);
end;
