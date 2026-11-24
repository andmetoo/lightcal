[CmdletBinding()]
param([switch]$SkipBuild, [switch]$RequireSigned, [switch]$TestSetup, [string]$BinaryPath, [string]$Version)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
function Run-Native([string]$Executable, [string[]]$Arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $Executable
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($info)
    try { $process.WaitForExit(); if ($process.ExitCode -ne 0) { throw "$Executable failed with exit code $($process.ExitCode)" } }
    finally { $process.Dispose() }
}
$compiler = $env:LIGHTCAL_NSIS_COMPILER
if ([string]::IsNullOrWhiteSpace($compiler)) {
    $command = Get-Command makensis.exe -ErrorAction SilentlyContinue
    if ($command) { $compiler = $command.Source }
}
if ([string]::IsNullOrWhiteSpace($compiler)) {
    $installed = Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe'
    if (Test-Path -LiteralPath $installed) { $compiler = $installed }
}
if ([string]::IsNullOrWhiteSpace($compiler)) { $compiler = & (Join-Path $PSScriptRoot 'prepare-nsis.ps1') }
if (-not (Test-Path -LiteralPath $compiler)) { throw 'NSIS compiler was not found' }
if (-not [string]::IsNullOrWhiteSpace($Version)) { $env:LIGHTCAL_RELEASE_VERSION = $Version }
. (Join-Path $PSScriptRoot 'Resolve-ReleaseVersion.ps1')
$version = Resolve-LightcalReleaseVersion $projectRoot
$outputDir = Join-Path $projectRoot 'target\nsis'
$stage = Join-Path $outputDir ('stage-' + [Guid]::NewGuid().ToString('N'))
Push-Location $projectRoot
try {
    if (-not $SkipBuild) { Run-Native 'cargo' @('build', '--release', '--workspace', '--locked') }
    if ([string]::IsNullOrWhiteSpace($BinaryPath)) { $BinaryPath = Join-Path $projectRoot 'target\release\lightcal.exe' }
    $BinaryPath = [IO.Path]::GetFullPath($BinaryPath)
    if (-not (Test-Path -LiteralPath $BinaryPath)) { throw 'Build the release executable before packaging' }
    $actualVersion = [Diagnostics.FileVersionInfo]::GetVersionInfo($BinaryPath).ProductVersion
    if ($actualVersion -ne $version) { throw "Payload ProductVersion $actualVersion differs from release version $version" }
    if ($RequireSigned -and (Get-AuthenticodeSignature -LiteralPath $BinaryPath).Status -ne 'Valid') { throw 'Signed packaging requires a trusted signed application payload' }
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item -LiteralPath $BinaryPath -Destination (Join-Path $stage 'lightcal.exe')
    foreach ($name in @('LICENSE','THIRD_PARTY_NOTICES.md','README.md')) { Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination $stage }
    # The license page shows Lightcal's own notice and the GPL terms. The
    # "How to Apply These Terms" appendix after the terms is a template with
    # placeholders for other projects, so it is left off the page; the
    # installed LICENSE stays the verbatim GPL text.
    $gpl = [IO.File]::ReadAllText((Join-Path $projectRoot 'LICENSE'))
    $termsEnd = 'END OF TERMS AND CONDITIONS'
    $cut = $gpl.IndexOf($termsEnd)
    if ($cut -lt 0) { throw "LICENSE does not contain '$termsEnd'" }
    $notice = @(
        'Lightcal'
        'Copyright (C) 2026 Lightcal contributors'
        ''
        'Lightcal is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 3 only (GPL-3.0-only).'
        ''
        'Lightcal is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the license terms below for details.'
        ''
        'Third-party notices are installed with Lightcal in THIRD_PARTY_NOTICES.md.'
        ''
        ''
    ) -join "`r`n"
    $page = $notice + $gpl.Substring(0, $cut + $termsEnd.Length) + "`r`n"
    if ($page -match '<(one line|year>|name of author|program>)') { throw 'License page still contains GPL template placeholders' }
    [IO.File]::WriteAllText((Join-Path $stage 'LICENSE-PAGE.txt'), $page, [Text.UTF8Encoding]::new($true))
    Copy-Item -LiteralPath (Join-Path $projectRoot 'crates\lightcal\assets\lightcal.ico') -Destination $stage
    $nsisRoot = Split-Path (Split-Path $compiler -Parent) -Parent
    $license = Join-Path $nsisRoot 'COPYING'
    if (-not (Test-Path -LiteralPath $license)) { $license = Join-Path (Split-Path $compiler -Parent) 'COPYING' }
    if (-not (Test-Path -LiteralPath $license)) { throw 'NSIS license file was not found beside the compiler' }
    Copy-Item -LiteralPath $license -Destination (Join-Path $stage 'NSIS-LICENSE.txt')
    $suffix = if ($TestSetup) { '-isolated-test' } else { '' }
    $output = Join-Path $outputDir "lightcal-$version-windows-x64-setup$suffix.exe"
    $arguments = @('/V3',"/DVERSION=$version","/DSTAGE=$stage","/DOUTPUT=$output")
    if ($TestSetup) { $arguments += '/DTEST_SETUP' }
    $arguments += (Join-Path $projectRoot 'packaging\nsis\lightcal.nsi')
    Run-Native $compiler $arguments
    if (-not (Test-Path -LiteralPath $output)) { throw 'NSIS did not produce an installer' }
    $checksum = (Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$output.sha256", "$checksum  $([IO.Path]::GetFileName($output))`n",[Text.UTF8Encoding]::new($false))
    Write-Output "Created NSIS installer: $output"
    Write-Output 'No installer was run, and no package was installed or published.'
} finally {
    Pop-Location
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue }
}
# Keep a failed `git describe` from failing the step after a successful package.
$global:LASTEXITCODE = 0
