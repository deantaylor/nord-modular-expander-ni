> Current workflow: official unmodified G1-Emu VST plus the automatic external S49 bridge. See [keyboard guide](../../docs/komplete-keyboard.md). The earlier custom-plugin/Maschine implementation below is historical.

# G1-Emu integration

The Mac runs the existing G1-Emu standalone and its CoreAudio backend. This repository supplies its optional controller socket and the Rust Maschine adapter. Full patch editing remains with the existing editor. No custom editor or replacement synth is involved.

## Build and run

The local Apple Silicon app has been built into `build/g1-emu/G1-Emu.app`. To reproduce it, install Rust, CMake and Xcode Command Line Tools, then run:

```sh
python3 scripts/build_g1.py
cargo build --locked
```

The build script fetches pinned open-source dependencies into ignored `build/g1-work/`, applies `standalone.patch`, copies the three integration source files, builds and runs the ACK regression, and copies the app/helper to `build/g1-emu/`. It never fetches a ROM. Existing source checkouts can be supplied with `--source-dir`, `--gearmulator-dir`, `--dsp-dir`, `--juce-dir` and `--build-dir`; a different revision is rejected, never reset.

Place your compatible local rack OS 3.03 ROM at `roms/NORD-MODULAR-RACK-VER-3.03.BIN`, or pass `--rom /absolute/path` below. Firmware, bank files, recordings and local builds are ignored by Git.

To run the historical standalone integration from this checkout, use the combined launcher:

```sh
python3 scripts/run_machine.py
```

The launcher builds the Rust controller if necessary, attaches to a compatible running G1-Emu or starts the local app, waits for OS/bridge readiness, then starts Maschine control. A duplicate launcher, occupied/incompatible bridge or old bridge produces an actionable error. If you already started a controller manually, quit it before using the launcher: the bridge accepts one controller connection.

```sh
python3 scripts/run_machine.py --patch-dir "/absolute/path/to/library"
```

The selected patch folder, bridge port, local ROM path and bank path are remembered in ignored `g1-state/controller-preferences.json`. This stores controller launch preferences, never patch contents. Invalid preferences are reported and preserved. Relative paths supplied on the command line are resolved when saved. An attached emulator keeps its current ROM/bank; saved ROM/bank paths apply when starting a new one. Add `--port`, `--rom` or `--flash` to choose another value, `--demo` to seed a NEW dedicated bank, `--visible` for hardware previews, or `--mock` for desktop screens controlling the real emulator without USB. `--no-build` uses the existing debug binary. These modes are not remembered.

Type `quit`, close a controller preview, or press Ctrl+C to stop the controller. G1-Emu remains open, whether started or attached; close its window normally when finished to save its banks. NI agents remain running. If G1-Emu cannot become ready, check its existing Settings/ROM prompts and `g1-state/launcher-emulator.log`. Audio output is selected in G1-Emu's Settings; the launcher does not replace that UI. A Finder double-click uses the last saved preferences.

The combined launcher is now available; the earlier separate commands are still useful for development:

In one terminal:

```sh
python3 scripts/run_g1.py --demo
```

This opens the existing G1-Emu app and enables the controller bridge on `127.0.0.1:47410`. On first use, `--demo` creates a dedicated `g1-state/controller-flash.bin`, uploads the project-owned `patches/MK3-Bridge.pch` through the actual OS, and stores it at bank 1 / patch 1. An existing bank file is never reseeded. Close any other G1-Emu standalone first: upstream permits one normal app instance. The first-run notice and Settings are G1-Emu's existing UI. Choose your audio output there if needed. Default upstream output gain is +36 dB; the automated helper uses 0 dB. The ROM and isolated bank file are passed explicitly; normal upstream preferences still use `~/.local/share/Animatek/G1-Emu/settings.conf`.

In another terminal:

```sh
cargo run --locked -- --g1
```

Keep NI agents running. Use `--visible` for desktop screen previews. `--mock --g1` opens the controller screens without USB but still controls the real emulator; plain `--mock` uses developer fixtures. Exit the controller with `quit` or Ctrl+C; the emulator remains running. Close G1-Emu normally to save its bank file. Physical STOP is currently an exit control only in hardware-test mode.

