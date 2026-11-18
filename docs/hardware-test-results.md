## Calibrated S49 controls and lamps — 16 November 2026

PLUG-IN toggles Module Edit; display buttons5–8 are unassigned/off. Hold TRACK + display buttons1–4 for slots A–D. SHIFT holds the existing Function modifier. Eight knobs and preset/page navigation keep their existing actions.

The corrected Encdr descriptor uses **indexed white** for display buttons1–8 at payload offsets2–9 (dim69, bright71). Monochrome lamps use SHIFT14, TRACK23, LEFT32, RIGHT33, BROWSER35 and PLUG-IN36; user confirmed white at158. Output15 is SCALE,31 STOP,34 CLEAR and37 MIXER; these stay unassigned/off. Inactive/available actions are dim, active choices/held buttons bright, unavailable actions off. Preset up/down lamp positions remain unverified and are not programmed; their input actions still work.

Ordered physical input capture and isolated lamp observations are recorded in `g1-state/s49-physical-input-map-20261116.json` and `g1-state/s49-led-calibration-20261116.json`. The inherited map had both wrong names/positions and the wrong display-button encoding; sending monochrome158 to display buttons caused green lights.

Physical checks distinguish LED output bytes from button input bits. Lamp calibration does not establish physical screen latency. Indexed white was directly verified on display3 and display8; contiguous group ordering is supported by the adjacent display2/3 observations and far-right endpoint. White dim69 is Encdr’s indexed dim state, covered by packet checks; relative physical dim brightness has not been compared yet.

# Verification — 11 November 2026

Host: macOS 26.7 (25G229), Darwin arm64; Rust 1.93.0 / Cargo 1.93.0, host aarch64-apple-darwin. Xcode developer directory: /Applications/Xcode.app/Contents/Developer. Exact Mac model was not collected. Encdr 0.8.5; encdr-view 0.8.5 plus the documented local patch.

## Commands actually executed

- `git status --short`: initially failed, no repository. `git init`: sandbox denied; authorized unrestricted retry succeeded. No commits made. Original files copied to docs/baseline before edits.
- `rustc -Vv`, `cargo -V`, `xcode-select -p`, `uname -sm`, `sw_vers`: succeeded with versions above.
- `cargo metadata --offline --format-version 1 --no-deps`: succeeded; Cargo.lock initially resolved both Encdr crates to 0.8.5 despite the 0.8.0 manifest range.
- Initial `cargo check --offline`: failed at json macro array indexing. Initial `cargo fmt --check`: formatting differences.
- Final `cargo check --offline --locked`, `cargo test --offline --locked`, `cargo clippy --offline --locked --all-targets -- -D warnings`, `cargo build --offline --locked`: passed. Three reducer tests cover individual encoders/bounds, slot/page isolation, malformed and non-finite input. Initial Clippy failures were corrected, not suppressed.
- `cargo run --offline -- --help`: succeeded.
- `cargo run --offline -- --diagnose` in sandbox: IOKit creation failed with 0xe00002be; process enumeration denied. Does not establish absence of hardware.
- Unrestricted `target/debug/modular-machine-mk3 --diagnose`: exited 0. Reported MK3 `dev:0002060017cc1600`, connected=true, control=true, screen backlog=0. NIHostIntegrationAgent was running. Backend also logged interface 5 screen claim failure 0xe00002c5 (exclusive access). No NI processes were stopped or suspended. No physical display success inferred. system_profiler returned no inventory text, so that output alone was inconclusive.
- `cargo run --offline -- --mock` in sandbox: GUI service connection errors; stuck during initialization. Its test process was terminated after SIGINT failed to leave initialization.
- Unrestricted mock with published encdr-view: failed with “Window was not created”. After local patch, unrestricted `target/debug/modular-machine-mk3 --mock` created both views, processed `turn 1 0.05` (cutoff 0.57 → 0.62), and `button top_1` (oscillator page). `quit` exited 0. A second patched mock run also exited 0 on Ctrl+C after DOM inspection and page simulation.
- Mock `inspect` after encoder simulation returned WebKit DOM text on **both** views containing `CUTOFF: 62%`. Right view displayed cutoff 62% and all eight parameter labels. Both reported width=480, height=272, and capture bridge type=function. Bottom of second knob row=229, above footer at 240. Native screenshot inspection was unavailable: the UI tool could not target the CLI executable. No claim of visual color/font QA.

