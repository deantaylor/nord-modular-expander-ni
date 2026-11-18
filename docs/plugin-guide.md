# Official G1-Emu VST workflow

Use the official **G1-Emu.vst3** or **G1-Emu FX.vst3**, installed under `~/Library/Audio/Plug-Ins/VST3`. The current local reference is upstream **v0.1.0-alpha.30**, copied unchanged from its macOS universal release. The archive SHA-256 is `d23699d13a54a3ceef30482407142c790cb0549f0d798af4fae060a422b74c8d`.

The older locally branded **Modular Machine G1** instrument/FX builds are superseded. They remain installed for existing-project recovery; this work does not replace plugins inside your DAW project automatically. Rescan and explicitly load the official **G1-Emu** instance for the new workflow.

The official VST runs the synth/audio and holds DAW state. Our external keyboard adapter speaks its existing DirectLink/PC Port protocol. No custom VST build, patched synth or second audio engine is required. The background service starts the selected Maschine MK3 or S49 Mk2 controller automatically when a plugin is available and releases it when the plugin closes. Performance MIDI retains ordinary DAW routing.

Choose the profile using **Maschine.command** for Maschine MK3 or **KompleteKontrol.command** for S49 Mk2. See [Maschine controls](key-assignments.md#maschine-mk3-with-the-official-vst), [S49 setup and diagnostics](komplete-keyboard.md), [official upstream release/setup](https://github.com/animatek/G1-Emu) and [external adapter build notes](../integrations/g1-official/README.md).

The previous custom-plugin implementation, tests and source patches are retained as historical material under `integrations/g1-emu/` and `docs/CONTEXT.md`; their JSON ports and manual launch flow are not the current product architecture. The old `build/package` custom-plugin app/package is also historical and should not be used for the official-VST workflow.
