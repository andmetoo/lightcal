# ADR 0007: Cloud builds and local release signing

Date: 2026-10-31. Status: accepted by the owner in chat.

Amended: 2026-11-01 at the owner's request to remove draft-only and unsigned-name restrictions.

Certum SimplySign Desktop can sign only on the maintainer's Windows PC. Signing calls `signtool` with `SIGN_CERT_SHA1` while that app is signed in; it does not launch a separate signer executable. The owner asked to follow the Koona Pin release shape: a tag opens a draft, the signed binary is uploaded afterwards, and publication is a separate step.

A stable `vX.Y.Z` tag contained in `main` builds the unsigned Windows application and NSIS setup in CI. It attaches ordinary release filenames to the existing release regardless of its title or draft/prerelease/public status. If no release exists, CI creates a draft. CI never changes an existing release's visibility or overwrites existing assets, so reruns cannot replace signed binaries with unsigned ones. A manual run on `main` can rebuild an existing `release_tag` using the current workflow. The upload job consumes the build artifact without a second checkout.

On the maintainer's PC, `just sign-draft vX.Y.Z` downloads `lightcal.exe` and its checksum (or the legacy `lightcal-unsigned.exe` pair). If the release has no executable yet, it recovers the unexpired Windows Actions artifact from a completed tag-push run whose commit matches the current tag, even if the release-upload job failed. It verifies SHA-256 and ProductVersion before signing, signs the application with `signtool`, verifies Authenticode, rebuilds NSIS with that signed application, signs the setup, and replaces the release assets. The portable ZIP and setup retain their ordinary filenames throughout. Signing does not check out or recreate a tag, and preserves an existing release's status. If necessary, it creates a draft only after signing has succeeded.

`just publish-release vX.Y.Z` is the explicit publication/promotion step. On Windows it downloads the six release assets, verifies their checksums, both executable signatures, the application version, and that the portable ZIP embeds the exact signed application. Only then does it remove legacy unsigned assets and publish/promote the release. Asset names and release titles are not evidence of signing. Neither command stores signing keys in the repository. CI review artifacts remain unsigned. The updater still requires a valid signature and the configured certificate pin before installing `lightcal.exe`; filenames alone do not authorize installation.

See the [installer guide](../windows-installer.md).
