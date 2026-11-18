> Current workflow: official unmodified G1-Emu VST plus the automatic external S49 bridge. See [keyboard guide](komplete-keyboard.md). The earlier custom-plugin/Maschine implementation below is historical.

# Current architecture

G1-Emu owns the ROM, DSP, audio, patch storage and existing editor transport. The Rust application supplies Maschine controls, displays and browser navigation. `src/session.rs` selects the real `src/g1.rs` adapter or developer fixtures; fixtures are not production patch storage or another editor.

## Control and engine boundaries

Hardware events are dispatched independently of WebKit by `src/input_dispatch.rs`, with a device authorization gate and shared G1 client. Relative MK3 input is accumulated/coalesced into bounded parameter targets. Each target carries current patch/assignment identity; external loads, slot changes and disconnects invalidate old targets. Native parameter frames use the current patch ID and bypass the slower ADC pot scanner. Confirmed engine readback supplies displayed values. Function is a real held panel input and released on disconnect/handover.

The localhost JSON bridge parses sockets on a dedicated I/O thread, queues bounded work onto the emulator engine thread and returns futures in request order. Client generations and lifetime guards cancel obsolete work. Settings GUI callbacks return to the message thread. Status reports running/problem state, upload/bank operation outcomes and actual assignments. File uploads and bank loads remain pending until their firmware outcome; conflicting operations are blocked. The original DirectLink editor transport remains available, but simultaneous editor UI synchronization has not been verified.

## Rendering and hardware

AppKit runs once for the session. Tao schedules controller/display ticks; views are constructed before that loop and bound to hardware on discovery. Per-frame event-loop re-entry has been removed after a live freeze sample. Each tick drains temporary native objects through an autorelease pool. Debug preview captures are asynchronous.

State dispatch coalesces at 16ms. WebKit captures asynchronously, one in flight per display, with at least 33ms between starts. Input processing does not wait for snapshots. List positions advance immediately before snapshots; rapid changes retarget existing rows rather than destroying them. Both control strips disappear in Browser; values use temporary bold emphasis. Inter is embedded offline.

Snapshot timeout, missing-header output or JavaScript errors retain the last good frame and trigger bounded HTML reload/latest-state replay. A 5s cooldown prevents reload storms. NI-service browser/header changes use full keyframes; bottom controls retain fast partial transfers. Identical frames are skipped. Actual service transfers measured about 30ms full/~4ms partial; these are not end-to-end physical latency claims.

Hardware discovery filters 17cc:1600. A process-lifetime HID manager thread owns initialization/enumeration/open across reconnects. State survives device disconnect and views retarget the current device; ownership gates input/output. Header presentation routing is independent of the user-confirmed encoder slices. Keep NI agents running. Clean shutdown lets output workers clear LEDs/show Encdr logos before invalidating connectivity.

## Files, launch and diagnostics

`src/patch_browser.rs` confines navigation to the canonical patch-library root, skips symlinks and naturally sorts names. `src/browser_state.rs` stores only controller position on clean exit; missing entries recover and corrupt preferences are preserved. File loading does not rewrite source patches or store banks.

`scripts/run_machine.py` opens/attaches to a compatible emulator, enforces a single launcher, persists controller preferences and leaves the synth running on controller exit. Controller-session logs rotate at 2MB with two backups and contain periodic engine/render health. Local packaging provides the matched release controller/native bridge and corresponding dependency sources without proprietary firmware or user data.

See [current evidence and pins](CONTEXT.md), [agreed controls](key-assignments.md), [return-session checks](test-on-return.md), [native integration](../integrations/g1-emu/README.md) and vendor PATCHES.md files. Automated checks establish their stated boundaries; physical feel and external editor synchronization remain separate.

The MD-MM comparison and latency boundaries are recorded in [md-mm-comparison.md](md-mm-comparison.md). Completed bank catalogues are shared immutably and polled every 10s; explicit refresh stays immediate. Host gain preference writes are coalesced on the GUI thread, outside the engine request path.
