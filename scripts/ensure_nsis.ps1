# Reuse a runner installation; otherwise retry transient Chocolatey feed failures.
$ErrorActionPreference = 'Stop'
# Native nonzero codes are inspected below so transient failures can be retried.
$PSNativeCommandUseErrorActionPreference = $false
$candidates = @(
    (Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe'),
    (Join-Path $env:ProgramFiles 'NSIS\makensis.exe')
)
function Find-Nsis {
    foreach ($path in $candidates) { if (Test-Path -LiteralPath $path -PathType Leaf) { return $path } }
    $cmd = Get-Command makensis.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return $null
}
$found = Find-Nsis
if ($found) { Write-Output $found; exit 0 }
for ($attempt = 1; $attempt -le 3; $attempt++) {
    choco install nsis -y --no-progress 2>&1 | Out-Host
    $code = $LASTEXITCODE
    $found = Find-Nsis
    if (($code -eq 0 -or $code -eq 3010) -and $found) { Write-Output $found; exit 0 }
    if ($attempt -lt 3) { Start-Sleep -Seconds (5 * $attempt) }
}
throw 'NSIS is unavailable after three attempts. Installer build stopped; no successful package is claimed.'
