# Microsoft Store MSIX distribution

Reviewed on 2026-10-30 against the official sources below.

## What is free

For **MSIX distributed through Microsoft Store**, Microsoft signs/re-signs the package after certification, hosts it, and provides its update channel. A CA code-signing certificate, SimpleSign identity, or Azure Artifact Signing subscription is not required for that channel. Free app pricing and Store signing are separate settings: the owner chooses free pricing in Partner Center.

Store signing does not sign a portable `lightcal.exe` hosted in GitHub Releases. Direct MSIX distribution outside Store still needs an appropriate trusted signing/distribution arrangement. Azure Artifact Signing is a separate paid service. Do not describe this workflow as a free public certificate or promise a Microsoft-signed standalone GitHub EXE.

Current Microsoft onboarding documentation describes free registration for individual and company accounts in the new flow; the owner already has a Store developer account, so no new account purchase or registration was attempted. GitHub standard runners are free for public repositories; private-repository compute and artifact/cache storage have plan allowances. No spending limit or billing setting was changed.

## Implemented locally

- `packaging/msix/AppxManifest.xml`: full-trust native desktop application, Windows 11 baseline, four language declarations, and package artwork.
- `scripts/package-msix.ps1`: builds locked release code, stages only the executable/licensing/readme/setup documents and artwork, fills identity using XML APIs, invokes SDK MakeAppx with validation, and writes a package SHA-256. It never installs, signs, creates a certificate, uploads, or submits a package.
- `just build-msix-preview`: an unsigned review package with `Lightcal.Preview` / `CN=LightcalPreviewOnly`. This is not a Store identity and is not a generally installable signed package.
- `just build-msix`: an unsigned submission package using real Partner Center public values; missing or preview identities are rejected.
- `.github/workflows/windows-msix.yml`: pull-request/manual packaging and artifact upload only. It has read-only repository permissions and no Partner Center credentials or publishing step. It uses a preview identity when production identity variables are absent.
- The app checks Windows package identity before portable update checking/staging. MSIX installs explain that updates belong to Store/the deployment channel and hide portable update controls. Merely being packaged is not proof of having been Store-certified; sideloaded packages are also package-managed.

Preview output: `target/msix/preview/lightcal-0.1.0.0-x64.msix` and its `.sha256`. Generated icons reuse the existing calendar artwork as static package branding; runtime tray date rendering remains separate.

## Public configuration

In Partner Center, create/reserve the app name under **Apps and games** and open **Product identity**. This app entry is separate from the Microsoft Graph OAuth registration used to read Outlook calendars.

Copy these exact values into ignored `.env.local` for local packaging, or repository **Actions variables** for CI:

```dotenv
LIGHTCAL_MSIX_IDENTITY_NAME=
LIGHTCAL_MSIX_PUBLISHER=
LIGHTCAL_MSIX_PUBLISHER_DISPLAY_NAME=
LIGHTCAL_MSIX_DISPLAY_NAME=Lightcal
```

| Configuration | Partner Center value |
| --- | --- |
| `LIGHTCAL_MSIX_IDENTITY_NAME` | `Package/Identity/Name` |
| `LIGHTCAL_MSIX_PUBLISHER` | `Package/Identity/Publisher`, including `CN=` |
| `LIGHTCAL_MSIX_PUBLISHER_DISPLAY_NAME` | `Package/Properties/PublisherDisplayName` |
| `LIGHTCAL_MSIX_DISPLAY_NAME` | The reserved Store display name if different from Lightcal |

These are public publisher/package identifiers, not signing material or account tokens. Do not substitute the OAuth Client ID, Store Product ID, a guessed company name, or a local certificate subject for the package identity assigned by Partner Center.

The workspace version `major.minor.patch` becomes MSIX `major.minor.patch.0`. Increase the version for a new package/update. The script targets x64 only and does not include unrelated files, `.env.local`, credentials, or private calendars.

## First publication and later automation

First build with the real identity, then use Partner Center for the first submission: free pricing, supported markets, descriptions/screenshots, privacy/data disclosures, age ratings, package upload, and certification. No first publication has occurred in this session. MakeAppx validation does not replace WACK, certification, or real installation tests.

Microsoft's current Store Developer CLI documentation requires the app's first publication to complete through Partner Center before CLI updates. The official GitHub Actions integration currently supports free products. For later automation the owner must associate the Partner Center tenant and a separate Entra service application with appropriate Partner Center permissions.

That CI service application is **not** the public desktop OAuth client: Store publishing credentials belong in protected GitHub environment secrets, not the desktop app, `.env.local`, code, logs, or chat. The intended future workflow should require an owner-approved `microsoft-store` environment, trusted branch, and explicit submission trigger. No publishing workflow or credentials were added because automatic approval review rejected that step under the current local/build-only scope. Obtain explicit owner authorization before adding it; never simulate authorization by flipping a CI variable or creating an indirect upload script.

## Validation evidence and remaining gates

Local release packaging with Windows SDK MakeAppx 10.0.28000 succeeded using the preview identity and normal validation. Workspace unit tests and Clippy with warnings denied passed after adding the package-update guard. No certificate was generated/imported and no package was installed, registered, submitted, or published. The GitHub workflow has not run remotely because changes remain unpushed.

Before a real Store release, validate:

1. Exact identity matching, logos/listing metadata, increasing version, WACK, and certification.
2. Installation, uninstallation, packaged data/DPAPI behavior, OAuth loopback redirect, tray lifetime, single-instance activation, shortcut/pinned activation, and Explorer restart in a disposable profile/VM.
3. Packaged startup behavior: the existing portable HKCU Run implementation is not validated as an MSIX startup task. Complete/adapt and verify it before advertising start-with-Windows for Store installs. Optional legacy Windows calendar capabilities also need packaging review; Graph/ICS remain primary.
4. Package update detection in a real installed MSIX. Portable EXE replacement is blocked in code but real Store upgrade delivery remains untested.
5. Matching GPL source availability and complete transitive attribution before distribution. The Microsoft Store signing service does not resolve the project's source/license obligations.
6. Preserve separate portable-signing credentials, behavior, and artifact labels if also publishing GitHub downloads. A Store package is not an MSIX wrapper around our portable replacement updater.

## Official references

- [MSIX signing options and Store signing](https://learn.microsoft.com/en-us/windows/msix/package/signing-package-overview)
- [Store MSIX package requirements](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-package-requirements)
- [Distribution paths, signing and update differences](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/choose-distribution-path)
- [Store developer account onboarding](https://learn.microsoft.com/en-us/windows/apps/publish/partner-center/open-a-developer-account)
- [First publication prerequisite for Store CLI](https://learn.microsoft.com/en-us/windows/apps/publish/msstore-dev-cli/overview)
- [GitHub Actions for subsequent Store updates](https://learn.microsoft.com/en-us/windows/apps/publish/msstore-dev-cli/github-actions)
- [Native MSIX manifest and MakeAppx packaging](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion)
- [GitHub Actions billing and public standard runners](https://docs.github.com/en/billing/concepts/product-billing/github-actions)
