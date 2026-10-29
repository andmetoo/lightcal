# ADR 0003: Prepare a Microsoft Store MSIX distribution channel

Date: 2026-10-30. Status: packaging preparation requested by the owner; real Store identity, first publication, and submitting automation remain pending.

The owner asked for Microsoft-managed signing and GitHub-based delivery for a free application. The applicable free signing channel is Microsoft Store MSIX: GitHub can build the package, and Microsoft signs/re-signs and hosts it after certification. This does not provide a free general-purpose Authenticode certificate for portable executables on GitHub. Microsoft Azure Artifact Signing for independent distribution is a separate paid service; no Azure subscription or signing account was created.

Keep the native Rust/Slint application. Add an unsigned MSIX packaging recipe and a read-only GitHub Actions build workflow. Store-mode identity must come from Partner Center; an explicit preview mode uses a separate test identity. Do not fabricate production publisher details or silently submit preview packages.

MSIX deployment owns installed binaries, so GitHub portable self-replacement is blocked when Windows reports package identity. Package update UI explains the package-managed channel. Portable SimpleSign builds remain supported independently under ADR 0002.

The first app publication must be completed through Partner Center. Official CLI/GitHub Actions automation can handle subsequent free-app updates after the app is live and the owner sets up API access. A publishing workflow was not added: automatic approval review rejected that step as outside the current build-without-submission scope. No workaround, API submission, secret provisioning, or remote publishing was attempted.

Read [Microsoft Store setup](../store-distribution.md) for public identity configuration, validation evidence, and remaining package lifecycle/certification checks.
