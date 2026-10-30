//! Which version a build reports, compared with GitHub releases, and stamps
//! into the Windows ProductVersion. A stable git tag wins over the Cargo
//! workspace version so a published `vX.Y.Z` binary is that version.

/// Stable `X.Y.Z`, with an optional leading `v`.
/// Prerelease and build metadata are rejected.
pub fn stable_semver(raw: &str) -> Option<&str> {
    let trimmed = raw.trim().strip_prefix('v').unwrap_or(raw.trim());
    if trimmed.is_empty() || trimmed.contains('-') || trimmed.contains('+') {
        return None;
    }
    let mut parts = trimmed.split('.');
    for _ in 0..3 {
        let part = parts.next()?;
        if part.is_empty() || part.parse::<u32>().is_err() {
            return None;
        }
    }
    if parts.next().is_some() {
        return None;
    }
    Some(trimmed)
}

/// `explicit` is `LIGHTCAL_RELEASE_VERSION`. `github_ref` is used only when
/// `github_ref_type` is `tag`. `git_exact_tag` is `git describe --exact-match`.
pub fn resolve_release_version(
    explicit: Option<&str>,
    github_ref_type: Option<&str>,
    github_ref: Option<&str>,
    git_exact_tag: Option<&str>,
    cargo_version: &str,
) -> String {
    let github = match github_ref_type {
        Some("tag") => github_ref,
        _ => None,
    };
    for candidate in [explicit, github, git_exact_tag] {
        if let Some(version) = candidate.and_then(stable_semver) {
            return version.to_string();
        }
    }
    stable_semver(cargo_version)
        .unwrap_or(cargo_version.trim())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_and_explicit_override_win_over_cargo() {
        assert_eq!(
            resolve_release_version(Some("v0.2.2"), Some("tag"), Some("v9.9.9"), None, "0.1.0"),
            "0.2.2"
        );
        assert_eq!(
            resolve_release_version(None, Some("tag"), Some("v0.2.1"), None, "0.1.0"),
            "0.2.1"
        );
        assert_eq!(
            resolve_release_version(None, Some("branch"), Some("v0.2.1"), None, "0.1.0"),
            "0.1.0"
        );
        assert_eq!(
            resolve_release_version(
                Some(""),
                Some("tag"),
                Some("v0.3.0-rc.1"),
                Some("v0.2.2"),
                "0.1.0"
            ),
            "0.2.2"
        );
        assert_eq!(
            resolve_release_version(None, None, None, None, "0.2.2"),
            "0.2.2"
        );
    }
}
