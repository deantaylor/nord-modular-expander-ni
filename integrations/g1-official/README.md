## Sequencer parameter checks

The PATTERN page uses the existing module catalogue/selection and parameter RPCs for NoteSeqA, NoteSeqB, CtrlSeq and EventSeq. It does not add a sequencer engine. Live playheads use the existing NMInfo meter stream mapped through the pinned LightMeterLayout and classic theme, independent from stored/editable step parameters. Run `python3 scripts/check_official_plugin.py --sequencer-only` for a private-fixture test of all four module types and CLEAR restoration without reload. `--playhead-only` checks advancing real playheads in a clocked common-area fixture without keybed MIDI.

## Slot parameter restore

`restore_slot` accepts `slot` and current `patchBinding`; `status.restore` reports `baselineAvailable`, `available`, `busy`, `serial`, `remaining`, `total`, `written`, `slot`, and `message`. Values are captured from the first complete slot snapshot/each new load, retained through section readback, and restored using bounded writes of changed/unconfirmed parameters plus confirmed readback. `available` means parameter changes exist; `baselineAvailable` only means the baseline is valid. Clean-slot restore sends no writes. No upload/reload is used. Baselines are session-local; unknown pre-connection edits cannot be undone. Protected regression: `python3 scripts/check_official_plugin.py --restore-only`.

# External bridge for the official G1-Emu VST

This adapter is a standalone process. It contains **no synth engine** and does not patch or rebuild the plugin. The official VST owns ROM, DSP, audio and project state.

`Adapter.cpp` reuses Animatek NME's production `ConnectionManager`, DirectLink transport, protocol codecs, patch parser/model, patch upload serializer and parameter formatters. It exposes the controller's loopback JSON protocol externally, translating supported operations into the official editor PC Port protocol. The Rust controller renders the screens and dispatches hardware input for the selected S49 Mk2 or Maschine MK3.

## Pinned sources

- Official macOS universal plugin archive: `animatek/G1-Emu`, release `v0.1.0-alpha.30`; SHA-256 `d23699d13a54a3ceef30482407142c790cb0549f0d798af4fae060a422b74c8d`.
- Animatek NME: `0ded1bb77ca2431b7ff52a8b55076430d85ac48d`.
- JUCE: `29396c22c93392d6738e021b83196283d6e4d850`.

NME protocol/model code is used unchanged under its GPL license. Local adapter sources, controller/vendor modification notices and upstream licenses must accompany redistribution. No ROM, commercial patches, banks or recordings are included. The existing user's ROM is selected in the official plugin itself.

## Build and install

```sh
cargo build --release --locked
python3 scripts/build_official_adapter.py
python3 scripts/install_keyboard_service.py
```

For existing pinned checkouts, pass `--nme-dir PATH --juce-dir PATH` to the adapter builder. Its CMake target links NME protocol/model code and JUCE; it does not link `g1Lib`, `g1Host`, `Engine`, `Runner` or DSP code.

The service installer stages a new immutable runtime directory before replacing its own per-user LaunchAgent and restores the previous registration if bootstrap fails. It does not install a synth, modify a DAW project, open key MIDI ports, stop NI services or terminate another controller owner.

## Lifecycle

The small Python watcher scans official DirectLink ports `47310–47317`, accepts plugin greetings and ignores standalone greetings. It selects the first available instance or an explicitly configured port. One service/file lock prevents duplicates. A shared Application Support launcher lock prevents concurrent manual hardware ownership. The installed watcher reads no project files from Documents; user patch imports live separately in private Application Support data. The adapter owns the single DirectLink connection while attached; another editor is not displaced.

When the plugin appears, the watcher starts an external adapter and the selected Rust controller with hidden views and background activation policy. When the plugin disappears, the adapter exits and the watcher gracefully stops only its controller, then returns to idle. No work runs inside the VST's audio callback and no new plugin binary is produced.

## State and capabilities

Patch downloads supply module metadata, parameters and the 18 knob assignments. Actual parameter updates are sent with slot/patch identity; readback asks for the affected section and updates displayed values from the returned data. Stale bindings are rejected. A partial full-patch download keeps the previous snapshot and is not marked successful. Synth-settings and allocation notifications supply slot/voice state; requested voices come from the confirmed patch header.

