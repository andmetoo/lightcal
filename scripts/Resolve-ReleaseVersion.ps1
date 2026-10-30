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
    # Untagged commits make `git describe --exact-match` exit 1. GitHub Actions
    # dot-sources this script with pwsh -Command, and that host exits with the
    # last native code. Run-Native uses Process.Start, so it does not clear it.
    $described = $null
    $gitOk = $false
    try {
        $described = & git -C $ProjectRoot describe --tags --exact-match 2>$null
        $gitOk = $LASTEXITCODE -eq 0
    } catch {
        $gitOk = $false
    } finally {
        $global:LASTEXITCODE = 0
    }
    if ($gitOk) {
        $fromGit = Convert-StableVersion "$described"
        if ($fromGit) { return $fromGit }
    }
    $manifest = [IO.File]::ReadAllText((Join-Path $ProjectRoot 'Cargo.toml'))
    $match = [regex]::Match($manifest, '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$')
    if (-not $match.Success) { throw 'Expected a stable workspace version' }
    return $match.Groups[1].Value
}
