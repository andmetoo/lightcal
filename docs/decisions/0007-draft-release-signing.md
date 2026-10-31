# ADR 0007: Draft release signing

Date: 2026-10-31. Status: accepted by the owner in chat.

SimpleSign Desktop can sign only on the maintainer's Windows PC. The owner asked to follow the Koona Pin release shape: a tag opens a draft, the signed binary is uploaded afterwards, and publication is a separate step.

A stable `vX.Y.Z` tag contained in `main` builds the unsigned Windows application and NSIS setup in CI and attaches them to a GitHub draft. The workflow does not sign, does not call SimpleSign, and does not mark the release public. The unsigned executable is named `lightcal-unsigned.exe`. The updater asset name `lightcal.exe` is reserved for the signed file.

On the maintainer's PC, `just sign-draft` checks out that tag, downloads the draft executable, verifies its SHA-256, signs it with SimpleSign Desktop, verifies Authenticode, builds the NSIS setup from the signed application, signs the setup, and uploads the signed files to the same draft. `just publish-release` removes the unsigned draft files and publishes the release. Neither command stores signing keys in the repository. CI review artifacts remain unsigned.

See the [installer guide](../windows-installer.md).
