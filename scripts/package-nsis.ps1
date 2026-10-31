[CmdletBinding()]
param([switch]$SkipBuild, [switch]$RequireSigned, [switch]$TestSetup, [string]$BinaryPath)
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
} finally { Pop-Location }
# Keep a failed `git describe` from failing the step after a successful package.
$global:LASTEXITCODE = 0
