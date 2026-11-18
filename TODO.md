# Deferred standalone keyboard split

- [ ] Implement the documented S49 KEY MODE two-zone split in future standalone software; not part of current VST work. See [design and acceptance checks](docs/standalone-keyboard-split.md).

# Active control surfaces / official plugin

- [x] Make Maschine MK3 selectable in the official VST background service with its own launcher and panel bindings.
- [x] Map Maschine arrows to knob pages, SHIFT + arrows to presets and ARRANGER to Sequencers.
- [x] Use dim idle lamps and bright active or held lamps on Maschine MK3, with MACRO disabled and dark.
- [ ] Physically verify Maschine MK3 controls and display feedback with the official VST.

- [x] Benchmark and remove redundant LED snapshots and playhead grid rebuilds; retain edit/navigation behavior. See docs/bridge-performance.md.

- [x] Gate PATTERN by actual patch sequencers; strict Edit Sequence lock; reference header; error-only screen text; matching Module Editor list.
- [x] Correlate isolated physical wheel UP/DOWN with all three MIDI sources: no messages during the presses; keybed and DAW routing unchanged.

- [x] Show step circles and real red playheads for NoteSeqA/B, CtrlSeq and EventSeq from the editor meter stream; verified on clocked official-VST fixtures.
- [x] Add PATTERN sequencer selection and display5 Sequence Edit for NoteSeqA/B, CtrlSeq and two-row EventSeq; wheel tilt/rotate/press, clean mode exits and CLEAR integration.
- [x] Capture PATTERN input/LED and PRESET UP/DOWN lamp positions on physical S49 Mk2.
- [ ] Physically review sequencer navigation, edits, indicators and screen responsiveness.

- [x] Keep CLEAR off/inactive for unchanged slots and switch it off after confirmed restoration.
- [x] Optimize CLEAR to write changed parameters only, including pending edits; clean-slot CLEAR sends zero writes.
- [x] CLEAR restores per-slot load-time parameter baselines through paced writes and confirmed readback, covering knobs and Module Editor without patch reload.
- [ ] Physically verify CLEAR lamp/input and restore feel on S49 Mk2.

- [x] Correct Display knobs to Theme1/Blobs2/Glitch3/Colour4; disable glitches in Colour and limit theme jumps to Light/Dark.
- [x] Add five-minute idle blob screensaver, sixty-minute intensity ramp and surface/MIDI wake with saved settings restored.
- [ ] Physically review screensaver fade/wake and sustained-idle behaviour.

- [x] Replace glitch switch with saved intensity(default50%), liveDisplay preview, >90% temporarythemejumps and100% three-frame ALERT.

- [x] Add saved Blobs/Glitches On-Off controls on Display knobs3/4, both on by default.

- [x] Sustain animation through held notes/pedal, release on final note-off, and add occasional synchronized blob glitches.

- [x] Add dual-Home organic halftone animation with idle breath and passive S49 note/velocity activity; stop elsewhere, cap frame rate.
- [ ] Physically review animation response and screen performance while playing/turning knobs.

- [x] Restore Settings on TRACK+display7 and add saved Light(default)/Dark/Colour themes on Display knob1.

- [x] Colour TRACK slot lamps and add TRACK+display8 local colour-intensity page with knob1 and saved preference.

- [x] Apply supplied peach Home/Browser screen language and infer matching module/settings/mixer styling; preserve mappings and live state.
- [x] Apply A red, B green, C orange, D yellow themes.
- [ ] Physically review the new screen design and slot palette.

- [x] Make pages exclusive, Browser close to Home, TRACK return Home while retaining slot modifier; remove duplicate browser name on screen2. Focused state/WebKit checks pass.