## Controls

The agreed mapping is in [key assignments](../../docs/key-assignments.md).

| Control | Action |
|---|---|
| Screen encoders 1–4 / 5–8 | Bottom row on left / right display |
| Groups A–D | Select G1 slots A–D |
| Macro | Cycle knob pages 1–8, 9–16, 17–18 |
| Browser | Toggle file/bank browser |
| Main encoder turn / press | Highlight / open or load in Browser |
| Physical < / > | Load previous / next preset in the selected source |
| Shift, held | Forward the real emulator Function/Shift input |
| Settings | Toggle actual emulator settings on the hardware |
| Mixer | Toggle master-volume page |

Browser Top 1/2/3/4 selects Files / Banks / Refresh / Load. Four-way Left goes up a folder in Files; Up/Down changes the highlight. Files default to the project's `patches/`, or the library saved by the combined launcher. `--patch-dir` selects another folder. Folder navigation, refresh and file selection stay inside that library; symlinks are skipped. Loading does not rewrite `.pch` files or store over banks. The display reports Loading then firmware-confirmed Loaded/failure. `<` / `>` immediately load adjacent `.pch` entries in the current folder, skipping folders, or adjacent nonempty bank patches; there is no wrap at boundaries.

Settings Top 1–3 selects its actual settings pages; Top 4 opens the existing **Emulator Settings** card on the Mac for complete ROM/audio/MIDI-device configuration. The hardware settings pages edit real host gain, clock, tune, global sync, four MIDI channels, velocity, local/keyboard/knob modes, program-change and pedal/LED settings through upstream APIs. Observed values come from readback; synth persistence follows the upstream OS. This is a settings view, not a patch editor.

Mixer shows the real master-volume position; encoder 1 adjusts it, and encoder 2 adjusts host output gain. The remaining controls are unassigned. Parameter, Settings and Mixer views put four flat encoder columns at each display’s bottom edge. Browser expands the left list to full height, retaining the right-hand controls.

Group lights follow confirmed active/selected slots. Browser/Settings/Mixer/Macro lights follow the viewed mode; Shift follows confirmed Function state. Selected tabs and available actions light appropriately; unused/unavailable buttons are off. NI ownership rules and neutral exit cleanup remain unchanged. Function is released on button-up, handover or connection loss, with queue-pressure retry.

The supplied patch assigns frequency to encoder 1 and mixer level to encoder 2. Other patches control their existing G1 panel assignments; use the existing editor to define those. All 18 panel assignments are exposed, with unused tiles labelled. Full arbitrary module-parameter mapping remains deferred. Groups E–H, Top 5–8, Play/Stop and pads have no G1 action. Macro replaces the earlier Top/Group page shortcuts; Groups A–D replace the earlier Top slot shortcuts.

Restart **both** G1-Emu and the controller after updating this bridge. The launcher rejects an older bridge missing the actual configuration interface. Existing emulator-only and controller-only commands remain available. Type `load-patch /absolute/path/file.pch` or `refresh-presets` in the controller terminal for development.

## Bridge and provenance

Pinned sources:

- [G1-Emu](https://github.com/animatek/G1-Emu): `efd178d8a6e13a38c74b5cce02853fc7dccbf5c1`
- [gearmulator-md-mm](https://github.com/joelanders/gearmulator-md-mm): `ab69ad003f1268d1ef373fbb5a47340269b6dd47` (alpha.13 dependency)
- [dsp56300](https://github.com/dsp56300/dsp56300): `8b731966f9792789adc39fe180a8cf68508f816a`, pinned asmjit submodule
- [JUCE](https://github.com/juce-framework/JUCE): `29396c22c93392d6738e021b83196283d6e4d850` (8.0.12)

G1-Emu's stock DirectLink is a raw PC Port MIDI transport, one editor connection, not a parameter/bank JSON API. The optional standalone bridge uses a separate localhost socket and leaves DirectLink available. It reuses `g1::KnobMap::read`, `g1::formatValue`, editor parameter messages through PresetsLink, panel button inputs (ADC fallback until patch identity is known), `PresetsLink::{readBank,bank,load,upload}` and existing `preparePch`/NME serialization. It does not write patch parameters directly into OS RAM. Initial support is Regular/Expanded rack G1, not Micro or G2; the launcher chooses Regular.

The upstream patch also fixes a reproduced startup race: a completed `SynthSettingsLink` read kept filtering ACKs for 300 ms, swallowing a subsequent single-packet patch upload ACK. Completed replies now release ownership immediately; abandoned reads retain the late-reply filter. `SettingsReplyCheck.cpp` tests both cases. This is a local patch, not an upstream compatibility guarantee.

The explicit `G1_MK3_PORT` opt-in enables protocol 1, newline JSON, one controller connection. Greeting identifies service/version/instance; requests carry `id` and `method`, responses echo the ID with `result` or `error`. Methods: `status`, `presets`, `set_knobs`, `select_slot`, `load_patch`, `load_file`. Knob writes carry the observed assignment binding and validated raw range. Validation covers the whole batch before writes. Accepted mutations are requests; confirmed parameter/LCD state comes from subsequent OS reads. Transport is bounded, uses nonblocking writes, times out and reconnects without replaying queued commands. Unavailable/booting states display no invented patch values. This is for trusted software on the same Mac; it is not a remote network service.

The bridge sources and upstream modification are GPL-3.0-or-later. Preserve G1-Emu/dependency licenses and full corresponding source if distributing binaries; the local build keeps its source checkouts and adds license/provenance beside the app. No firmware is part of those sources.

## Verification

```sh
python3 -B scripts/check_g1.py
```

Requires the built debug controller and logged-in macOS desktop. It runs the real ROM/DSP/CoreAudio host against a fresh isolated flash file, stores the permitted patch, rejects a stale knob binding, changes two actual knobs, confirms OS feedback, checks rendered controller text, opens Browser, scrolls to a local `.pch` variation, waits for upload success and verifies its different actual knob values, checks file-list layout, switches to Banks, reloads the stored patch and verifies that exiting the controller leaves the emulator running. Its 24-bit DSP recording is checked for E4/E5 frequency and changed AC amplitude, removing the hardware DAC DC offset before analysis.

Passed on this Mac: 27 Rust tests, strict Clippy, the settings-reply regression, upstream DSP check and real integration. Recorded evidence: `g1-state/verification-1794399367030120000/` (ignored). Measured about 329.7/659.3 Hz and 4.20× level ratio. CoreAudio opened Apartment Output at 44.1 kHz. The installed GUI standalone was also launched through `run_g1.py --demo`: its bank survived restart, real controller events changed both assignments, and controller bank reload restored them. Native-window visual inspection was unavailable because macOS computer-use permission was not granted. The user subsequently confirmed physical Maschine frequency/level control with sound/value feedback and browsing/loading their patches. These are user-reported passes; exact patch names were not recorded. Simultaneous external-editor synchronization remains unverified. G2 and full parameter enumeration remain deferred.

Browser-button file loading passed the automated real-ROM/DSP and full six-row rendered-list check; evidence: `g1-state/verification-1794401671124544000/` (ignored). All 27 Rust tests, strict Clippy, formatting, build and patch applicability passed. Earlier physical bank browsing/control passes remain accepted; the new Browser file flow has not yet been physically exercised.


Launcher verification: `python3 -B scripts/test_launcher.py` covers launch sequencing, missing ROM, malformed preferences, duplicate locking, framing, wrong-service ports and outdated bridges. `python3 -B scripts/check_launcher.py` uses an isolated real ROM/CoreAudio host and desktop controller to verify attachment, remembered folder/port, duplicate rejection, terminal quit and SIGTERM cleanup while preserving the emulator. Real evidence: `g1-state/launcher-check-1794402755504926000/`. It does not close or alter the user's current emulator session. The double-click entry point has executable permissions and passed zsh syntax validation; Finder interaction is not separately claimed.

Next mapping decisions are recorded in [key assignments](../../docs/key-assignments.md). Current mappings stay in place until the user defines the desired performance actions.


Agreed mapping verification now includes: real ROM Function gating/release (patch knobs ignored while the modifier is held), master ADC changes, actual synth tempo readback, Group A–D slot changes, Macro pages, previous/next file loading, mode/slot LED state, held-Shift handover release, and four bottom tiles per screen with encoder numbers 1–4 / 5–8 and checked bounds. The standalone's native Settings callback reuses Panel's existing Settings card. Native-card visual inspection and the new physical light/layout checks are not claimed by headless integration tests.

Final agreed-mapping evidence: `g1-state/verification-1794404290766046000/` includes real Settings/Mixer values checked in rendered WebKit text, plus Function release after socket loss and the other control checks above. Launcher compatibility also passed at `g1-state/launcher-check-1794404205975078000/`. New physical brightness/alignment and native-card visual use remain separate from these automated passes.


Patch-name/display fix: G1's LCD uses custom character codes for lowercase descenders; the bridge now decodes g/j/p/q/y and arrows instead of blanking them. Restart G1-Emu to load this native-bridge update. The macOS controller now captures both screens asynchronously so snapshot waits do not block input processing. `MK3_SCREEN_TIMING=1` enables diagnostic completion timings. A bounded hardware-only page check is available with `python3 -B scripts/check_screen_updates.py` after quitting the existing controller; it changes only controller pages, never patches/settings. Physical evidence: `g1-state/screen-timing-1794405471207652000/`, capture maxima about36/52ms, controller-loop median14ms. This is not a panel scanout measurement or a before/after speedup claim.


Inter visual design: the live screens now follow the supplied dark, minimal references, with slot/patch and Nord Modular headings, light large numbers and separate small units. A bundled variable Inter font loads offline. Browser scrolling uses 180ms eased row positions, retargets the same row elements, and keeps capture active through the transition. A bounded native capture burst supports offscreen hardware views; unchanged output on the other screen is skipped. Reduced motion is respected. Error/loading notices remain available and selection remains separate from patch loading.

The real check verifies the embedded font is loaded, four-column bounds, preserved values, and an intermediate list position followed by a settled position. It captures normal/browser/settings/mixer RGBA previews; `python3 -B scripts/render_previews.py g1-state/<verification-directory>` produces PNGs. New evidence: `g1-state/verification-1794407830302803000/`. Physical scrolling/capture check: `g1-state/screen-timing-1794407875832862000/`, 67 left/10 right captures during the same page/highlight sequence; final snapshot timings23.2/29.5ms, input-loop median14.2ms. First/cold captures reached about137ms; this is not an end-to-end scanout measurement.

The top-right voice indicator is **allocated (requested)** for the active slot. Allocated counts come from the real LCD slot allocation; requested counts come from the patch header through PresetsLink. Both require the updated native build; restart G1-Emu and the controller after this update. Browser scrolling moves the selected candidate into the main name position; clicking loads it. The selected row moves with the list into the name position; neighboring rows use progressively lower opacity. Closing Browser restores the loaded name. Hardware keeps the confirmed encoder mapping, swapping header/browser presentation roles independently of encoder slices. Control commands wake the worker immediately; state refresh is 16ms. Native 2× Retina captures average four pixels per output pixel for smoother Inter text.

Encoder response follow-up: MK3 movement is relative; Rust accumulates/coalesces bounded parameter targets. The native bridge now uses verified editor `$13/$40` parameter messages with the current slot patch ID, bypassing the G1 pot scanner. ADC input remains a fallback while patch identity is unknown. Measured confirmation median fell from95.5ms to11.6ms in comparable isolated probes; a mixed safety run measured12.4ms with one118.6ms outlier. See [encoder protocol and evidence](../../docs/encoder-response.md). The launcher requires `encoderInput=editor-parameters` and explains when the running native app needs restarting. It never closes the user's synth for this upgrade.

Browser hides knob controls on both screens. Activity uses bold values (weight400), with no glow. Hidden native-frame tests include rapid reversals and pixel-band checks. MK3 service browser/header animation sends self-contained upper-screen keyframes; bottom value changes retain small partial transfers and identical frames are skipped. Full-frame transfer costs about30ms versus about4ms for small partials on this setup, so all-frame full blitting is intentionally avoided.


## Recovery and local delivery

Native bridge requests now execute through a bounded engine-thread queue; GUI/socket handling remains on the message thread. Client generation/lifetime guards prevent obsolete work after close. Bank loads expose an operation serial and matching LoadAck outcome, just as file uploads expose firmware confirmation. New patch IDs invalidate queued knob targets and seed the direct-parameter path after load. The launcher requires these capabilities and requests a normal save/close/reopen when an older native app is still running. It never closes that app itself.

Browser position is stored separately from emulator data on clean controller exit. Missing folders fall back to the library root, corrupt preference files are retained, and natural numeric sorting supports large libraries. Module/parameter labels use separate lines; activity remains bold and both control strips hide in Browser.

Renderer timeouts, missing-header captures and JavaScript errors retain the last good output and reload/replay current state with a recovery cooldown. The HID manager has one process-lifetime owner. These changes address evidenced failure paths; the reported intermittent freeze has not been conclusively reproduced. See [current evidence](../../docs/CONTEXT.md) and [physical acceptance](../../docs/test-on-return.md).

Build the local app/source archive with `cargo build --release --locked --offline` then `python3 scripts/package_local.py`; verify using `python3 scripts/check_package.py`. App runtime data is external under `~/Library/Application Support/Modular Machine`. No proprietary ROM, user banks or patch library is included. The project launcher logs to `g1-state/controller-session.log` (2MB with two backups); packaged runtime uses its external state directory. Existing-editor synchronization is on hold at the user’s request and has not been demonstrated.


MIDI pads use the upstream standalone’s existing `EmuHost::queueMidiIn` path, the same queue used by its keyboard input. New `pad_midi` requests validate pad/slot/note/velocity and choose the selected slot’s real configured channel. Releases retain their original channel/note; `pad_panic` and bridge close release tracked notes without a global MIDI panic. Status advertises `padMidi`, `padMidiHeld` and actual `midiInBytes`. No external editor or MIDI helper service is launched. The launcher requires the new capability; save/close/reopen normally after updating. Physical Pad Mode selects drums, Keyboard selects chromatic; scales later. See [controls](../../docs/key-assignments.md).

Encoder sensitivity is 50% and write/status batches are paced at 16ms, coalescing latest knob and Settings/Mixer targets. An isolated10,001-target transport test and real-ROM MIDI/rapid-turn check are in `scripts/check_midi_pads.py` and the ignored Rust `encoder_storm_is_paced_and_latest_target_survives` test. These reduce pressure and verify recovery boundaries; they do not prove that the user’s entire intermittent crash problem had one cause.


Responsiveness follow-up: hardware input dispatch now runs independently of the WebKit loop. Native sockets are serviced on a dedicated I/O thread, with synth access retained on the bounded engine queue. `transportThread=dedicated` identifies this build. Gain preference writes coalesce on the GUI thread, with a shutdown flush, rather than blocking the engine on each detent. Explicit UI-stall and independent-dispatch checks cover these boundaries; see [comparison and measurements](../../docs/md-mm-comparison.md). The reference project was inspected as source and was not launched. WebKit snapshot/display latency remains distinct from control delivery.


## DAW plugin builds

Upstream's existing plugin targets are now available with the same controller bridge: **Modular Machine G1** instrument and **Modular Machine G1 FX**, Apple Silicon VST3/AU. Build with `python3 scripts/build_g1.py --plugins` using the directory flags above. The expanded historical `standalone.patch` covers both runtime integrations. Plugin audio comes from upstream Processor/Runner and the DAW; the adapter does not link/run a standalone EmuHost. Safe engine access is reported as `engineAccess=runner-lock`. Each instance has its own control port47420–47483 and generation-bound parameter identity.

Run `python3 scripts/run_machine.py --plugin --port PORT --no-build` to attach to a historical locally built plugin. A single available instance is selected automatically. For multiple instances, specify the port shown in the plugin's native Settings. Controller preferences are separate from standalone launch preferences. See the [plugin guide](../../docs/plugin-guide.md) for the current official VST workflow.