## Hardware matrix

| Check | Result |
| --- | --- |
| MK3 17cc:1600 control connection | Backend reported connected/control=true |
| NI service observation | NIHostIntegrationAgent running; unchanged |
| Screen interface | Exclusive raw claim error logged; service fallback/output unverified |
| Both physical screens, color/orientation | Not tested |
| Eight real encoder mappings/sensitivity | Not tested; simulated reducer tests pass |
| Top/group buttons and 16 pads | Event logging implemented; physical input not tested |
| Pad/button LED test and physical cleanup | Implemented; not physically tested |
| Unplug/replug and ownership handover | Not tested; disconnect handling exits, restart required |
| G1 audio/editor or G2 engine | Not implemented |

## Next user-run verification

Run `cargo run --locked -- --mock`, move overlapping windows apart, and enter `turn 1 0.05`, `button top_1`, `inspect`, `quit` in the launching terminal. Then run `cargo run --locked -- --visible --led-test` with MK3 powered. Observe both screens, turn each encoder, press top/group/pad controls, and quit. Record screen photographs, console errors, whether lights clear, and the effect of the instance button. Do not stop NI services in response to the backend's generic warning without investigating the MK3 service screen route first.

## User-confirmed physical test — 11 November 2026

The user ran `ENCDR_FORCE_SERVICE=1 cargo run -- --visible` with NI services left running and replied “all working” to these three checks:

- Both physical MK3 screens show the demo.
- Encoder 1 changes cutoff.
- Top button 1 selects the oscillator page.

These are user-reported passes, not agent-observed photographs or input logs. This establishes basic display/control operation through the NI service route. All eight encoders individually, other buttons/pads, LED cleanup, colors/orientation, shutdown and reconnect still require explicit checks. G1 audio/editor integration remains absent.

The application now selects `ENCDR_FORCE_SERVICE=1` on macOS before starting threads, so `cargo run -- --visible` selects the confirmed screen route without an environment prefix. This default change has static/unit validation; the user's physical test used the explicit environment prefix before the default was added.

Subsequent shutdown evidence: the user reported a stuck terminal; the agent identified a `--mock` process (PID 58900), tried SIGTERM, then used SIGKILL after it remained running. Process absence was verified. Clean exit is therefore not universally reliable; investigate the mock event-loop/shutdown path rather than considering previous successful exits sufficient.

## Hardware-test mode increment — 11 November 2026

Changed `src/main.rs`, added `src/hardware_test.rs` and `screens/hardware-test.html`, and extended the local encdr-view macOS patch. Documentation, Cargo.lock/manifest and .gitignore updated. No commits made. Host/toolchain unchanged from above.

Commands/results:

- `cargo fmt --check`, `cargo check --offline --locked`, `cargo test --offline --locked`, `cargo clippy --offline --locked --all-targets -- -D warnings`, `cargo build --offline --locked`: passed. Six Rust tests now include diagnostic control independence/directions, rejection of terminal simulation in real sessions, ignored releases and invalid values.
- `python3 /tmp/mk3-mock-check.py` (bounded subprocess harness, unrestricted GUI access): final run passed three sessions: demo mock + quit, diagnostic mock + quit, diagnostic mock + SIGINT. All exited 0 within five seconds after exit request. Each diagnostic session simulated encoder 8 in both directions, pad 16 and top 8; DOM snapshots confirmed the intended counters, SIMULATED INPUT labels, preserved capture bridge and 480×272 screen bounds. All diagnostic cells ended above y=230. Earlier assertions caught content-vs-border measurement differences and a bottom pad-row overlap; both corrected. Those failed assertion sessions were terminated by the harness; they were not shutdown-hang reproductions.
- No MK3 hardware test mode session was launched by the agent. Previously user-confirmed basic hardware operation remains the only physical evidence. New LED colors, counters on real controls, STOP-button exit, window-close exit, cleanup and reconnect still need physical/user validation.

The event-loop wake change is a mitigation for a suspected idle wait. Repeated quit/SIGINT success does not prove the intermittent hang eliminated. Window-close handling is implemented but not exercised by the harness. No NI services were changed.

