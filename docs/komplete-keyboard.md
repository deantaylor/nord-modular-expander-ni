## PATTERN — sequencers

Press **PATTERN** (between TRACK and SCENE) to list the current slot’s Note Sequencer A/B, Control Sequencer and Event Sequencer modules on the left display. The wheel selects sequencers on the left until **Edit Sequence** is enabled with display button5. The right display is locked otherwise; left/right tilts cannot focus it while locked.

Press **display button 5** to toggle **Edit Sequence** in the right header. Inactive is outlined; active is filled, alongside the blank title bar and selection count. While editing, the wheel controls the right display:

- Tilt **left/right** to select one of the 16 stored steps.
- For notes, intervals and values, **rotate** to change the selected step directly. No centre press is needed.
- For Event Sequencer, rotate or tilt left/right to browse steps, tilt **up/down** to choose row1/row2, and **press** the wheel to toggle the selected event. Rotation does not toggle events.

Press display5 again to lock the right display and return the wheel to sequencer selection on the left. Press PATTERN again, TRACK, another mode/page button, or a dedicated parameter-page arrow to leave the page/edit mode. Wheel directions are separate hardware inputs from the dedicated page-arrow buttons. The eight parameter knobs are inactive on this page.

All edits go to actual module parameters and use confirmed feedback. **CLEAR** restores sequence edits alongside other changed parameters. No patch reload or VST modification is used. Every step has a circle: event steps are filled when on and outlined when off. The actual running playhead turns its circle red, separately from the editing-selection border. Live positions come from the emulator’s meter stream; stopped/invalid or stale data shows no red marker. Both EventSeq rows share its playhead. Engine length is displayed separately from the16 stored values and is not rewritten/clamped by navigation.

PATTERN is off/inactive when the patch has no supported sequencers, dim when they are present, and bright on its page. Module metadata is checked without opening an editor. Display5 is dim when editable and bright in Sequence Edit. PRESET UP/DOWN now have verified lamps and reflect normal previous/next availability across pages. Empty patches show a no-sequencers message without enabling edit mode.

## CLEAR — restore loaded slot values

Load a patch into a slot, adjust its assigned knobs or Module Editor parameters, then press **CLEAR** to restore that slot’s original parameter values. Only changed values are written; unchanged parameters are skipped, and CLEAR on an unchanged slot sends no writes. Recent unconfirmed edits remain eligible so fast turn-then-CLEAR is safe. The bridge confirms readback; it does not reload/upload the patch, change assignments or affect other slots. CLEAR is off and inactive when the slot matches its baseline, dim only while parameters differ (including unconfirmed edits), and bright while restoring. It turns off again after confirmed restoration. Further knob edits wait for completion; release SHIFT before using CLEAR.

A separate baseline is retained for each slot during the bridge session and replaced on a confirmed patch load. When attaching to an already-running patch, the first confirmed snapshot is the baseline; edits made before the bridge connected cannot be recovered. Restarting the bridge establishes new baselines. This restores ordinary editable parameter values, not patch structure, MIDI settings or saved banks.

The proposed KEY MODE keyboard split is deferred to standalone software; see [design](standalone-keyboard-split.md). It is not available in the current VST bridge.

At full screensaver glitch intensity, occasional **FEED ME** or **CONFORM** messages hold for two seconds before glitching away. They occur less often than ALERT.

The bridge appears as **Nord Modular** in the keyboard’s INSTANCE list (previously Encdr).

## Idle screensaver

After five minutes without keyboard interaction, the UI fades to blobs. Glitch intensity rises from the saved level to 100% over sixty minutes. Any surface control or performance MIDI activity wakes it and fades the previous UI back in. Held notes prevent idle. Saved settings remain unchanged; Colour uses Dark temporarily for the saver. Glitches are disabled in normal Colour mode; automatic theme glitches alternate Light/Dark only.

## Display effect intensity

TRACK+8: knob2 Blobs On/Off, knob3 Glitch intensity0–100% (default50%). Display now previews the effect live. Above90% transient theme jumps automatically return; at100% an occasional three-frame ALERT screen flash goes clear → glitched → clear → hidden. Saved theme and MIDI routing stay unchanged.

## MIDI-reactive Home animation

