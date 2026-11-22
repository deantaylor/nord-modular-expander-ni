# Licensing and third-party sources

The original Nord Modular Expander for Ni Controllers code and the three development patch fixtures are licensed under the [GNU General Public License, version 3 only](../LICENSE). Third-party components retain the licenses and notices listed below.

## Included in the repository

| Component | Terms and provenance |
| --- | --- |
| Encdr and Encdr View | Vendored and modified from RufusIbiza/Encdr **0.8.5**, commit [`4e1fa75`](https://github.com/RufusIbiza/Encdr/tree/4e1fa75aa329fc18823be88e6a0c1b385f53f6ac), under GPL-3.0-or-later. Each crate has its own license and change record. [Encdr license](../vendor/encdr/LICENSE) · [Encdr changes](../vendor/encdr/PATCHES.md) · [Encdr View license](../vendor/encdr-view/LICENSE) · [Encdr View changes](../vendor/encdr-view/PATCHES.md) |
| Module diagrams | ADSR and Filter E/F drawing geometry adapted from Animatek NME at the pinned revision below, under GPL-3.0-only. [Attribution](../screens/THIRD_PARTY_NOTICES.md) · [GPL v3 text](../screens/GPL-3.0.txt) |
| Inter | Copyright 2020 The Inter Project Authors. SIL Open Font License 1.1. [Font provenance](../screens/fonts/README.md) · [OFL text](../screens/fonts/OFL.txt) |
| Eastlab artwork | Project-supplied [SVG](../screens/assets/eastlab-logo-dark.svg) and [generated startup image](../vendor/encdr/assets/eastlab-loading.png). The startup image lives in the vendored crate because Encdr renders it. The repository has no documented redistribution terms for this artwork. |
| Historical standalone bridge | Bridge sources and upstream modifications are GPL-3.0-or-later. [Source pins and integration notes](../integrations/g1-emu/README.md) · [Reproducible build script](../scripts/build_g1.py) |

## Installed or fetched separately

| Component | Terms and provenance |
| --- | --- |
| G1-Emu | The official GPL v3 VST runs separately. The supported workflow uses the unchanged macOS universal release **v0.1.0-alpha.30**. [Upstream release](https://github.com/animatek/G1-Emu/releases/tag/v0.1.0-alpha.30) · [Setup](../README.md#setup). macOS archive SHA-256: `d23699d13a54a3ceef30482407142c790cb0549f0d798af4fae060a422b74c8d` |
| Animatek NME | The adapter builds against upstream protocol and patch model code under GPL v3. [Pinned source `0ded1bb`](https://github.com/animatek/Animatek-NME/tree/0ded1bb77ca2431b7ff52a8b55076430d85ac48d) · [License](https://github.com/animatek/Animatek-NME/blob/0ded1bb77ca2431b7ff52a8b55076430d85ac48d/LICENSE) |
| JUCE | The adapter builds against JUCE modules offered under AGPL v3 or the commercial JUCE license. [Pinned source `29396c2`](https://github.com/juce-framework/JUCE/tree/29396c22c93392d6738e021b83196283d6e4d850) · [License terms](https://github.com/juce-framework/JUCE/blob/29396c22c93392d6738e021b83196283d6e4d850/LICENSE.md) |
| Rust dependencies | Registry dependencies are pinned in [Cargo.lock](../Cargo.lock) and retain their own terms. Run `cargo metadata --locked --format-version 1` for package license metadata, then consult the corresponding source notices. |

Keep licenses, attribution, local modification notices, and complete corresponding source with any distributions that require them. The Eastlab artwork still needs documented redistribution terms. The ROM, firmware, NI drivers, and user patch libraries are supplied separately.
