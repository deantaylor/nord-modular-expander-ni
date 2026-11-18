## Hidden renderer background policy — 13 November 2026

Screen delay persisted after removing the value cache. The hardware renderer's hidden WebKit views still used default background throttling. Set Wry BackgroundThrottlingPolicy::Disabled (supported macOS14+), preserving existing capture cadence/backpressure. Added state-script execution timing warnings above100ms: snapshot duration alone excludes waiting for JavaScript execution.

Focused hidden real-WebKit check: five value updates reached DOM in16–21ms. Release build passes. Short smoke testing does not establish multi-minute idle behavior or physical panel timing. Parameter/audio path unchanged.

## Current display-only correction — 13 November 2026

The user confirms sound responds properly; the remaining complaint is knob-to-screen delay. Removed a display cache that rejected new observed values outside the expected movement range while an edit was pending. Renewed turns extended that hold. The UI now renders the latest actual engine value, including reversals; pending target accumulation/confirmation stays separate and no pending target is presented as confirmed.

Two focused tests verify repeated reversals cannot hold old numbers and unconfirmed targets remain hidden. Rust release passes. No adapter/audio, HTML or33ms capture changes. This is a verified state-display correction, not a measured physical scanout improvement.

## Current mixed-operation follow-up — 13 November 2026

Remaining user-reported lag also occurs on preset arrows. Actual live log showed Upload timeout at packet0; a file load could begin while the adapter still owned a parameter section read. File uploads now defer until that transaction/SDK work drains, clearing old parameter state before start. Startup settings read retries independently without NME's global reply lock. Rate-limited CONTROL_WAIT diagnostics record remaining actual read/fetch/upload waits.

Two protected12-load sequences mixed knob edits with Defunkt/MK3-MIDI and filterbells1/MK3-MIDI; all pass, maximum confirmed loads339/493ms, shared bank unchanged. This does not reproduce/measure the remaining physical delay and is not a claim that all lag is resolved. Test harness excludes automatic watcher attachment via the existing launcher lock and refuses while the production controller owns it.

## Four-second stall reproduced and corrected — 13 November 2026

The adapter’s background parameter query also locked NME’s global reply queue. A missing reply blocked knob writes for its3000ms timeout; periodic2s settings polling could trigger the same problem. Targeted reads now use the adapter’s own deadline/assembly without the global reply lock. Automatic settings polling is removed; connection-time and explicit setting-change refresh remain. Full patch operations retain their safeguards. Screen pacing remains33ms.

A test relay withholding actual replies reproduced4s starvation: old adapter133 requests →1 wire parameter frame,3996ms gap. Fixed adapter136 wire frames in4s, maximum gap59ms, then confirmed recovery. Bank unchanged. Run `python3 scripts/check_official_plugin.py --lost-reply`; evidence g1-state/official-check-1794574471903442000. This is protocol-wire evidence, not a physical scanout measurement.

## Current correction after physical feedback — 13 November 2026

The user found the16ms capture/read-first update worse. S49 capture is restored to33ms. Parameter writes now take priority over background section reads and do not wait on the adapter’s readInFlight flag; full upload/fetch and NME protocol wait guards remain. First read is due16ms; subsequent section starts are limited to40ms. NME’s supported sendParameterFromUser is restored. Persistent native CF loop remains. These supersede the experimental sender/cadence described below.

Actual Defunkt-patch protected testing: old median43.1/max64.8ms; final42.6/max50.8ms; continuous144 writes/reversals pass with confirmed progress, shared bank unchanged. This is not a substantial median-speed improvement or proof of physical responsiveness. The change removes a structural input gate and reverses the unaccepted rendering tuning. Confirmation notices now clear on success and report expiry instead of staying “awaiting” forever.

## Current official-plugin / S49 follow-up — 13 November 2026

The official adapter now reuses the standalone's captured `$13/$40` parameter frame without a checksum or extra focus message. First targeted readback is due after16ms and later writes cannot postpone it. A persistent native CF event loop and coalesced2ms wake-up replace repeated timed AppKit pumping; all NME model/protocol calls remain on the main thread. This is not the standalone's dedicated synth transport thread and cannot guarantee progress during a blocked NME callback.

Like-for-like protected official-VST checks: old median60.4ms/max87.3ms command-to-confirmation; updated32.1ms/max36.1ms (16 alternating targets). During144 continuous/reversing writes, confirmed changes increased68→119 and maximum confirmation gap decreased98.2→54.0ms. Shared bank unchanged. Evidence: g1-state/official-check-1794569194252496000 and g1-state/official-check-1794569158795690000. The Python probe uses buffered socket reads; earlier unbuffered numbers overstated latency.

S49 capture interval is16ms; one pending capture per display and backpressure still bound output. Other controllers retain33ms. This is a scheduling limit, not measured physical scanout or guaranteed60fps. Physical feel still needs user acceptance.

# Encoder response and display investigation — 11 November 2026

The MK3 reports relative encoder deltas. The controller accumulates them into bounded raw parameter targets, retains fractional motion, and coalesces queued writes by assignment. This is separate from how the emulator applies a target.

## Measured input bottleneck

The original bridge converted each target with `KnobMap::positionFor` and wrote an emulated panel ADC channel. That is a potentiometer-input simulation. The inverse scaling is correct, but the firmware scanner adds delay.