Both Home screens share one continuous, drifting field of dots. It breathes gently at idle; note density increases its spread and number of lobes, while velocity strengthens expansion and motion. It becomes faint behind parameter values to preserve readability. Held notes keep the expansion/intensity up until the final note-off (or sustain pedal release), then it relaxes smoothly. Occasional short dot-row glitches and RGB tears add variation. It follows the current screen theme and disappears on other pages.

A passive subscriber observes **KOMPLETE KONTROL S49 MK2 Port 1**; it does not forward, remap or consume keyboard MIDI. DAW clips and other instruments' MIDI are not exposed by the official VST bridge and do not drive this animation. The dots are an input-activity visual, not an audio meter or sounding-voice display.

## Settings access and themes

**TRACK + display button 7** opens emulator Settings. **TRACK + display button 8** opens Display: **knob1** selects Light, Dark or Colour; **knob4** adjusts colour intensity in Colour mode. Light is the default; chosen theme is saved. **TRACK** returns Home. Slot LED colours remain A red/B green/C orange/D yellow across all themes. No MIDI-channel reassignment is applied by this update.

## Exclusive pages and TRACK Home — 16 November 2026

BROWSER, Module Edit, Mixer and Settings are exclusive pages. Opening Browser leaves the previous page; closing it or loading a patch returns to the assigned-knob Home view, not Module Edit. The right browser screen shows source/folder and navigation help, without repeating the selected patch name.

Pressing TRACK returns to Home from any bridge page. Holding TRACK still makes display buttons1–4 select slots A–D. Releasing it stays on Home. Home retains the last assigned-knob page; no old editor/mixer page is resumed underneath another page. Entering another page closes the old one and clears its pending control targets.

## Screen headers and blank Mixer display — 16 November 2026

Screen1 now carries the slot/patch name and allocated(requested) voice counter. Screen2 carries the parameter page counter; the Nord Modular brand title is removed. The patch/browser presentation follows screen1; encoder positions and right-hand module diagrams remain tied to their physical screens.

In Mixer, screen2 is completely blank, including headers, modifiers and notices. Its intentional blank capture bypasses the macOS ink-recovery guard so the previous page is actually cleared. The guard is restored on leaving Mixer and accepts other screen2 content without requiring a brand heading.

## Controls-only Mixer and readback priority — 16 November 2026

Mixer now shows only its knob controls: no duplicate slot/patch list above them, no meters/bars, and no Master section on the right display. Knobs1–4 remain A–D; unavailable or multiple-output patches offer no level control. The right display is blank in Mixer except necessary modifier/status feedback. Nord master control remains deferred; no DAW volume is controlled.

Active edits take priority over stalled inactive-slot readbacks. Reads have a bounded retry budget, and leaving Mixer discards obsolete inactive reads. A repeatedly failing slot model becomes unavailable until it is read afresh rather than blocking all other knob feedback. Incoming-pid mismatches are logged during tracing for diagnosis. These changes leave the audio engine and official VST untouched.

## Revised Mixer and input recovery — 16 November 2026

All four slots appear together on the physical left screen. **Knobs1–4** map directly to A–D. No meter-style bars are shown. Patches with multiple Output modules or no Output module offer no level control; unused encoder turns are ignored. Uncached slots say Select slot to read and load through normal TRACK slot selection, without Mixer starting background full-patch reads.

The right screen contains the **Nord master** section. Master control is deferred by user choice so the official VST stays unmodified; it does not control Live or any DAW fader. Use G1-Emu’s own master knob. The previous implementation read/wrote the Nord master-pot input inside the locally modified emulator; that in-process route cannot be reused by the external stock-VST bridge.

The adapter now pins parameter writes/readback to each slot’s confirmed patch ID, preventing inactive-slot patch replacement from losing a later normal knob update. Slot commands take priority over parameter reads, preserving the ROM’s300ms spacing. PLUG-IN from Mixer opens Module Edit reliably, and ordinary knobs resume after exiting Mixer.

## MIXER page and patch output levels — 16 November 2026

Press the physical **MIXER** button (after PLUG-IN) to open/close Mixer. Its captured input is report01 byte5 mask01; calibrated lamp37 is dim when available, bright while Mixer is open. SHIFT+MIXER is left to NI’s native secondary function; it does not toggle our page.

