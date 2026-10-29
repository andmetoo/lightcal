[CmdletBinding()]
param([switch]$Preview, [switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$configFiles = @((Join-Path $projectRoot '.env.local'), (Join-Path $projectRoot '.env'))

function Read-Setting([string]$Name) {
    $value = [Environment]::GetEnvironmentVariable($Name)
    if ($null -ne $value) { return $value }
    foreach ($configPath in $configFiles) {
      if (Test-Path -LiteralPath $configPath) {
        foreach ($line in [IO.File]::ReadAllLines($configPath)) {
            if ($line.TrimStart().StartsWith('#')) { continue }
            $parts = $line.Split('=', 2)
            if ($parts.Length -eq 2 -and $parts[0].Trim() -eq $Name) {
                return $parts[1].Trim().Trim('"').Trim("'")
            }
        }
      }
    }
    return ''
}

function Run-Native([string]$Executable, [string[]]$Arguments) {
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Executable
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    foreach ($argument in $Arguments) { $startInfo.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($startInfo)
    try {
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "Native tool failed with exit code $($process.ExitCode)" }
    } finally { $process.Dispose() }
}

if ($Preview) {
    $identityName = 'Lightcal.Preview'
    $publisher = 'CN=LightcalPreviewOnly'
    $publisherDisplayName = 'Lightcal preview - not a Store submission'
    $displayName = 'Lightcal Preview'
    $channel = 'preview'
} else {
    $identityName = Read-Setting 'LIGHTCAL_MSIX_IDENTITY_NAME'
    $publisher = Read-Setting 'LIGHTCAL_MSIX_PUBLISHER'
    $publisherDisplayName = Read-Setting 'LIGHTCAL_MSIX_PUBLISHER_DISPLAY_NAME'
    $displayName = Read-Setting 'LIGHTCAL_MSIX_DISPLAY_NAME'
    if ([string]::IsNullOrWhiteSpace($displayName)) { $displayName = 'Lightcal' }
    foreach ($pair in @(@('LIGHTCAL_MSIX_IDENTITY_NAME', $identityName), @('LIGHTCAL_MSIX_PUBLISHER', $publisher), @('LIGHTCAL_MSIX_PUBLISHER_DISPLAY_NAME', $publisherDisplayName))) {
        if ([string]::IsNullOrWhiteSpace($pair[1])) { throw "Set $($pair[0]) from Partner Center Product identity; use -Preview for an unsigned review-only package" }
    }
    if ($identityName -eq 'CONFIGURE' -or $publisher -eq 'CN=CONFIGURE' -or $identityName -eq 'Lightcal.Preview') { throw 'Placeholder identities cannot be used for a Store package' }
    $channel = 'store'
}
if ($identityName -notmatch '^[A-Za-z0-9][A-Za-z0-9.-]{2,49}$' -or $identityName.EndsWith('.')) { throw 'Invalid MSIX identity name' }
if (-not $publisher.StartsWith('CN=')) { throw 'Publisher must match the CN= value from Partner Center' }

$manifestText = [IO.File]::ReadAllText((Join-Path $projectRoot 'Cargo.toml'))
$versionMatch = [regex]::Match($manifestText, '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"\s*$')
if (-not $versionMatch.Success) { throw 'Expected a stable workspace version in Cargo.toml' }
$versionParts = 1..3 | ForEach-Object { [int]$versionMatch.Groups[$_].Value }
if ($versionParts | Where-Object { $_ -gt 65535 }) { throw 'MSIX version components must be <= 65535' }
$version = ($versionParts -join '.') + '.0'

$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$makeAppx = Get-ChildItem -LiteralPath $sdkRoot -Directory | Sort-Object Name -Descending |
    ForEach-Object { Join-Path $_.FullName 'x64\makeappx.exe' } |
    Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $makeAppx) { throw 'Install the Windows SDK x64 MakeAppx tool' }

Push-Location $projectRoot
try {
    if (-not $SkipBuild) { Run-Native 'cargo' @('build', '--release', '-p', 'lightcal', '--locked') }
    $binary = Join-Path $projectRoot 'target\release\lightcal.exe'
    if (-not (Test-Path -LiteralPath $binary)) { throw 'Build the release executable before packaging' }
    $outputDir = [IO.Path]::GetFullPath((Join-Path $projectRoot "target\msix\$channel"))
    $expectedRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'target\msix')) + [IO.Path]::DirectorySeparatorChar
    if (-not $outputDir.StartsWith($expectedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'MSIX output escaped the workspace staging directory' }
    $stage = Join-Path $outputDir ('stage-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'lightcal.exe')
    foreach ($name in @('LICENSE', 'THIRD_PARTY_NOTICES.md', 'README.md')) {
        Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination (Join-Path $stage $name)
    }
    Copy-Item -LiteralPath (Join-Path $projectRoot 'docs\provider-setup.md') -Destination (Join-Path $stage 'provider-setup.md')
    Run-Native $binary @('--export-msix-assets', (Join-Path $stage 'Assets'))

    [xml]$manifest = [IO.File]::ReadAllText((Join-Path $projectRoot 'packaging\msix\AppxManifest.xml'))
    $namespace = [Xml.XmlNamespaceManager]::new($manifest.NameTable)
    $namespace.AddNamespace('f', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
    $namespace.AddNamespace('uap', 'http://schemas.microsoft.com/appx/manifest/uap/windows10')
    $identity = $manifest.SelectSingleNode('/f:Package/f:Identity', $namespace)
    $identity.SetAttribute('Name', $identityName)
    $identity.SetAttribute('Publisher', $publisher)
    $identity.SetAttribute('Version', $version)
    $manifest.SelectSingleNode('/f:Package/f:Properties/f:DisplayName', $namespace).InnerText = $displayName
    $manifest.SelectSingleNode('/f:Package/f:Properties/f:PublisherDisplayName', $namespace).InnerText = $publisherDisplayName
    $manifest.SelectSingleNode('//uap:VisualElements', $namespace).SetAttribute('DisplayName', $displayName)
    $manifest.Save((Join-Path $stage 'AppxManifest.xml'))

    $package = Join-Path $outputDir "lightcal-$version-x64.msix"
    Run-Native $makeAppx @('pack', '/d', $stage, '/p', $package, '/o', '/h', 'SHA256')
    $checksum = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$package.sha256", "$checksum  $([IO.Path]::GetFileName($package))`n", [Text.UTF8Encoding]::new($false))
    Write-Output "Created unsigned $channel MSIX: $package"
    Write-Output 'No certificate was generated/imported, and no package was installed or submitted.'
} finally { Pop-Location }
