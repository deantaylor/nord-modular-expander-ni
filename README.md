# Nord Modular NI Interface

Control the **official G1-Emu VST** on macOS from a **Maschine MK3** or **Komplete Kontrol S49 Mk2**. The bridge handles knobs, screens and patch browsing. The plugin runs the synth and audio in your DAW, and performance MIDI uses your usual DAW routing.

Install the official **G1-Emu.vst3**, rescan in your DAW, and load **G1-Emu**. Once our background service is installed, the bridge starts automatically when the plugin is available and cleans up when it closes. There is no Terminal launch step for each plugin load.

Double-click **Modular Machine Plugin.command** for Maschine MK3 or **Komplete Keyboard Plugin.command** for the S49 Mk2. The background service uses one surface at a time. Run the other launcher to switch while G1-Emu stays open. See the [Maschine controls](docs/key-assignments.md#maschine-mk3-with-the-official-vst), [S49 guide](docs/komplete-keyboard.md) and [adapter build notes](integrations/g1-official/README.md).

Terminal setup for Maschine MK3:

```sh
cargo build --locked --release
python3 scripts/install_keyboard_service.py --controller maschine
```

Use `--controller komplete` for the S49 Mk2. For full patch editing, use the supported G1 tools. See [scope](docs/scope.md) and [official plugin setup](docs/plugin-guide.md).

## Controller shortcuts

**ARRANGER** on Maschine MK3 or **PATTERN** on S49 Mk2 opens the sequencer page. Display button 5 unlocks the right display for step navigation and editing. Switching it off returns the wheel to sequencer selection on the left. The live playhead uses red circles. Centre press toggles event steps.

**ERASE** on Maschine MK3 or **CLEAR** on the S49 Mk2 restores changed parameters to the current slot’s loaded values, including assigned knobs and Module Editor edits, without reloading the patch. See [key assignments](docs/key-assignments.md).

## Local files

The repository includes three project-owned patch fixtures for development checks. Supply your own compatible ROM and patch library for normal use. See [official plugin setup](docs/plugin-guide.md) for requirements and [third-party notices](screens/THIRD_PARTY_NOTICES.md) for the adapted display code.