The left screen shows all four slots; the right contains the deferred Nord master section. **Knobs1–4** control their respective patch Output-module levels (raw0–127 using the upstream formatter). The other knobs are inactive. The horizontal bars show level settings, not audio activity. Exactly one1Output/2Output/4Output module in a patch makes its level unambiguous; patches with multiple Outputs are labelled and require choosing the desired output in Module Edit. No output module means no level control. All edits retain binding/range checks and real readback; loading another patch invalidates old targets. Native level changes modify those patch parameters, not host mixer faders.

The bridge lazily fetches the four slot models on entry; it does not change slot focus to edit inactive slots. Pending values are explicitly marked until confirmed. The current official external interface does not expose host master volume/gain or final per-slot output VU telemetry. Nord master control is deferred; no DAW volume substitution is used. Module-specific meter messages exist for meter modules, but are not a measurement of each slot’s final output and are not presented as one here.

## Confirmed slot feedback — 16 November 2026

While TRACK is held, exactly the engine’s active slot among buttons1–4 is bright. Other slots stay dim even if enabled/pinned; a held slot button does not override confirmed focus. Both screens’ TRACK hint includes the active slot letter, and the patch header retains it while new patch metadata loads. The adapter fetches uncached confirmed slots rather than leaving the surface permanently NOT READY. Slot selection and Function release remain available during that read; parameter edits still wait for valid metadata.

This uses the connected **Komplete Kontrol S49 Mk2,17cc:1610**, its calibrated indexed-white lamps and32-byte input reports. S49 Mk3 is not the selected profile.

## Slot selection and native SHIFT chord — 16 November 2026

TRACK + display1 selects A even when another slot change is pending. The adapter sends the slot-enable mask and activation separately, with300ms quiet spacing matching the official emulator’s settings link; confirmation still comes from the OS active-slot readback. Selecting status clears after success. An isolated official-VST check confirms B→A,C→A,D→A in624–810ms without touching shared banks.

SHIFT retains NI’s native secondary functions. NI documents SHIFT + MIDI/PLUG-IN as its MIDI-mode switch: https://support.native-instruments.com/support/solutions/articles/69000880031-native-instruments-switching-your-controller-to-midi-mode . The bridge reserves SHIFT+PLUG-IN for that native transition, suppresses Module Edit and pauses until the existing NI ownership handover back to Encdr. Plain SHIFT still holds the bridge’s Function modifier; it cannot exclusively replace keyboard-native SHIFT. No recent plain-SHIFT ownership loss was observed, and there is no evidence that SHIFT caused the slot-selection failure.

## Calibrated S49 controls and lamps — 16 November 2026

PLUG-IN toggles Module Edit; display buttons5–8 are unassigned/off. Hold TRACK + display buttons1–4 for slots A–D. SHIFT holds the existing Function modifier. Eight knobs and preset/page navigation keep their existing actions.

The corrected Encdr descriptor uses **indexed white** for display buttons1–8 at payload offsets2–9 (dim69, bright71). Monochrome lamps use SHIFT14, TRACK23, LEFT32, RIGHT33, BROWSER35 and PLUG-IN36; user confirmed white at158. Output15 is SCALE,31 STOP,34 CLEAR and37 MIXER; these stay unassigned/off. Inactive/available actions are dim, active choices/held buttons bright, unavailable actions off. Preset up/down lamp positions remain unverified and are not programmed; their input actions still work.

Ordered physical input capture and isolated lamp observations are recorded in `g1-state/s49-physical-input-map-20261116.json` and `g1-state/s49-led-calibration-20261116.json`. The inherited map had both wrong names/positions and the wrong display-button encoding; sending monochrome158 to display buttons caused green lights.

# Komplete Kontrol S49 Mk2 with the official G1-Emu VST

Development now targets the connected **S49 Mk2 (`17cc:1610`)**. Use the official **G1-Emu.vst3** or **G1-Emu FX.vst3**. This project runs an external control/display bridge; it does not modify the VST or run another synth engine. Keybed MIDI stays independent in your DAW.

## Normal use

