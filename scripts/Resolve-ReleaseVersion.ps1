function Resolve-LightcalReleaseVersion([string]$ProjectRoot) {
    function Convert-StableVersion([string]$Raw) {
        if ([string]::IsNullOrWhiteSpace($Raw)) { return $null }
        if ($Raw.Trim() -match '^v?(\d+\.\d+\.\d+)$') { return $Matches[1] }
        return $null
    }

    $explicit = Convert-StableVersion $env:LIGHTCAL_RELEASE_VERSION
    if ($explicit) { return $explicit }
    if ($env:GITHUB_REF_TYPE -eq 'tag') {
        $tagged = Convert-StableVersion $env:GITHUB_REF_NAME
        if ($tagged) { return $tagged }
    }
    try {
        $described = & git -C $ProjectRoot describe --tags --exact-match 2>$null
        if ($LASTEXITCODE -eq 0) {
            $fromGit = Convert-StableVersion "$described"
            if ($fromGit) { return $fromGit }
        }
    } catch {
        # Cargo.toml remains the fallback when git is unavailable.
    }
    $manifest = [IO.File]::ReadAllText((Join-Path $ProjectRoot 'Cargo.toml'))
    $match = [regex]::Match($manifest, '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$')
    if (-not $match.Success) { throw 'Expected a stable workspace version' }
    return $match.Groups[1].Value
}
