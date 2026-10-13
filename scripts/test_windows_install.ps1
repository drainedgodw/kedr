param([Parameter(Mandatory=$true)][string]$Setup)
$ErrorActionPreference = 'Stop'
$target = Join-Path $env:TEMP ('Kedr Setup Test ' + [Guid]::NewGuid().ToString('N'))
$setupFull = (Resolve-Path $Setup).Path
function Install-Preview {
  $p = Start-Process -FilePath $setupFull -ArgumentList @('/S', "/D=$target") -Wait -PassThru
  if ($p.ExitCode -ne 0) { throw "Setup failed: $($p.ExitCode)" }
}
Install-Preview
if (!(Test-Path "$target\kedr-studio.exe") -or !(Test-Path "$target\codecedar.ico")) { throw 'Missing binary/icon' }
$menu = Join-Path ([Environment]::GetFolderPath('StartMenu')) 'Programs\Kedr\CodeCedar.lnk'
if (!(Test-Path $menu)) { throw 'Missing Start menu shortcut' }
$link = (New-Object -ComObject WScript.Shell).CreateShortcut($menu)
if ($link.TargetPath -ne "$target\kedr-studio.exe") { throw 'Wrong shortcut target' }
if (!$link.IconLocation.StartsWith("$target\codecedar.ico")) { throw 'Wrong shortcut icon' }
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CodeCedar'
if (!(Test-Path $key)) { throw 'Missing uninstall registration' }
& "$target\kedr-studio.exe" --help
if ($LASTEXITCODE -ne 0) { throw 'Installed binary health check failed' }
Set-Content "$target\my-project.txt" 'must survive uninstall'
Install-Preview
$p = Start-Process -FilePath "$target\Uninstall.exe" -ArgumentList '/S' -PassThru
$p.WaitForExit()
for ($i=0; $i -lt 60 -and (Test-Path "$target\kedr-studio.exe"); $i++) { Start-Sleep -Milliseconds 500 }
if (Test-Path "$target\kedr-studio.exe") { throw 'Binary survived uninstall' }
if (!(Test-Path "$target\my-project.txt")) { throw 'Uninstaller deleted unrelated user file' }
if (Test-Path $menu) { throw 'Menu shortcut survived uninstall' }
if (Test-Path $key) { throw 'Uninstall registration survived uninstall' }
Remove-Item "$target\my-project.txt"
Remove-Item $target -Force -ErrorAction SilentlyContinue
Write-Output 'Windows silent install/upgrade/menu/icon/health/uninstall tests passed.'