- [x] Bind physical MIXER and add real A–D patch Output levels on knobs1–4, with output ambiguity guards and independent native readback.
- [x] Recover normal knobs and slot switching after Mixer operations; protected official-VST regression includes inactive-slot replacement and normal knob edits afterward.
- [x] Show A–D on left, no meters or ambiguous-output level options; keep Nord-master section right.
- [x] Simplify Mixer to knob controls only, remove Master display, and bound/preempt stalled inactive readbacks.
- [ ] Physically compare knob responsiveness after readback-priority fix. Nord master control deferred by explicit user choice to retain the unmodified official VST; no DAW substitute.

- [x] Record keyboard-only development and decoupled key MIDI.
- [x] Download/verify official G1-Emu alpha.30 release; install unmodified G1-Emu and G1-Emu FX VST3 alongside existing bundles.
- [x] Replace the custom-plugin dependency with an external adapter using official DirectLink/PC Port and unchanged NME protocol/model code.
- [x] Verify official-VST patch upload/download, real knob mapping/confirmed readback, voice counts and stale-target rejection; shared banks unchanged.
- [x] Install one background S49 service with plugin discovery, stable/preferred instance selection, duplicate suppression and owned-child cleanup.
- [x] Verify the installed launchd service against the official VST: S49 connected/control ACTIVE, both screen counters advancing, plugin-close return to idle. Keys/NI services remain independent.
- [x] Remove S49 top5–8 slot bindings; close Browser only after a confirmed successful load, retaining it on failures.
- [x] Dim available mapped button lamps and brighten active source choices using NI Dim/Bright encoding; clear former slot lamps.
- [x] Fix sustained-turn readback starvation: preserve the first 40ms deadline and prioritize due reads. Protected official-VST regression:144 writes,72 confirmed changes while moving, max gap81ms, shared flash unchanged.
- [x] Carry standalone direct parameter framing and persistent native event loop into external adapter; first readback16ms, S49 capture16ms. Protected same-probe comparison: median confirmation60→32ms/max87→36ms; continuous reversal test passes.
- [x] Correct reported responsiveness regression: restore33ms S49 capture/NME sender, prioritize writes over in-flight section reads, bound read starts to40ms, clear stale waiting notices. Actual Defunkt sweep/status tests pass.
- [x] Reproduce4s write starvation with dropped replies and fix global reply-lock coupling:136 actual writes/4s vs1 before, maxgap59ms; recover final confirmed value; shared bank unchanged. Remove automatic2s settings poll.
- [x] Serialize Files/preset upload after adapter-owned read completion, discard old read state, preflight unsupported modules, retry startup settings without global lock, and record prolonged CONTROL_WAIT diagnostics.24 mixed edit/load checks pass, max493ms; physical issue remains open.
- [x] After user confirmed sound is responsive, remove direction-filtered confirmed-value display cache. Show latest actual engine readback during pending/reversing turns; preserve input accumulation. Two focused tests/release pass; no transport/capture tuning.
- [x] Disable hidden WebKit background throttling on supported macOS; log state-script completion delays >100ms. Hidden real-renderer five-update check16–21ms and release build pass; long-idle/physical acceptance remains open.
- [x] Add/enable requested ten-minute knob→target→readback→JS→capture→host-transfer pipeline trace, sequence IDs/content hashes, session log history and read-only trace_keyboard.py summary.
- [x] Inspect/preserve reproduced trace: software stages/host transfers quick, no measured multi-second active-stage stall. Add S49 service full-frame delivery trial and full/bytes trace fields; unchanged frames suppressed.
- [x] After user confirmed screen-only lag, restore partial updates (full-frame trial off) and replace continuous knob-activity captures with one-shot cleanup. Curve/WebKit/activity/build checks pass; preserve trace for comparison.
- [x] Add immediate marked encoder feedback and native-formatted acceptance previews; retain separate confirmed values and stale-preview cleanup.
- [ ] Compare physical graphics response after restored partial transfers and reduced activity captures; NI service acceptance does not establish LCD scanout.
- [ ] Inspect the user's reproduced screen delay using pipeline trace; physical LCD scanout is not observable from host-transfer completion.
- [x] Trace native UI Edit→OS mode/module editing and document official PC Port boundary (module-edit-mode.md); no native Edit-button/current-module query exposed by our connection.
- [x] Show PAGE1/3–3/3 on the Nord Modular screen; preserve voice indicator and invalidate identical-page rendering.
- [x] Wire Module Edit on screen button5 with real module catalogue, validated/coalesced parameter writes, module navigation and dynamic pages; preserve assigned knob maps.
- [x] Add confirmed-parameter ADSR/Mod-Env and Filter A–F diagrams on physical right; preserve knob tiles, support off-page graph updates, clear unsupported/browser/exit views. Focused math/data/hidden-WebKit checks and screenshot review pass.
- [x] Capture actual SHIFT/TRACK press/release, bind Function and Track+display1–4 slots; independent page lamps and pressed-state brightness.
- [x] Redesign envelope/filter graphs, add base oscillator waveforms and remove redundant Module Edit tile labels.
- [x] Capture ordered physical inputs and calibrate display indexed-white LEDs, SHIFT/TRACK/BROWSER/PLUG-IN/page lamps in Encdr descriptor.
- [x] Fix TRACK+1 pending reversal and space real slot writes; protected official-VST B/A,C/A,D/A confirmation passes. Reserve native Shift+PLUG-IN MIDI-mode chord.
- [x] Make Track lamps follow focus rather than pinned slots, retain confirmed screen slot during loading and fetch uncached active-slot metadata. Unit/WebKit/real official-VST checks pass on S49 Mk2 profile.
- [ ] Physically accept normal Shift/Track operation after feedback fix, compare dim/bright appearance and locate PRESET lamp outputs.
- [ ] Physically verify button5 entry/exit, module parameter feel and right-screen diagrams.
- [ ] Assess physical screen response after this update; Browser lamp is now physically calibrated. NI ownership handovers still pause surface output.
- [ ] User acceptance in the DAW: rescan/load official G1-Emu, check physical knob feel and browsing, then close/reopen the plugin.
- [x] Capture/map BROWSER and verify preset/page-arrow routing. Remove the incorrect dial-to-Function binding; S49 SHIFT stays unassigned.
- [x] Audit NI/Encdr control roles; remove SETUP/PLUG-IN synth mappings, allow only assigned controls, and pause the surface for detected native modes.
- [ ] Physically verify SETUP/MIDI → INSTANCE → Encdr handover and absence of duplicate DAW panel actions.
- [x] Fix the not-ready status reconnect loop and command-rejection disconnects; request banks lazily and prioritize parameter edits.
- [ ] Verify the remaining S49 button bits/lamps and morph-knob behavior; master/gain remain official-plugin/DAW controls.
- [ ] Add explicit on-keyboard instance switching and concurrent editor arbitration only if requested; current policy is first/preferred available plugin.

