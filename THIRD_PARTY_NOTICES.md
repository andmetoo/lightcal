# Third-Party Notices

Lightcal is distributed under the GNU General Public License v3.0 only (`GPL-3.0-only`). See [LICENSE](LICENSE) for the full license text.

This distribution incorporates the following open-source Rust crates:

| Crate | Version | License (SPDX) | Upstream Repository |
| --- | --- | --- | --- |
| `slint` / `slint-build` | `1.18.1` | `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0` | https://github.com/slint-ui/slint |
| `chrono` | `0.4.45` | `MIT OR Apache-2.0` | https://github.com/chronotope/chrono |
| `serde` | `1.0.229` | `MIT OR Apache-2.0` | https://github.com/serde-rs/serde |
| `serde_json` | `1.0.154` | `MIT OR Apache-2.0` | https://github.com/serde-rs/json |
| `tray-icon` | `0.25.1` | `Apache-2.0 OR MIT` | https://github.com/tauri-apps/tray-icon |
| `muda` | `0.20.0` | `Apache-2.0 OR MIT` | https://github.com/tauri-apps/muda |
| `windows` | `0.61.0` | `MIT OR Apache-2.0` | https://github.com/microsoft/windows-rs |
| `windows-sys` | `0.61.2` | `MIT OR Apache-2.0` | https://github.com/microsoft/windows-rs |
| `fontdue` | `0.9.3` | `MIT OR Apache-2.0` | https://github.com/redox-os/fontdue |
| `winres` | `0.1.12` | `MIT` | https://github.com/mxre/winres |
| `ical` | `0.11.0` | `Apache-2.0` (upstream LICENSE file) | https://github.com/Peltoche/ical-rs |
| `rrule` | `0.14.0` | `MIT OR Apache-2.0` | https://github.com/fmeringdal/rust-rrule |
| `chrono-tz` | `0.10.4` | `MIT OR Apache-2.0` | https://github.com/chronotope/chrono-tz |
| `ureq` | `3.5.0` | `MIT OR Apache-2.0` | https://github.com/algesten/ureq |
| `native-tls` | `0.2.18` | `MIT OR Apache-2.0` | https://github.com/rust-native-tls/rust-native-tls |
| `sha2` | `0.10.9` | `MIT OR Apache-2.0` | https://github.com/RustCrypto/hashes |
| `socket2` | `0.6.5` | `MIT OR Apache-2.0` | https://github.com/rust-lang/socket2 |
| `base64` | `0.22.1` | `MIT OR Apache-2.0` | https://github.com/marshallpierce/rust-base64 |
| `getrandom` | `0.3.4` | `MIT OR Apache-2.0` | https://github.com/rust-random/getrandom |
| `url` | `2.5.8` | `MIT OR Apache-2.0` | https://github.com/servo/rust-url |

## Design & Typography References

- **Inter Font**: `crates/lightcal/assets/fonts/Inter.ttf` and the static `Inter-Bold.ttf` instance are licensed under the SIL Open Font License, Version 1.1 (Copyright 2016-2024 The Inter Project Authors, Rasmus Andersson). Full license text is preserved in `crates/lightcal/assets/fonts/OFL.txt`. Bundled for UI and icon rasterization. `Inter-Bold.ttf` is generated from the bundled variable font at `wght=700`, `opsz=14` using fontTools 4.61.1; the font's OFL metadata is preserved.
- **NSIS installer runtime**: the Windows setup and uninstaller use NSIS (zlib/libpng license), Modern UI, and the public-domain LZMA compressor. The full compiler-distribution notice is installed as `NSIS-LICENSE.txt`; see https://nsis.sourceforge.io/License. NSIS is a packaging tool, not a Cargo dependency.
- UI design mockups in `design/` use the open-source **Inter** (SIL Open Font License 1.1) and **JetBrains Mono** (SIL Open Font License 1.1) typefaces in Penpot. The compiled desktop calendar UI uses system fonts (`Segoe UI`, `Cascadia Mono`, `Yu Gothic UI`) at runtime.

