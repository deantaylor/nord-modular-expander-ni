# Deferred standalone feature — 17 November 2026

The S49 two-zone split is documented for future standalone software, not current VST implementation. See [design](standalone-keyboard-split.md). Do not add a Live virtual MIDI route or modify the official VST for this deferred feature.

# Current scope — 18 November 2026

The **Komplete Kontrol S49 Mk2** and **Maschine MK3** are selectable surfaces. The service runs one surface at a time. Use the **official unmodified G1-Emu VST**; this project supplies an external controls/display bridge, started automatically in the background when the plugin is available. Keybed MIDI remains independent in the DAW. The locally branded Modular Machine G1 plugin approach is superseded and must not be extended.

The external adapter uses the official DirectLink/PC Port editor protocol for patch and parameter feedback. No custom editor, synth engine, modified VST or guessed state. Existing historical implementation notes below are retained for reference and do not override this scope.

# Product scope — clarified 11 November 2026

Latest user instructions take precedence over the broader original handoff. We are running an existing Nord emulator on the Mac and using Maschine MK3 as a control surface for selected functions.

## Responsibilities

- **Mac emulator:** synthesis, audio output, actual Nord patch loading/state and its existing editor. Keep these responsibilities in the emulator and its supported tools.
- **Maschine MK3:** browse/select/load patches, turn eight encoders to change selected active-patch parameters, navigate parameter pages as needed, and show useful patch/parameter feedback on its screens.
- **This application:** the smallest practical adapter connecting MK3 controls and displays to the emulator's real control/state interface. Refresh the hardware when patches or relevant parameter values change.

Use the existing editor for full patch construction and cable/module editing. There is no custom or mock editor deliverable. Do not build another synth engine, reproduce a complete Nord editor, invent a parallel production patch format, or make remaining generic Encdr checks a prerequisite for emulator integration.

## First actual integration

1. Verify G1-Emu running independently on Apple Silicon, producing audio with the user's compatible local ROM and a permitted patch.
2. Inspect the real backend interfaces for listing/loading patches, observing the current patch, and reading/writing module parameters. Do not assume standard MIDI CC exposes arbitrary patch parameters.
3. Connect one known patch and two mapped parameters end to end. Demonstrate that a Maschine encoder changes an actual emulator value/audio response, and the display reflects current backend state.
4. Add patch browse/load on Maschine, then expand the mapping to eight encoders and useful pages. Keep external-editor synchronization limited to relevant state the backend actually exposes.

Choose the bridge from verified upstream support: documented API, process IPC, supported MIDI/SysEx, or a small isolated upstream integration if necessary. No commitment to an unverified protocol or compatibility claim.

## Current implementation status

User-confirmed hardware proof: dual screens; eight encoders both ways; top/group buttons and pads; reconnect with retained counters; STOP exit; LEDs off and Encdr logo on exit. Remaining edge checks are documented but do not block the next integration work.

The optional `--g1` controller now communicates with a locally patched G1-Emu standalone. Its real ROM/DSP/CoreAudio integration check confirms two knob assignments, recorded frequency/level changes, stored-patch browsing/reload and rendered value feedback. All 18 G1 panel knob assignments are exposed across three pages. The user confirmed physical frequency/level control with sound/value feedback, then browsing/loading their patches from Maschine. Simultaneous existing-editor synchronization remains outstanding; the earlier hardware evidence is accepted. See [G1 integration](../integrations/g1-emu/README.md). Plain launch still uses developer fixtures. Typed patch/module/parameter data and an EngineAdapter test double support developing the real bridge. Hardware-free screen simulation and optional developer JSON fixtures remain test tools. The separate mock-editor window, commands and IPC were removed following the scope correction. Developer JSON is opt-in via --state-file and is not a Nord patch format or a replacement patch library.

G1 is first. G2 is a separate future feasibility question. No ROM or proprietary patch is bundled or downloaded by this repository.

## Agreed controls

The user defined the working mapping: four bottom encoder tiles per display, Groups A–D slots, Macro knob pages, physical < / > preset switching, held Shift as the real Function input, Settings for actual emulator configuration, Mixer for master volume, and state-driven button lights with inactive controls off. These are implemented; see [key assignments](key-assignments.md). Settings UI is explicitly in scope and reuses real upstream state/APIs and the existing Mac settings card; it does not add a patch editor. Further controls remain undefined until requested.

## DAW formats — user requested

The user requested VST versions using upstream G1-Emu support. The same controller can now attach to locally branded VST3/AU instrument and FX instances. Audio/transport/state remain in the upstream DAW plugin; the external Maschine controller selects a live instance and reuses its real parameters, patches and MIDI input. This adds no synth engine or editor. See [plugin guide](plugin-guide.md).

## Additional keyboard — user requested

The connected Komplete Kontrol S49 Mk2 (`17cc:1610`) can be selected as an alternative surface using the same G1 state/transport/screens. Keys use upstream standalone MIDI input or DAW track MIDI. Maschine remains the default profile; this does not add a synth engine or a patch editor. Other keyboard models and simultaneous dual-surface control are not claimed. See [keyboard guide](komplete-keyboard.md).