Comparable isolated real-ROM probes used the project-owned sine patch, a private TCP port and temporary flash. Each measured 20 status calls and 12 alternating adjacent knob values. An unsuccessful confirmation poll waited 10ms; measurements include RPC and polling time, not physical scanout.

| Native path | Status median | Knob confirmation median | Observed maximum |
| --- | ---: | ---: | ---: |
| ADC, 20ms bridge poll | 17.72ms | 95.54ms | 116.27ms |
| ADC, 5ms bridge poll | 4.62ms | 99.34ms | 114.46ms |
| Editor parameter messages, 5ms poll | 3.93ms | 11.62ms | 13.26ms |

The direct probe confirmed every change on the first status poll and traced exactly 12 parameter messages. Evidence: `g1-state/reverse-audit-editor-1794423023605832000`. Earlier probes: `reverse-audit-1794422415877002000` and `reverse-audit-5ms-1794422677498286000` under `g1-state/`.

A separate audio/safety run measured 12.4ms median with one 118.6ms outlier. This is evidence of a substantial median improvement, not a guarantee that every update has fixed latency. Its recorded DSP waveform changed from about331Hz to659Hz, with a changed level ratio7.8. Morph-value writes, held Function gating and stale-assignment rejection passed. Evidence: `g1-state/encoder-path-1794423078775318000`. Reproduce with `python3 scripts/check_encoder_path.py`.

## Verified editor message and integration

The incoming parameter frame is:

```text
F0 33 (4C | slot) 06 pid 40 section module parameter value F7
```

There is no additional checksum in this editor message. Source evidence is the existing local Nord-Modular checkout at commit `e909a4584645ed154bc507c5f425c7f747e6a89f`: `webmidi/core/src/protocol/client.ts`, `sendLegacyParameterChange`; recorded editor frames in `webmidi/docs/comms-audit-plan.md`; the legacy `$13/$40` grammar in `legacy/nmedit-master/libs/libnmprotocol/src/midi.pdl`. Morph values use section2, module1 and the morph-group parameter index. The older C++ generic message builder adds a checksum; the capture-backed parameter sender does not. The real-ROM test verifies the selected framing.

The native bridge validates the complete knob batch against current assignments and ranges. PresetsLink coalesces parameter writes and sends them on the existing PC Port while holding its worker-side protocol ownership. Live writes can preempt background reads, but never upload/load/store operations. Patch IDs come from existing RequestPatch replies, are invalidated on local load/upload, and are checked again when a queued value is sent. Unknown identity retains the established ADC fallback. Function hold preserves the existing panel-control gating. No patch parameter is written directly to OS RAM.

This does not prove simultaneous external-editor synchronization. The existing editor remains responsible for patch construction. The native build and source patch are reproducible through `scripts/build_g1.py`; modified sources remain under GPL.

## Reversal and rendering investigation

The former browser tests checked the final DOM after scrolling. They did not prove intermediate hidden-window frames were present. `scripts/check_browser_motion.py --hidden` now runs the same asynchronous native snapshot path with hidden WebKit views, checks image bands during 124 direction changes before animations settle, and verifies settled focus, gradient, reopen, click-to-load, bold activity and hidden knob controls on both screens. JavaScript errors are reported through the native bridge. Latest evidence: `g1-state/quick-fix-1794423206548636000` (57 captured reversal frames without blank list frames).

The physical reversal failure was not reproduced in those WebKit snapshots. On the MK3 service path, upper-screen animation changes now use complete frames so a lost incremental update cannot leave later browser frames dependent on an incorrect retained image. Bottom numeric changes retain fast partial rectangles; duplicate frames are skipped. A focused screen-manager regression verifies all three cases.

Actual NI transfer measurements explain why complete frames are not used for every value change: median full transfer29.6ms versus3.8ms for partial transfers in the refined physical check. Latest captures were about18.9/18.7ms and command-loop median14.3ms. Evidence: `g1-state/screen-timing-1794422787579484000`. These are pipeline measurements, not photographs or a user-perceived smoothness pass.

The display still coalesces native snapshots to at most30fps. Shorter event-loop waits and a wakeable4ms worker receive interval reduce additional scheduling delay. Browser hides both knob strips. Value activity uses Inter weight400 rather than glow.

The updated native app must be running to use the new input path. The launcher checks its capability and gives a save/normal-restart instruction instead of silently attaching to the old ADC bridge.


## Thread-safe bridge follow-up

Requests now execute on the engine thread through a bounded queue; GUI polling is 2ms. The final isolated safety/audio run (`g1-state/encoder-path-1794474492107972000`) measured 10.1ms median confirmation with one 114.6ms maximum. Stale bindings, Function suppression and morph writes passed; recorded frequency/level changes passed. These are command-to-engine-readback measurements, not physical knob-to-display scanout or perceived smoothness guarantees.


The MD-MM comparison moved hardware dispatch and native socket servicing off their respective display/UI loops. See [measurement boundaries](md-mm-comparison.md). Historical maxima from `check_encoder_path.py` may include its former 100ms readback polling delay; that probe now uses 1ms. Direct readback and command-to-view timings must not be described as physical panel scanout.