1. Rescan plugins in the DAW if needed, then load **G1-Emu** from the official release. The older **Modular Machine G1** builds are retired from this workflow; existing projects are preserved.
2. Once the background service is installed, opening a plugin automatically starts the external adapter and S49 controller. There is no Terminal or launcher step on each load. Closing the plugin stops those two processes; the small watcher waits for the next instance.
3. Keep NI agents running. If the keyboard is showing another app, press **INSTANCE** and select **Nord Modular**. Automatic bridge startup and NI hardware focus are separate: the bridge does not kill other software or repeatedly steal focus.
4. Use Browser to load a `.pch` file if the official plugin has an empty patch. A knob needs an assignment in the patch to control a parameter.

The background service is installed once with **Komplete Keyboard Plugin.command**, or:

```sh
python3 scripts/install_keyboard_service.py
```

It runs as your user through `~/Library/LaunchAgents/local.modular-machine.keyboard.plist`, including after login. The other existing project `.command` entry points now install/start this same keyboard service. They no longer start the custom synth build.

## Patch library for background use

macOS prevents the login service from reading the project folder in Documents. The installer makes a private, accessible copy of the configured `.pch` library under `~/Library/Application Support/Modular Machine/patch-libraries/`; originals are untouched, and these user files are not bundled into the distributable runtime. This installation imported 702 local patch files.

`keyboard-preferences.json` records `patch_dir` for the active private library and `source_patch_dir` for the original location. Add new `.pch` files to the active `patch_dir` and refresh Browser. Reinstalling the bridge preserves this private library. It does not continuously synchronize the original Documents folder.

## Keys remain independent

The bridge opens only the official editor/control connection and S49 surface interfaces. It does not open the keybed MIDI port, intercept notes, transpose them or inject them into the plugin. Keep your existing DAW MIDI input/track monitoring and recording settings. No additional keyboard routing is required for the surface.

The Nord Modular screen now shows the current assigned-knob page as **PAGE 1 / 3**, **PAGE 2 / 3** or **PAGE 3 / 3**. The other screen keeps allocated(requested) voices. Page numbering is hidden in Browser and other views.

See [native Edit path and proposed module page](module-edit-mode.md) for using a screen button to switch from assigned knobs to real parameters of a selected module. Module Edit is now available using PLUG-IN; see the mapping below.

## Surface mapping

| S49 control | Action |
| --- | --- |
| Eight knobs | Current page of actual patch knob assignments, four per display |
| BROWSER | Toggle the patch browser, as on Maschine |
| Page left/right | Previous/next knob page |
| PRESET up/down | Previous/next patch, exactly like Maschine’s left/right preset arrows |
| 4D encoder press | Open Browser; press again to load the focused entry |
| 4D encoder turn/directions | Browser selection; left goes to parent folder in Files; left/right scan knob pages outside Browser |
| Top 1–4 in Browser | Files, Banks, Refresh, Load |
| PLUG-IN | Toggle Module Edit / assigned-knob view |
| Top5 | Unassigned; lamp off |
| Top6–8 | Unassigned; lamps off |
| SHIFT | Hold existing bridge Function; release clears it |
| TRACK + display buttons1–4 | Select slots A–D; also hold SHIFT for existing Function slot pinning |

After a confirmed successful file/bank load, Browser closes and the parameter view returns. Highlighting entries and entering folders do not close it; failed loads leave it open for retry. PLUG-IN toggles Module Edit; Top5–8 remain unassigned with lamps off.

Available mapped buttons with known lamp offsets are dim; active Browser source buttons are bright. Unavailable actions are off. Page arrows are independently dim while available, bright while pressed; preset arrows respect list boundaries and loading. BROWSER and TRACK use the calibrated lamps listed above. Native/DAW lamps are not assigned by the application.

Knob values, assignments, patch names and allocated/requested voices come from the official PC Port protocol. Parameter display units use NME's formatters. Encoder writes remain coalesced and use the existing half sensitivity; readback uses the affected parameter section rather than downloading the whole patch per movement. The first readback is scheduled after 16ms and is no longer postponed by every subsequent write; writes have priority; due reads start only after queued targets drain and are limited to one start per40ms. This keeps confirmed values advancing during sustained turns. Incomplete patch downloads do not replace confirmed snapshots or count as a successful load.

