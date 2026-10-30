# ADR 0003: Microsoft Store package channel (superseded)

Date: 2026-10-30. Status: superseded by [ADR 0006](0006-nsis-windows-distribution.md).

The owner initially requested preparation of a Microsoft Store MSIX channel, then explicitly abandoned it in favor of an NSIS installer. The MSIX manifest, packaging recipe, workflow, build recipes, and setup guide have been removed. Old local build artifacts and public identity values in ignored owner configuration are not an active distribution channel. Nothing was submitted or published.

Use the [NSIS installer guide](../windows-installer.md) for the current Windows build and signing workflow. Runtime package-identity checks may remain defensively for previously packaged copies; they do not provide or build an MSIX channel.
