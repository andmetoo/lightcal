# ADR 0006: NSIS Windows distribution

Date: 2026-10-30. Status: accepted by the owner in chat. Supersedes ADR 0003.

The owner abandoned MSIX distribution and requested an NSIS installer. Remove that channel's manifest, recipe, workflow, public example settings, and user-facing setup instructions. Keep ignored owner configuration untouched and preserve historical decisions as superseded records.

Ship a per-user Windows x64 NSIS setup with optional shortcuts and an uninstaller that preserves settings and credentials. Keep the Rust/Slint application and portable update payload independent of the installer. Use a checksum-verified portable compiler for reproducible local and CI builds without installing build tools into Windows.

Retain SimpleSign Desktop signing. A signed release verifies the application before embedding it, then signs and verifies the setup. Unsigned local and CI review builds remain explicitly unsigned. No keys are committed and no package is published as part of this work.

Use compile-time isolated installer fixtures for automated installation/removal tests. Real Windows registration, signing, and the owner's app data must not be modified by those tests. See the [installer guide](../windows-installer.md).
