[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$installer = ''
$testRoot = ''
Push-Location $projectRoot
try {
    & (Join-Path $PSScriptRoot 'package-nsis.ps1') -SkipBuild -TestSetup
    $manifest = [IO.File]::ReadAllText((Join-Path $projectRoot 'Cargo.toml'))
    $version = [regex]::Match($manifest,'(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"').Groups[1].Value
    $installer = Join-Path $projectRoot "target\nsis\lightcal-$version-windows-x64-setup-isolated-test.exe"
    $testRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot ('target\installer-tests\' + [Guid]::NewGuid().ToString('N'))))
    $expectedRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'target\installer-tests')) + [IO.Path]::DirectorySeparatorChar
    if (-not $testRoot.StartsWith($expectedRoot,[StringComparison]::OrdinalIgnoreCase)) { throw 'Fixture path escaped the workspace' }
    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    $sentinel = Join-Path $testRoot 'user-file.txt'
    [IO.File]::WriteAllText($sentinel,'preserve user-owned files')
    # TEST_SETUP compiles out registry, shortcuts, app launch, and real-profile detection.
    $process = Start-Process -FilePath $installer -ArgumentList @('/S',"/D=$testRoot") -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Isolated install failed: $($process.ExitCode)" }
    $exe = Join-Path $testRoot 'lightcal.exe'
    if (-not (Test-Path -LiteralPath $exe)) { throw 'Installer did not extract the application' }
    $expectedHash = (Get-FileHash -LiteralPath 'target\release\lightcal.exe' -Algorithm SHA256).Hash
    if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash -ne $expectedHash) { throw 'Installed payload differs from release' }
    $uninstaller = Join-Path $testRoot 'uninstall.exe'
    $process = Start-Process -FilePath $uninstaller -ArgumentList '/S' -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Isolated uninstall failed: $($process.ExitCode)" }
    if (Test-Path -LiteralPath $exe) { throw 'Uninstaller left the application executable' }
    if ([IO.File]::ReadAllText($sentinel) -ne 'preserve user-owned files') { throw 'Uninstaller modified an unrelated file' }
    Write-Output 'NSIS isolated installation, payload hash, uninstallation, and user-file preservation passed.'
    Write-Output 'No real registry, shortcuts, startup, credentials, or application profile were touched.'
} finally {
    Pop-Location
    if ($testRoot -and (Test-Path -LiteralPath $testRoot)) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    if ($installer -and (Test-Path -LiteralPath $installer)) {
        Remove-Item -LiteralPath $installer, "$installer.sha256" -Force -ErrorAction SilentlyContinue
    }
}
