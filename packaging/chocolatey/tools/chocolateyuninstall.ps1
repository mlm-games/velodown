$ErrorActionPreference = 'Stop'

$packageName = 'velodown'
$softwareName = 'VeloDown*'

$uninstalled = $false
[array]$key = Get-UninstallRegistryKey -SoftwareName $softwareName

if ($key.Count -eq 1) {
  $key | % { 
    $file = "$($_.UninstallString)"
    
    if ($file) {
      # Remove quotes if present
      $file = $file.Trim('"')
      
      # Run uninstaller
      $packageArgs = @{
        packageName    = $packageName
        fileType       = 'EXE'
        silentArgs     = '/S'
        validExitCodes = @(0)
        file           = $file
      }
      
      Uninstall-ChocolateyPackage @packageArgs
      
      # Remove from PATH
      $installPath = Join-Path $env:LOCALAPPDATA "Programs\velodown"
      $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
      if ($currentPath -like "*$installPath*") {
        $newPath = ($currentPath.Split(';') | Where-Object { $_ -ne $installPath }) -join ';'
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
      }
      
      # Clean up any remaining files
      if (Test-Path $installPath) {
        Remove-Item -Path $installPath -Recurse -Force -ErrorAction SilentlyContinue
      }
      
      $uninstalled = $true
    }
  }
} elseif ($key.Count -eq 0) {
  Write-Warning "$packageName has already been uninstalled by other means."
} elseif ($key.Count -gt 1) {
  Write-Warning "$($key.Count) matches found!"
  Write-Warning "To prevent accidental data loss, no programs will be uninstalled."
  Write-Warning "Please alert package maintainer the following keys were matched:"
  $key | % {Write-Warning "- $($_.DisplayName)"}
}

if ($uninstalled) {
  Write-Host "$packageName has been uninstalled successfully." -ForegroundColor Green
}