Supported operations include slots, patch files/bank listing/loading, normal module parameter knobs, voice counts and PC-Port synth settings. Hardware master/gain and native host Settings UI are not exposed by this protocol adapter. Mapping changes made elsewhere require refreshed patch metadata; concurrent external editor operation and morph assignments need further verification. No keybed MIDI routing or DAW MIDI clip handling is implemented in this surface process.

## Focused checks

```sh
python3 scripts/check_official_plugin.py
python3 scripts/check_keyboard_service.py
# After installation, with no user plugin active:
python3 scripts/check_installed_keyboard.py
python3 -m unittest discover -s scripts -p test_keyboard_service.py
```

The first loads the **official release VST** in the project test host and changes RAM-only fixtures. It verifies that the shared bank file is unchanged. The second checks automatic discovery, one background controller, duplicate suppression and plugin-close cleanup using hidden views without claiming physical hardware. Both use logs under ignored `g1-state/`.

S49 control correction (13 November 2026): no Function/SHIFT binding. A former dial event was misidentified as SHIFT and is now ignored. Any adapter Function API retained for compatibility/tests does not establish a keyboard binding. SETUP stays hardware-native; see [control ownership policy](../../docs/komplete-keyboard.md#native-controls-and-conflicts).

Continuous encoder readback preserves the first40ms read deadline; later writes cannot debounce it indefinitely. Due reads take priority over the next write. Run `python3 scripts/check_official_plugin.py --readback-only` for the bounded reversing sweep. Official alpha.30 ignores legacy G1_DATA_DIR: this test requires macOS sandbox-exec to protect the shared bank/settings paths, rather than claiming that environment variable isolates stock plugins. It does not start audio devices or restart the user's plugin.

Latest latency follow-up supersedes the40ms deadline above: targeted parameter readback is due after16ms, and writes use the standalone-verified $13/$40 frame without checksum/extra focus. A coalesced high-resolution wake-up services the model on a persistent macOS CF run loop; no NME callbacks move to worker threads. See [measurement evidence](../../docs/encoder-response.md).

Physical-feedback correction supersedes the experimental sender/read-first policy: restore NME sendParameterFromUser; live coalesced writes take priority and may proceed during targeted section reads, still respecting protocol wait/full-patch guards. First read due16ms; starts capped at one per40ms. S49 capture reverted33ms. See latest [latency evidence](../../docs/encoder-response.md); do not treat fixture timings as panel-feel acceptance.

Multi-second stall fix: targeted read queries set expectsReply=false because the adapter owns their assembly/deadline; they must not acquire NME’s global3s reply wait. SDK full fetch/upload waits stay protected. Automatic2s settings refresh is removed; startup and explicit setting changes still refresh. `python3 scripts/check_official_plugin.py --lost-reply` verifies actual wire writes continue under missing responses and confirmed state recovers.

Module Edit API: modules returns a lazy patch-bound catalogue; module_select validates patchBinding/section/container index, module_exit leaves the mode, and set_parameters validates/coalesces binding+section+module+parameter targets with actual descriptor ranges. status.edit contains only selected-module parameters and uses existing value formatters. Normal7-bit controls only; custom/morph/unsupported ranges excluded. `python3 scripts/check_official_plugin.py --module-edit` tests real protocol edits and controller/WebKit paging; it requires this project's controller-launcher lock. No native Edit-button/cursor synchronization or VST modification.

Passive MIDI activity: `MidiActivity.h` subscribes to the S49 Mk2 performance MIDI input for note-event animation only; it never forwards or remaps messages. `status.midiActivity` supplies connection/source, note-on serial, velocity and Unix timestamp. DAW-internal clips are not visible through this tap. `g1midiactivitycheck` checks callbacks/source filtering without opening devices; its `--sources` mode only lists input names.

Exact parameter labels: installer reads the user's fingerprint-verified rack OS 3.03, decodes its static name-table initialization, and creates only a private `parameter-names.json` metadata cache in Application Support. `MK3_PARAMETER_NAMES` points the external adapter to it. Names match the official VST's OS short-name spellings/case, with descriptor/range validation and metadata fallback for unsupported inputs. No ROM or generated cache is bundled in this repository or staged runtime; no VST code or memory is changed.

Activity telemetry also includes activeNotes, heldVelocity/heldPeakNotes, and releaseTimestampMs/releaseVelocity/releaseNotes. Held summaries respect per-channel note-off, zero-velocity note-on, sustain and reset messages; local callback state never changes MIDI routing or audio.