The custom/branded plugin and standalone tasks below are historical.

# Roadmap

Scope: [Mac emulator + Maschine control surface](docs/scope.md). G1-Emu owns audio, patch data and full editing. No custom editor. Completed baseline hardware checks remain accepted. [Current evidence](docs/CONTEXT.md) distinguishes automated coverage from physical acceptance.

## Implemented and automatically verified

- [x] Pinned Apple Silicon G1-Emu build with the user's external rack OS3.03 ROM; actual DSP/audio and parameter changes verified.
- [x] Eighteen assigned panel controls across three pages, actual labels/units/values, slot selection and allocated(requested) voices.
- [x] Agreed Groups A–D, Macro, Browser, < / >, Shift, Settings, Mixer and state-driven LED mappings.
- [x] Browser local files/subfolders/parent navigation, Files/Banks, refresh, active-slot loading and separate highlight/click activation.
- [x] Firmware-confirmed file and bank outcomes; pending labels preserve the previous patch; conflicting operations are blocked.
- [x] Real-ROM coverage of 18 user patch copies, including no assignments; slot isolation, long names, empty-folder refresh, repeated invalid-file rejection/retry and source-file preservation.
- [x] Stable fading list motion during rapid reversals; both knob strips hidden in Browser; activity bold without glow; two-line module/parameter labels.
- [x] Corrected physical presentation routing independently of the user-confirmed encoder mapping. Latest orientation awaits acceptance below.
- [x] Relative encoder coalescing and direct parameter messages, stale-binding invalidation, small-range sensitivity and Function safety. Actual readback/audio checked.
- [x] Page1/page3 assignments, empty page2/final inactive controls, label geometry and independent page/browser/slot state checked.
- [x] Combined launcher, compatible-instance attachment, duplicate/port errors, external controller preferences and double-click project entry point.
- [x] Clean-exit browser folder/source/selection restoration; missing-folder recovery and corrupt-file preservation; natural numeric sorting and large-library bounds.
- [x] Bounded engine-thread native requests, lifetime/client-generation guards and stopped-engine handling.
- [x] Persistent HID API owner thread and real repeated-enumeration regression.
- [x] Asynchronous bounded snapshots, unchanged-frame skipping, browser/header full keyframes and fast bottom partial updates.
- [x] Missing-header capture rejection and JavaScript/capture failure reload/latest-state replay; injected fault checks passed.
- [x] Sustained bridge traffic, client reconnect/Function release, isolated emulator restart and no stale command replay coverage.
- [x] Rotating controller logs and periodic engine/render health summaries; documented restart and shutdown behavior.
- [x] Local app/source packaging implemented with matching release controller/native bridge, version/provenance, dependency licenses and external ROM/bank/library data.
- [x] Final local package signature, binary/source digests, hidden real-engine launcher smoke, isolated offline Cargo resolution and native CMake configure.
- [x] README, architecture, controls, integration guide and concise handoff updated; old handoff chronology archived.

