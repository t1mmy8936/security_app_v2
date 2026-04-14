; Watchtower — Inno Setup Installer Script
; Requires: Inno Setup 6+ (https://jrsoftware.org/isinfo.php)

#define MyAppName "Watchtower"
#define MyAppVersion "2.0"
#define MyAppPublisher "Watchtower Security"
#define MyAppURL "http://localhost:66"

[Setup]
AppId={{B7F4E2A1-9C3D-4E5F-8A6B-1D2E3F4A5B6C}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={userdesktop}\Watchtower
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=Watchtower-Setup
SetupIconFile=..\public\favicon.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
UninstallDisplayName={#MyAppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Messages]
WelcomeLabel1=Welcome to Watchtower
WelcomeLabel2=This will install Watchtower on your computer.%n%nWatchtower is a security testing suite built with Rust.%n%nDocker Desktop will be installed automatically if not detected.%n%nRequirements:%n  - PowerShell 5.1+%n  - Internet connection (if Docker Desktop is not installed)%n%n"I find your lack of security disturbing."

[Files]
; Core project files
Source: "..\Cargo.toml"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\Cargo.lock"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\rust-toolchain.toml"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\docker-compose.yml"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\Dockerfile"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\execute-order-66.ps1"; DestDir: "{app}"; Flags: ignoreversion

; Source code
Source: "..\src\*"; DestDir: "{app}\src"; Flags: ignoreversion recursesubdirs createallsubdirs

; Styles
Source: "..\style\*"; DestDir: "{app}\style"; Flags: ignoreversion recursesubdirs createallsubdirs

; Public assets
Source: "..\public\*"; DestDir: "{app}\public"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Dirs]
Name: "{app}\data"
Name: "{app}\reports"

[Icons]
Name: "{group}\Watchtower"; Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{app}\execute-order-66.ps1"""; WorkingDir: "{app}"; Comment: "Execute Order 66"
Name: "{group}\Watchtower Dashboard"; Filename: "http://localhost:66"; Comment: "Open Watchtower Dashboard"
Name: "{group}\Uninstall Watchtower"; Filename: "{uninstallexe}"
Name: "{userdesktop}\Watchtower"; Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{app}\execute-order-66.ps1"""; WorkingDir: "{app}"; Comment: "Execute Order 66"

[Run]
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -Command ""Set-Content -Path '{app}\.env' -Value 'LOCAL_PROJECTS_PATH=C:\Users\%USERNAME%\source\repos' -Encoding UTF8"""; Flags: runhidden; StatusMsg: "Creating configuration..."
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{app}\execute-order-66.ps1"""; WorkingDir: "{app}"; Description: "Launch Watchtower now"; Flags: postinstall nowait skipifsilent shellexec

[UninstallRun]
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -Command ""Set-Location '{app}'; docker compose down -v 2>$null"""; Flags: runhidden; RunOnceId: "StopContainers"

[Code]
var
  NeedsDocker: Boolean;
  DownloadPage: TDownloadWizardPage;

function IsDockerInstalled(): Boolean;
var
  ResultCode: Integer;
begin
  Result := Exec('cmd.exe', '/c docker --version', '', SW_HIDE, ewWaitUntilTerminated, ResultCode)
            and (ResultCode = 0);
end;

function InitializeSetup(): Boolean;
begin
  Result := True;
  NeedsDocker := not IsDockerInstalled();
end;

procedure InitializeWizard();
begin
  DownloadPage := CreateDownloadPage(
    'Installing Docker Desktop',
    'Watchtower requires Docker Desktop. Downloading now...',
    nil
  );
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var
  DockerInstaller: String;
  ResultCode: Integer;
begin
  Result := True;

  if (CurPageID = wpReady) and NeedsDocker then
  begin
    // Ask user before downloading ~500MB
    if MsgBox('Docker Desktop was not detected on this machine.' + #13#10 + #13#10 +
              'Watchtower needs Docker Desktop to run.' + #13#10 +
              'Download and install it now? (~500 MB download)' + #13#10 + #13#10 +
              'If you choose No, you can install Docker Desktop manually later' + #13#10 +
              'from https://docker.com/products/docker-desktop',
              mbConfirmation, MB_YESNO) = IDNO then
    begin
      // User declined — skip Docker install, continue with Watchtower
      Exit;
    end;

    DockerInstaller := ExpandConstant('{tmp}\DockerDesktopInstaller.exe');

    // Download Docker Desktop from official URL
    DownloadPage.Clear;
    DownloadPage.Add(
      'https://desktop.docker.com/win/main/amd64/Docker%20Desktop%20Installer.exe',
      'DockerDesktopInstaller.exe',
      ''
    );
    DownloadPage.Show;
    try
      try
        DownloadPage.Download;
      except
        MsgBox('Docker Desktop download failed:' + #13#10 + #13#10 +
               GetExceptionMessage + #13#10 + #13#10 +
               'You can install Docker Desktop manually after setup.' + #13#10 +
               'https://docker.com/products/docker-desktop',
               mbError, MB_OK);
        Exit;
      end;
    finally
      DownloadPage.Hide;
    end;

    // Run Docker Desktop installer (silent, accept license)
    WizardForm.StatusLabel.Caption := 'Installing Docker Desktop (this may take a few minutes)...';
    if Exec(DockerInstaller, 'install --quiet --accept-license', '', SW_SHOW, ewWaitUntilTerminated, ResultCode) then
    begin
      if ResultCode = 0 then
      begin
        NeedsDocker := False;
        MsgBox('Docker Desktop installed successfully!' + #13#10 + #13#10 +
               'Note: You may need to restart your computer before Docker is fully available.',
               mbInformation, MB_OK);
      end
      else
      begin
        MsgBox('Docker Desktop installer exited with code ' + IntToStr(ResultCode) + '.' + #13#10 + #13#10 +
               'You may need to install it manually from:' + #13#10 +
               'https://docker.com/products/docker-desktop',
               mbError, MB_OK);
      end;
    end
    else
    begin
      MsgBox('Failed to launch Docker Desktop installer.' + #13#10 + #13#10 +
             'You may need to install it manually from:' + #13#10 +
             'https://docker.com/products/docker-desktop',
             mbError, MB_OK);
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ProfilePath, ProfileContent, FuncBlock: String;
begin
  if CurStep = ssPostInstall then
  begin
    // Add 'execute order 66' function to PowerShell profile
    ProfilePath := ExpandConstant('{userdocs}\WindowsPowerShell\Microsoft.PowerShell_profile.ps1');
    
    FuncBlock := '# Watchtower — "execute order 66"' + #13#10 +
                 'function execute { ' + #13#10 +
                 '    param([Parameter(ValueFromRemainingArguments)]$args)' + #13#10 +
                 '    if ("$args" -match "^order\s+66$") {' + #13#10 +
                 '        & "' + ExpandConstant('{app}') + '\execute-order-66.ps1"' + #13#10 +
                 '    } else {' + #13#10 +
                 '        Write-Host "Unknown order." -ForegroundColor Red' + #13#10 +
                 '    }' + #13#10 +
                 '}' + #13#10;
    
    if FileExists(ProfilePath) then
    begin
      if not LoadStringFromFile(ProfilePath, ProfileContent) then
        ProfileContent := '';
      if Pos('function execute', ProfileContent) = 0 then
        SaveStringToFile(ProfilePath, #13#10 + FuncBlock, True);
    end
    else
    begin
      ForceDirectories(ExtractFilePath(ProfilePath));
      SaveStringToFile(ProfilePath, FuncBlock, False);
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ProfilePath, ProfileContent: String;
  StartPos: Integer;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Remove the execute function from PowerShell profile
    ProfilePath := ExpandConstant('{userdocs}\WindowsPowerShell\Microsoft.PowerShell_profile.ps1');
    if FileExists(ProfilePath) then
    begin
      if LoadStringFromFile(ProfilePath, ProfileContent) then
      begin
        StartPos := Pos('# Watchtower', ProfileContent);
        if StartPos > 0 then
        begin
          // Remove the watchtower block (rough cleanup)
          Delete(ProfileContent, StartPos, Pos('}' + #13#10, Copy(ProfileContent, StartPos, Length(ProfileContent))) + 2);
          SaveStringToFile(ProfilePath, ProfileContent, False);
        end;
      end;
    end;
  end;
end;