Next command: `cargo run --locked -- --hardware-test --visible`. Turn each encoder both directions; press all top/group buttons and pads; check colors, borders and physical pad row colors. Press hardware STOP to exit; confirm LEDs clear and terminal returns. Review `hardware-test-session.json` (saved before cleanup; its cleanup field deliberately does not claim success). Counter totals are event counts, not pass/fail or velocity calibration.

## Shutdown cleanup fix — 11 November 2026

User reported that after the hardware test exited, LEDs and display content remained unchanged. This is a failed cleanup observation, superseding earlier assumptions that a returned terminal meant output cleanup worked.

The saved `hardware-test-session.json` from that real session was inspected: all eight encoders had positive and negative movement, every top button had one press, and all 16 pads had positive events. Group A–H counters were zero; group coverage is still outstanding. The original report remains untouched and Git-ignored.

Root cause found in published encdr 0.8.5: DeviceHandle::disconnect/Drop and the input worker's explicit Disconnect branch set is_connected=false before output worker completion. Screen and LED teardown guards interpreted this as physical unplug and skipped their final writes. Local `vendor/encdr` patch preserves existing physical-connectivity/ownership state through output worker joins, then publishes disconnected state. Physical USB errors and ownership-loss guards remain unchanged. GPL source provenance and modifications are in vendor/encdr/PATCHES.md.

Checks on the same macOS arm64 host:

- `cargo check --offline` updated Cargo.lock to select the local encdr 0.8.5 patch; subsequent `cargo check --offline --locked`, `cargo fmt --check`, `cargo clippy --offline --locked --all-targets -- -D warnings`, `cargo build --offline --locked` passed.
- `cargo test --offline --locked`: 6 app tests passed.
- `cargo test --offline --locked -p encdr graceful_shutdown_tests`: 2 lifecycle tests passed (110 upstream tests filtered). Connected/control flags stay valid during output completion; physical disconnect or lost ownership is never reversed.
- A bounded subprocess ran `target/debug/modular-machine-mk3 --led-test` with stdin `quit` and exited 0. Initial log level did not expose transfer detail.
- A second bounded subprocess ran the same command with `RUST_LOG=info`, allowed three seconds of initialization, then sent `quit`, with a 20-second shutdown timeout. Exited 0. Logs confirmed OS HID controls and NI-service screens, successful LED clears for `button_leds` and `pad_leds`, then 261148-byte exit screensaver sends to left and right with no transfer errors. Device worker and USB event thread exited afterward.

This is actual backend/device-transfer evidence, not visual verification. NI services were unchanged. The agent did not claim a photograph or visual observation of cleared LEDs. Expected behavior: test LEDs off, test pages replaced with Encdr exit screensaver; MK3 and LCD backlights may stay powered. User visual confirmation, physical STOP exit with the patch, ownership handover, group controls, and unplug/replug remain to validate. No commits made.

### User-confirmed cleanup result

On 11 November 2026, after the patched connected-device exit test, the user confirmed: “Led's are off - screens showing encdr logo”. LED clearing and replacement of test graphics by the Encdr exit screen are therefore physically confirmed by the user. This verifies the expected neutral exit state, not power-off. Physical STOP-button exit with the patch, group A–H coverage, instance ownership handover and unplug/replug remain separate checks.

## Reconnect and ownership lifecycle increment — 11 November 2026

Changed src/main.rs, src/hardware_test.rs and the three live screen templates; added src/connection.rs and scripts/check_mock.py. Extended vendor/encdr scan filtering and service-route messaging. Updated documentation and patch notices; no commits made. Host remains macOS 26.7 arm64 with Rust/Cargo 1.93.0.

Implemented behavior: wait for initial MK3, monitor filtered background discovery, retain views/state/counters on loss, retarget screen device IDs on return, pause input/output when another app owns MK3, and redraw/restore test LEDs when control returns. Connection status/counters are included in screen snapshots and saved hardware reports. Stale events from a different old device are ignored. Hardware mode still rejects all terminal simulation.

Commands/results:

- `cargo fmt --check`, `cargo check --offline --locked`, `cargo test --offline --locked`, `cargo clippy --offline --locked --all-targets -- -D warnings`, `cargo build --offline --locked`: passed. App suite now 10 tests, including absent-device ownership, same-address replug, handover without reconnect, and stale old-device events.
- `cargo test --offline --locked -p encdr graceful_shutdown_tests`: 2 passed.
- `cargo test --offline --locked -p encdr scan_filter_restricts_mk3_without_claiming_maschine_plus`: 1 passed. The shared discovery predicate accepts MK3, rejects Maschine+, and preserves the no-filter case. No other physical NI device was claimed to test this.
- `python3 -B scripts/check_mock.py` with unrestricted GUI access: passed three bounded sessions (demo quit, hardware-test mock quit, hardware-test mock SIGINT). Diagnostic sessions simulate E8 both directions, pad 16, top 8, group H, disconnect/reconnect and handover/takeover. Counters are preserved; attempted input while inactive is ignored; DOM confirms the expected counters and ACTIVE / RETURNS 1, SIMULATED INPUT label, capture bridge, 480×272 bounds and no diagnostic cells below y=230. All exited 0 within the five-second exit timeout. These are simulated lifecycle events, not physical replug evidence.
- Confirmed no prototype process was running, then ran a bounded real `target/debug/modular-machine-mk3 --led-test` session with RUST_LOG=info through the smoke script's Session helper. It discovered MK3 via the background monitor, reported ACTIVE / connections=1 / reconnects=0, accepted stdin quit, logged LED clears for button_leds/pad_leds and both exit screensaver transfers, then exited 0. The saved user hardware report was not overwritten. NI services were unchanged.

Remaining manual test: run `cargo run --locked -- --hardware-test --visible`; press group A–H; unplug USB, reconnect and check the same counters return and RETURNS increases; press physical STOP and check terminal return and the already-established neutral output state. If compatible NI software is running, separately test instance ownership away/back. Waiting with no device at initial startup, physical replug at either the same or new USB address, handover, group coverage and physical STOP with this revision remain unverified. Do not infer those passes from the mock tests or from the initial discovery/quit run.

### User-confirmed reconnect test

On 11 November 2026, the user replied “works” to the requested sequence: press groups A–H; unplug/reconnect USB and check counters survive/RETURNS increases; press STOP and check terminal return, LEDs off and Encdr logos. These checks are recorded as user-reported passes. No additional photographs or event logs were supplied for this sequence. The exact USB address before/after replug was not recorded, so do not claim both same-address and changed-address physical cases independently tested.

Remaining manual coverage: instance ownership handover, initial launch with MK3 absent, preview-window close, and explicit color/orientation/border QA. G1 audio/editor integration remains unimplemented.

## Scope correction and controller groundwork — 11 November 2026

User clarified the product: run the existing emulator/audio on the Mac and use Maschine for selected parameters and patch browse/load. They explicitly do not need a mock editor and do not want remaining generic Encdr tests to gate progress. Current scope is in docs/scope.md and the revised TODO. The separate mock-editor window, CLI and custom WebView IPC were removed; no mock-editor feature remains.

Controller groundwork retained: typed fixture parameters/module/slot navigation, EngineAdapter/MockEngine test double, and optional project-owned JSON fixtures. Persistence is opt-in with --state-file; normal launch does not create a parallel patch library. Fixture code is a development aid, not an emulator adapter or real patch support.

On the same macOS 26.7 arm64 / Rust-Cargo 1.93.0 host, after removal: `cargo fmt --check`, `cargo check --offline --locked`, `cargo test --offline --locked` (16 tests), `cargo clippy --offline --locked --all-targets -- -D warnings`, and `cargo build --offline --locked` passed. `python3 -B scripts/check_mock.py` passed three bounded GUI sessions: typed fixture value rendered in Hz/kHz with valid layout, demo quit, diagnostic reconnect/handover/counter/layout checks with quit and SIGINT. No MK3 session or new physical verification was requested for this cleanup. No NI processes changed. No commits made.

Changed source includes project.rs, engine.rs, model.rs, storage.rs, session.rs, options.rs, panel/runtime integration and typed screen values. The current implementation still has no real emulator connection or audio. Next engineering task is upstream G1-Emu interface/build investigation, followed by one real patch/two mapped parameters and then Maschine patch browse/load. Full patch editing stays in the existing emulator/editor.
