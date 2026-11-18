# Legacy G1-Emu integration

This directory contains the previous in-process controller bridge and its reproducible upstream patch. The [official VST workflow](../../README.md#setup) uses the external adapter instead. Retain the older plugin names and IDs for DAW projects that already refer to those builds.

## Build and run

```sh
python3 scripts/build_g1.py
cargo build --locked
python3 scripts/run_machine.py --controller maschine
```

Run commands from the repository root. The builder fetches pinned source into `build/g1-work/`, applies [standalone.patch](standalone.patch), copies the bridge/helper sources, and builds the standalone app under `build/g1-emu/`. Pass `--source-dir`, `--gearmulator-dir`, `--dsp-dir`, `--juce-dir` and `--build-dir` for custom checkout/build paths. The script rejects different revisions.

Use a compatible local rack OS 3.03 ROM at `roms/NORD-MODULAR-RACK-VER-3.03.BIN` or pass `--rom PATH` to the launcher. The launcher supports `--patch-dir PATH`, `--flash PATH` and `--port PORT`; use disposable banks and a separate port for testing. `--demo` seeds a new dedicated bank. Exiting the controller leaves the synth running. Native bridge changes require a normal emulator restart after saving the user's work.

Build with `--plugins` to produce the historical **Modular Machine G1** instrument and **Modular Machine G1 FX** VST3/AU and the test host used by the official adapter fixtures. To attach to a legacy plugin, run `python3 scripts/run_machine.py --plugin --port PORT --no-build` with the port shown in its Settings.

## Source pins and maintenance

| Source | Revision |
| --- | --- |
| [G1-Emu](https://github.com/animatek/G1-Emu) | `efd178d8a6e13a38c74b5cce02853fc7dccbf5c1` |
| [gearmulator-md-mm](https://github.com/joelanders/gearmulator-md-mm) | `ab69ad003f1268d1ef373fbb5a47340269b6dd47` |
| [dsp56300](https://github.com/dsp56300/dsp56300) | `8b731966f9792789adc39fe180a8cf68508f816a` |
| [JUCE](https://github.com/juce-framework/JUCE) | `29396c22c93392d6738e021b83196283d6e4d850` |

[scripts/build_g1.py](../../scripts/build_g1.py) checks out the matching mc68k and asmjit submodule revisions. The bridge code and upstream modification are GPL-3.0-or-later. See [licensing](../../docs/licensing.md) for source distribution requirements.

[Mk3ControlBridge.h](Mk3ControlBridge.h) implements the opt-in `G1_MK3_PORT` loopback protocol, separate from DirectLink. Socket I/O and bounded engine work are isolated. Writes use confirmed patch identity and bank/file loads require the matching firmware outcome. The standalone path uses editor parameter messages when the patch ID is known and supports pad MIDI through the existing emulator input queue.

Focused checks include [check_g1.py](../../scripts/check_g1.py), [check_encoder_path.py](../../scripts/check_encoder_path.py) and [test_launcher.py](../../scripts/test_launcher.py). Developer packaging uses [package_local.py](../../scripts/package_local.py) and [check_package.py](../../scripts/check_package.py). The source packager currently expects the checkout locations in its `UPSTREAM` map; prepare those with the builder's directory overrides.