BROWSER is captured at report0x01 byte5 mask0x04. The former byte6 mask0x04 “SHIFT” identification was wrong: the user identified it as a dial event. It is now ignored. A fresh isolated user capture identifies SHIFT as byte2 mask0x80 and TRACK as byte4 mask0x10. SHIFT now holds the existing bridge Function state. TRACK is a separate local slot-selection modifier: hold it with display buttons1–4 for A–D; it never invokes Function by itself. With SHIFT also held, those slots use the existing Function pinning behavior. TRACK release leaves the surface on Home and restores normal Home button actions. BROWSER/TRACK lamps are calibrated. Host audio settings and gain remain plugin/DAW controls. No new Maschine development is planned.

## Native controls and conflicts

| Controls | Bridge policy |
| --- | --- |
| SETUP | Native hardware settings; no emulator settings action |
| INSTANCE | NI instance selection; select Encdr to return to our surface |
| MIDI | Native MIDI mode/templates; no synth action |
| SHIFT | Explicit bridge Function modifier, only while Encdr owns the instance |
| PLUG-IN | Explicit Module Edit toggle |
| MIXER | Toggle the slot Output-level page |
| TRACK | Return Home; hold with display buttons1–4 for slot selection |
| Transport, mute/solo and editing buttons | DAW functions; no synth actions |
| SCALE/ARP, octave, keybed, wheels, strip, pedals | Native/performance MIDI; untouched |
| BROWSER, preset/page arrows, top1–4 in Browser, eight knobs and 4D | Explicit bridge assignments above, only while Encdr owns the instance |

Detected SETUP/MIDI/INSTANCE presses pause our surface input and display/LED updates. Resume by selecting INSTANCE → Encdr after the NI ownership handover. A native menu that retains NI ownership must not also edit the synth. Reconnecting the device resets the local pause. Physical menu transitions still need user acceptance.

