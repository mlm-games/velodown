$ErrorActionPreference = 'Stop'

$packageName = 'velodown'
$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url = '{{DOWNLOAD_URL}}/{{VERSION}}/velodown_{{VERSION}}_x64-setup.exe'

$packageArgs = @{
  packageName   = $packageName
  fileType      = 'EXE'
  url           = $url
  softwareName  = 'VeloDown*'
  checksum      = '{{SHA256_HASH}}'
  checksumType  = 'sha256'
  silentArgs    = "/S"
  validExitCodes= @(0)
}

# Ensure .NET dependencies
if (!(Test-Path "HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full")) {
  Write-Warning ".NET Framework 4.5 or higher is required. Installing..."
  choco install dotnet4.5 -y
}

Install-ChocolateyPackage @packageArgs

# Add to PATH if not already there
$installPath = Join-Path $env:LOCALAPPDATA "Programs\velodown"
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -notlike "*$installPath*") {
  [Environment]::SetEnvironmentVariable("Path", "$currentPath;$installPath", "User")
  $env:Path = "$env:Path;$installPath"
}

Write-Host "VeloDown has been installed successfully!" -ForegroundColor Green
Write-Host "You can run it from the Start Menu or by typing 'velodown' in the command line." -ForegroundColor Cyan