The intermittent freeze has not been conclusively reproduced. The older HID/run-loop crash and additional thread/render failure paths have been addressed; this is not a guarantee that every freeze is eliminated.

## MIDI and fast-turn follow-up

- [x] Halve patch and Settings/Mixer encoder sensitivity, preserving fractional steps and browser navigation.
- [x] Coalesce latest knob/control targets with a 16ms write interval and pace status requests.
- [x] Regression: 10,001 encoder targets produce bounded native batches and retain the final confirmed target.
- [x] Physical Pad Mode uses drum notes 36–51; Keyboard uses chromatic 48–63. Scales remain for later.
- [x] Strike velocity, duplicate-pressure suppression, confirmed pad/mode lights and actual selected-slot MIDI input.
- [x] Original-channel note-off; mode-switch/STOP/disconnect/client-loss cleanup and panic bypass for queue pressure.
- [x] Real-ROM MIDI gate/pitch audio, mode/velocity/release and rapid-encoder checks with an isolated project-owned patch.
- [ ] Assess pad velocity/feel and the two physical mode buttons with the updated build.

## Responsiveness from MD-MM comparison

- [x] Inspect the requested MD-MM source and document native input/engine/render separation.
- [x] Dispatch hardware input independently of WebKit while preserving ownership and release edges.
- [x] Service native control sockets independently of the JUCE UI thread; keep synth changes on the engine queue.
- [x] Remove per-detent gain-file writes from the engine thread; share completed catalogue snapshots and reduce redundant polling.
- [x] Verify hardware dispatch without render calls and native requests during a deliberate 600ms UI stall.
- [ ] Assess physical knob/pad response and panel latency after restarting the matching pair. If display lag remains, measure the WebKit capture/transfer stage against a native renderer before changing the visual design.

## Captured screen/LED freeze

- [x] Sample the live frozen controller while leaving the synth running; identify main-loop re-entry and idle output workers.
- [x] Use one persistent AppKit loop, precreated views, per-tick autorelease pools and asynchronous preview capture.
- [x] Verify UI/fault recovery, MIDI/audio, page capture and clean shutdown; restart only the controller and observe continuing live output beyond the earlier freeze interval.
- [ ] Continue normal physical use to establish whether other freeze paths remain; the captured failure path is addressed.

