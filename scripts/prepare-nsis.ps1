[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$tools = Join-Path $projectRoot 'target\tools'
$compiler = Join-Path $tools 'nsis-3.13\Bin\makensis.exe'
if (Test-Path -LiteralPath $compiler) { Write-Output $compiler; return }
New-Item -ItemType Directory -Path $tools -Force | Out-Null
$archive = Join-Path $tools 'nsis-3.13-verified.zip'
# Portable official build; no system installation or registry writes.
& curl.exe -fL --retry 2 --max-time 120 --silent --show-error 'https://deac-ams.dl.sourceforge.net/project/nsis/NSIS%203/3.13/nsis-3.13.zip' -o $archive
if ($LASTEXITCODE -ne 0) { throw 'Could not download NSIS; set LIGHTCAL_NSIS_COMPILER to an installed makensis.exe' }
$expected = 'BA63DFFC4410EE89193E1CB5A41989991BD77C61068DA17E3156D136B7B0B3D8'
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expected) { throw 'Official NSIS archive checksum mismatch' }
Expand-Archive -LiteralPath $archive -DestinationPath $tools -Force
if (-not (Test-Path -LiteralPath $compiler)) { throw 'Verified NSIS archive did not contain the expected compiler' }
Write-Output $compiler
