# Verify the archive embeds the exact application whose Authenticode was checked.
param([Parameter(Mandatory)][string]$Archive, [Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($Archive)
try {
    $payloads = @($zip.Entries | Where-Object { $_.Name -ieq 'lightcal.exe' })
    if ($payloads.Count -ne 1) { throw 'Portable archive must contain exactly one lightcal.exe' }
    $stream = $payloads[0].Open()
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $hash = [Convert]::ToHexString($sha.ComputeHash($stream)).ToLowerInvariant() }
    finally { $stream.Dispose(); $sha.Dispose() }
    if ($hash -ne (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash.ToLowerInvariant()) {
        throw 'Portable archive does not contain the verified signed application'
    }
} finally { $zip.Dispose() }