## Physical acceptance when the user returns

See [test on return](docs/test-on-return.md). These are acceptance checks, not requests to repeat the already-passed generic Encdr baseline.

- [ ] Save unsaved G1 work and restart normally to load the updated native bridge; exercise the combined launcher/double-click app.
- [ ] Confirm first-screen patch/browser and second-screen Nord Modular, keeping encoders aligned.
- [ ] Assess encoder feel, bold feedback and rapid bidirectional preset scrolling on actual panels.
- [ ] Use local file Browser, subfolder/parent, Files/Banks and refresh, including long names/errors; return to parameter control.
- [ ] Use Settings/Mixer/held Function and confirm physical light states.
- [ ] Reconnect MK3 during a real session with latest changes; confirm retained browser/page state and fresh engine values. Earlier generic reconnect passed.
- [ ] Choose representative patches with the user and useful first-page assignments in the existing editor when that workflow is resumed.

## DAW plugin versions

- [x] Build upstream-based Apple Silicon VST3 and AU instrument/FX variants with distinct local identities.
- [x] Add per-instance Maschine bridges, safe host lifecycle/engine access, actual parameters/MIDI and DAW state recall.
- [x] Add plugin-only launcher/discovery and refuse ambiguous instance selection; expose the port in native plugin Settings.
- [x] Verify actual VST3 modules: two instances, host parameters, MIDI audio, pause/resume, state restore and no stale targets; verify FX audio input.
- [x] Install newly named bundles alongside existing plugins; both AUs pass Apple's validator.
- [ ] User acceptance in their own DAW, including the eight physical knobs/pads and their preferred host automation workflow.
- [ ] Define DAW clip recording for direct Maschine pad notes as a separate MIDI-routing feature.

## On hold or deliberately deferred

- Existing-editor synchronization: on hold following the user's objection to opening other software. Identify/connect/change/reassign/reopen checks are not claimed completed. Do not launch it automatically.
- Favorites/recent index: leave out until actual browsing establishes a need; no second index for now.
- Bank Store: leave out until explicitly requested and destination/overwrite interaction is agreed. File loading remains non-destructive to banks.
- Scales for Keyboard mode, octave controls, sustain, note repeat and further performance key assignments: define later with the user.
- Arbitrary unassigned parameters, additional morph mappings, G2 and Raspberry Pi/Linux: separate scope after G1 acceptance.
- NI instance handover, initial no-device startup, preview-window close and explicit color/border QA: recorded hardware edges, non-blocking unless touching those paths.

Sources/pins/licenses: [integration](integrations/g1-emu/README.md), [architecture](docs/architecture.md), [controls](docs/key-assignments.md), [context](docs/CONTEXT.md).

## Komplete Kontrol S49 Mk2

- [x] Identify the connected keyboard and available performance/DAW MIDI ports.
- [x] Add an explicit alternative controller profile, launcher, shared G1 screens and panel mapping.
- [x] Capture native reports and correct eight knob positions/decoding; verify captured input, ring crossing, ownership and navigation.
- [x] Verify live NI-service ownership and completed screen transfers; document upstream standalone/DAW keyboard MIDI routes.
- [x] User confirmed corrected knob 5 feedback in both directions; normal key MIDI already works and must remain decoupled. No automatic key routing/interception added.
- [ ] Verify remaining keyboard buttons/orientation; BROWSER is captured; the mistaken SHIFT/dial bit is ignored. Verify remaining lamps without assigning native controls.
- [ ] Define Light Guide/scales/chord/arp and simultaneous Maschine+keyboard surface ownership separately if needed.

- [x] Add a dedicated S49 plugin launcher and distinguish it from standalone/standard Maschine launchers; observe live VST browsing and advancing S49 frame counters after NI focus returns.