NI documents SETUP as keyboard settings and SHIFT as a contextual modifier in its [S-Series Mk2 manual](https://www.native-instruments.com/fileadmin/ni_media/downloads/manuals/komplete_kontrol/KOMPLETE_KONTROL_S_Series_MK2_Manual_English_2_0.pdf) (hardware overview and host-integration chapters). [Encdr’s Mk2 hardware reference](https://github.com/RufusIbiza/Encdr/blob/main/docs/hardware/ni_komplete_kontrol_mk2.md) describes the NI-service ownership handover and separately lists HID events. Detectability does not establish exclusive ownership. Its generic encoder-report description differs from this device's recorded report; retain the locally verified knob decoder.

BROWSER and other assigned panel controls can also have NI/host behavior. Encdr ownership is required, but cannot guarantee that a separate DAW control-surface script ignores its own MIDI port. If duplicate actions persist in Live, disable only the S49 Control Surface/Remote integration that handles those panel actions, preserving normal keybed MIDI Track input. The bridge does not change DAW routing or disable NI services automatically.

### Observed Live configuration — 13 November 2026

Live 12.4.16's latest MIDI configuration log (11:48:40, lines33356–33381) has `Komplete_Kontrol_S_Mk2` active with both input and output set to `Komplete Kontrol DAW - 1`. S49 Port1, Port2 and DAW-1 all have input Track and Remote enabled. `Komplete_Kontrol_A` is present but has None/None ports; it is not a second connected S49 script.

This confirms an independent active DAW control path alongside our HID bridge. Remote=true alone does not prove a learned MIDI assignment or prove that a physical button reaches both consumers. SETUP-to-synth and the dial-to-Function binding were definite conflicts in our previous mapping and are removed. Distinguish those from mode-dependent DAW-script overlap; do not claim every enabled MIDI port is a duplicate binding.

## Module Edit

Press **screen button5** to switch from the18 assigned patch knobs to the real parameters of a selected module. Turn the main dial, use4D up/down, or top buttons1/2 to select a module. Page arrows (or4D left/right) page through that module's parameters. The eight encoders correspond to the displayed parameters, four per screen. Press5 again to restore the assigned-knob page.

The module list shows selected/neighboring names and Poly/Common context. PAGE X/Y now counts that module's parameter pages. Only selected-module values stream; the full catalogue loads on entry or patch change. No saved knob assignments are rewritten. Modules without parameters remain selectable with inactive encoders. SETUP/INSTANCE and keybed MIDI remain independent; SHIFT/TRACK follow the explicit mappings above. This is a module-control page, not a cable/module-construction editor or native Edit-cursor mirror. [Implementation and controls](module-edit-mode.md).

### Envelope, filter and oscillator diagrams

In Module Edit, selecting **ADSR**, **Mod-Env** or **Filter A–F** draws a parameter-driven diagram on the **physical right-hand screen** above its four encoder tiles. Envelope stages/shape/inversion and filter cutoff/resonance/mode/slope/bypass follow parameter values across pages, with requested changes shown immediately and marked pending until confirmed. Multimode filters show LP/BP/HP lines with a legend.

Graphs use a dark panel, fine grid, colored filled curves and envelope control points, inspired by Serum’s graph presentation. OscA/B show sine, triangle, saw or pulse from their verified waveform selector; pulse width follows the upstream percentage formatter. OscC shows sine, OscSlvB shows pulse/width, and fixed sine/triangle/saw slave oscillators show their base wave. Module Edit tiles omit knob numbers and repeated module titles.

Module browsing remains on the left. The curves are base-parameter schematics; they do not show live audio, gate/modulation signals or measured DSP response. Unsupported modules retain their parameter controls without a diagram. [Supported types and details](module-edit-mode.md#right-hand-parameter-diagrams).

## Multiple instances and ownership

The service owns one selected surface. Use **Komplete Keyboard Plugin.command** to select the S49 Mk2. See [Maschine controls](key-assignments.md#maschine-mk3-with-the-official-vst) for the other profile. By default it attaches to the first available plugin on the official DirectLink range `47310–47317`, and retains that instance until it closes. A standalone greeting is ignored. It does not infer the selected DAW track.

To select a specific instance, set `direct_port` in `~/Library/Application Support/Modular Machine/keyboard-preferences.json`; preserve your `patch_dir` entry. Remove `direct_port` for automatic selection. Changing the selected port cleans up only the service's own adapter/controller before reconnecting.

Enable DirectLink in the official plugin Settings if you have disabled it. DirectLink allows **one editor client**. The bridge waits if that connection is already occupied; simultaneous NME editing over that same DirectLink is not claimed. It cooperates with existing manual launcher locks rather than killing their processes. Quit an old manual controller when migrating to the automatic service.

## Diagnostics and stopping

Files under `~/Library/Application Support/Modular Machine/`:

- `keyboard-status.json`: watcher state and its owned child PIDs/selected port.
- `keyboard-service.log`: discovery/lifecycle log, rotated automatically.
- `keyboard-controller.log` and `adapter-session.log`: current connection diagnostics.
- `keyboard-launch-error.log`: service bootstrap/runtime failures.

To stop the background service:

```sh
launchctl bootout "gui/$(id -u)/local.modular-machine.keyboard"
```

This leaves the VST, DAW, NI agents and key MIDI running. Delete its LaunchAgent plist as well if you want it disabled across login. Run the installer again to enable it.

## Source and verification

The external adapter reuses the official [Animatek NME](https://github.com/animatek/Animatek-NME) protocol/model code, without opening the editor. See [adapter source/build notes](../integrations/g1-official/README.md). Official VST binaries are downloaded unchanged from [G1-Emu releases](https://github.com/animatek/G1-Emu/releases).

The S49's native input is a 32-byte `0x01` report: knob positions at little-endian byte pairs 10/11 through 24/25, on a 0–999 ring; main encoder at byte 30. A physical capture and user-confirmed knob response established this mapping. Focused replay tests cover all eight encoders, directions/wrap and ownership resynchronization.

Real official-VST verification covers patch download/upload, knob assignments, confirmed parameter edits, voice counts and stale-target rejection, with the shared bank file unchanged. Background startup/cleanup is tested separately with hidden controller views; those checks do not claim physical feel or all button mappings.

Button/responsiveness follow-up: unready status now carries no knob metadata, avoiding the former controller reconnect loop. Ordinary command rejections preserve the connection. Files mode does not request a bank scan unless Banks is selected; parameter edits take priority over an active bank scan. The incorrect dial-to-Function binding is removed; native and unassigned controls are filtered before synth dispatch.

Screen-lag verification (13 November): the isolated official-VST continuous/reversing test sent144 writes and received72 confirmed value changes during movement; maximum inter-confirmation gap81ms. This measures engine readback, not physical display latency. Eight focused control/browser checks and the release/native builds passed. Captures in the existing live log were about4ms; separate NI ownership handovers pause display output and remain a coexistence limitation. Health reports now include maximum capture duration and screen backlog.

Latest lag follow-up: the adapter uses NME’s supported parameter sender and a persistent native event loop. S49 capture scheduling is33ms (the16ms experiment was reverted), retaining one pending frame and backpressure. Corrected like-for-like engine measurements improved median60→32ms and maximum87→36ms; these are not physical scanout timings. See [latency evidence](encoder-response.md).

Responsiveness correction: the earlier read-first/16ms capture tuning was rejected by user testing. Writes now take priority over background section snapshots, without the adapter’s full-section readInFlight block. Full patch operations and protocol reply serialization remain protected. Screen captures are back to33ms. Pending confirmation status clears on success or reports expiry instead of appearing stuck forever. An isolated Defunkt continuous/reversal check passes; physical responsiveness is still for user assessment.

Four-second stall correction: background parameter reads no longer lock NME’s global three-second reply queue, and automatic two-second settings polling is removed. Startup and explicit settings refresh remain. A dropped-reply test reproduced the old4s stall (one outgoing knob write) and verified136 outgoing writes with maximum59ms gap after the fix, plus confirmation recovery. Screen pacing remains33ms; no changes to DAW/key MIDI. See [wire-level regression evidence](encoder-response.md).

Mixed-operation follow-up: file/preset uploads now drain an existing parameter read before starting and clear old read targets/state. Startup settings retry independently until known. Long waits are recorded as `CONTROL_WAIT` in adapter-session.log, including read/fetch/upload/deferred flags. Two isolated12-load edit/preset sequences pass (including filterbells1), maximum confirmed load493ms, banks unchanged; physical lag remains for user assessment. Isolated test hosts now require the launcher lock and refuse while the production keyboard bridge owns it.

Immediate encoder feedback: requested values drive the screen before engine readback. Numbers use the adapter’s upstream descriptor formatter in its acceptance reply; curves use the requested raw value immediately. A small dot/bold number and diagram PENDING label distinguish the preview. Confirmed values remain separate and authoritative; confirmation clears the marker, while rejection, timeout, disconnect or a changed parameter/patch removes stale feedback. This supersedes the earlier confirmed-only display correction. Audio writes, polling and33ms capture pacing are unchanged.

Hidden renderer scheduling: hardware display WebKit views explicitly disable background throttling on supported macOS14+; they must remain live while their Mac windows are hidden. State-script completion over100ms is now logged separately from snapshot timing. A focused hidden-renderer check delivered five DOM updates in16–21ms; this is not a physical display-latency or long-idle acceptance result. Audio/parameter transport and33ms capture pacing are unchanged.

## Knob-to-screen diagnostic trace

`trace_pipeline: true` in keyboard-preferences.json enables a ten-minute detailed window each time the service attaches. It records decoded/accepted input and input-lock wait, computed targets, coalesced RPC batches, adapter parameter sends, section-read request/response and changed confirmed values, state dispatch, completed JavaScript, captured frames and NI-service transfer completion. Parameter commands carry input sequence IDs; state updates carry state IDs and the latest accepted input ID. Coalescing intentionally skips some intermediate turns. Capture and screen-transfer stages share content hashes; those identify pixels, not a physical LCD refresh acknowledgment.

Read the current session with `python3 scripts/trace_keyboard.py`. Logs are `~/Library/Application Support/Modular Machine/keyboard-controller.log` and `adapter-session.log`; the previous three sessions are retained as .1/.2/.3. The trace expires automatically; set trace_pipeline false to disable it on future connections. It does not log keybed MIDI or ROM data. State sequences are snapshots, so latest_input_seq is not proof every individual input was displayed.

### Full-frame delivery trial

`s49_full_frames: true` selects self-contained updates for the S49 NI-service screen path. It is currently **disabled** after graphics-only lag feedback: the trial increased routine transfer cost. Normal updates use partial dirty rectangles; startup and recovery still redraw complete frames. Unchanged frames are suppressed; capture pacing/backpressure and parameter transport are unchanged. The trace records full=true and transfer byte counts. Set false and reconnect the external bridge to restore partial updates. This is a diagnostic trial, not proof that partial commands caused the observed lag.

Graphics pacing: knob activity uses one timer to clear its bold state; it no longer starts a continuous capture burst for every number change. Browser/notice animation is retained. This reduces redundant capture work without changing parameter delivery or the33ms capture limit.
