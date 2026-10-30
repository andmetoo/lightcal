//! Version embedded by `build.rs` for About, update checks, and the Windows file version.
pub const VERSION: &str = env!("LIGHTCAL_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_version_matches_a_current_release() {
        let parsed = crate::updater::SemVer::parse(VERSION).expect("stable release version");
        assert_ne!(VERSION, "0.1.0");
        assert!(
            parsed
                >= crate::updater::SemVer {
                    major: 0,
                    minor: 2,
                    patch: 1
                }
        );
    }
}
