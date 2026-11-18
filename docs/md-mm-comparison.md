# Maschine MD-MM comparison and latency work

Reference inspected: [mo0kid/maschine-md-mm](https://github.com/mo0kid/maschine-md-mm), commit `af1ef88222f328824b4d2f8c69dac6be6e266eb0`. Source was read in a temporary checkout; the reference app was not built or launched. Its GPL notices remain upstream; the changes here use our existing G1 APIs.

| Path | Reference implementation | This project after the comparison |
|---|---|---|
| Hardware input | Native NIHIA callbacks on a serial dispatch queue | Existing Encdr events consumed by `mk3-input-dispatch`, independently of WebKit |
| Engine writes | Bounded front-panel command queue, drained into emulated UART | Bounded/coalesced G1 commands, dedicated localhost I/O thread, bounded engine-task queue and verified parameter/MIDI input |
| Ownership | Tracks press targets and releases on the original instrument | Device authorization gates input; device/client loss releases Function and pad notes |
| Display worker | Native RGB565 renderer, changed frames only, at least 60ms per display | Existing asynchronous WebKit capture, at least 33ms per display; upper keyframes and bottom partials |
| Shared work | Input callbacks do not wait for screen rendering | Hardware input no longer waits for the display loop; socket servicing no longer waits for a JUCE UI timer |

The reference is not simply rendering faster: its per-display 60ms limiter caps output around 16.7fps. Its important advantage is separating input and emulator commands from display work. Source: [controller input and render worker](https://github.com/mo0kid/maschine-md-mm/blob/af1ef88222f328824b4d2f8c69dac6be6e266eb0/source/elektron/md/mdJucePlugin/mdMaschineController.cpp), [native client callbacks](https://github.com/mo0kid/maschine-md-mm/blob/af1ef88222f328824b4d2f8c69dac6be6e266eb0/source/elektron/md/mdJucePlugin/mdMaschineNihiaClient.cpp), [bounded firmware queue](https://github.com/mo0kid/maschine-md-mm/blob/af1ef88222f328824b4d2f8c69dac6be6e266eb0/source/elektron/md/mdLib/mdhardware.h).

## Changes applied

- `src/input_dispatch.rs` consumes hardware events on a dedicated thread. It holds the device authorization gate while applying input, refreshes metadata only when its generation changed, and signals the display loop when state changes. It never runs WebKit. Diagnostic mode still forwards events through its existing reducer.
- `src/session.rs` shares the real G1 client behind a mutex; rendering takes a state snapshot, rather than owning the only input path.
- `Mk3ControlBridge.h` services sockets on its own thread. Synth RAM and parameter changes stay on the engine queue. Stopped-engine replies contain static metadata, avoiding startup races. Native Settings callbacks release the lifetime gate before opening UI.
- Host gain changes queue/coalesce preference saves onto the GUI thread, with a pending save flushed at bridge shutdown. Disk writes no longer run synchronously for every gain detent on the engine thread.
- Completed preset catalogues are polled every 10s instead of every 500ms; incomplete catalogues retain 500ms polling, and explicit refresh remains immediate. Catalogue snapshots share immutable storage instead of copying hundreds of names on each tick. Unchanged LEDs avoid rebuilding a full view snapshot on every loop.
- Encoder sensitivity stays at the requested 50%; existing 16ms write pacing remains. No MD/MM-specific UART command or mapping was copied into G1.

## Measurement boundaries

The first dedicated-transport test completed 12 four-RPC knob/MIDI/status batches during an intentional 600ms UI stall; maximum batch 12.4ms. All 72 MIDI bytes were consumed and no notes remained held. Evidence: `g1-state/ui-stall-1794477473615754000`.

A separate probe measured native set/readback median 9.6ms/max10.0ms and desktop command-to-confirmed-view-state median 49.9ms/max50.2ms (`g1-state/control-latency-1794477569845202000`). The latter includes the desktop event/render loop; it is not a physical hardware input measurement and does not describe the new independent hardware dispatch path. It also excludes physical screen scanout. Native hardware feel still needs use on Maschine.

The earlier parameter timing probe used a 100ms sleep between unsuccessful readback polls, which could add 100ms to a reported maximum. It now polls at 1ms for that measurement; do not treat its historical maxima as isolated proof of native stalls.

WebKit still adds a snapshot/compositing stage that the reference's native raster renderer avoids. If display lag remains after these input-path changes, measure state-to-capture-to-transfer before replacing the renderer; preserve the agreed Inter layout and authoritative values. A higher frame-rate setting alone does not remove the input/display coupling that was addressed here.

## Subsequent captured output freeze

The user subsequently reported frozen LEDs/screens with working pads. A live sample isolated the main thread in repeated Tao/AppKit loop entry, while output workers waited for new frames/lights. That per-frame loop re-entry has been replaced by a single persistent AppKit loop; views are created before it, application ticks use an autorelease pool, and preview capture is asynchronous. The independent input path explains why pads continued working during the old stall. New desktop command-to-confirmed-state timing was 35.4ms median/40.2ms max (`g1-state/control-latency-1794479382402474000`). The same live emulator was retained while replacing the controller, and output heartbeats/capture counters continued beyond the previous freeze interval. This addresses a captured failure rather than attributing every freeze to message volume.
