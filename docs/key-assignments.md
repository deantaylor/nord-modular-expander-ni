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

## Deferred standalone split controls

KEY MODE → two zones; display buttons1–4 choose the left slot A–D,5–8 choose the right slot A–D, with four knobs per display and slot-coloured key lights. **Not implemented; standalone work only.** See [split design](standalone-keyboard-split.md). Existing VST bindings remain the current controls.

## Display adjustment and slot colours

Hold **TRACK + display button 7** to open the existing emulator **Settings** page. Buttons 1–3 select settings sections, and button 4 retains its existing Mac settings action. Merely opening Settings does not alter MIDI channels or other synth values. **TRACK** returns Home.

Hold **TRACK + display button 8** to open **Display**; release TRACK, then use **knob 1** to select **Light / Dark / Colour**. Light is the default. With Colour selected, **knob 4** increases colour intensity (0–100%). **Knob 2** sets **Blobs On/Off** (left = Off, right = On). **Knob 3** adjusts **Glitch intensity** from 0–100%, default 50% (the original effect). Glitch control is unavailable in Colour mode. Above90% the palette briefly alternates Light/Dark and restores; at100% occasional full-screen ALERT flashes show clear/glitched/clear text over three rendered frames, then disappear. Display provides a live preview. These display choices are saved locally; **TRACK** returns Home.

While holding TRACK, display buttons **1–4** show A red, B green, C orange, D yellow: available slots dim, selected slot bright. Display buttons 7/8 are dim while TRACK is held and bright for the active Settings/Display page. Other button assignments remain unchanged.

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

## Current S49 modifiers, lamps and Module Edit — 16 November 2026

- SHIFT is now captured correctly (report01 byte2 mask80) and holds the existing bridge Function state; release/ownership loss clears it. The old byte6 mask04 dial event remains ignored.
- Hold TRACK (byte4 mask10) + display buttons1–4 to select slots A–D in any page. TRACK alone is local and never sends Function. While also holding SHIFT, these slot buttons retain the existing Function slot-pinning behavior. Track-held slot choices replace the buttons' Browser/module actions until release.
- Known mapped lamps are dim when available, bright for active modes/source/slot or while pressed, off when unavailable. Page arrows use independent availability instead of generic macro-active state. Top6–8 stay off. Browser and Track lamps are calibrated; native NI lamps remain NI-owned.
- Module Edit tiles show parameter names/values only. Right screen shows filled envelope/filter graphs with grid/control points and base waveform diagrams for OscA/B/C and fixed sine/triangle/saw slave oscillators. Existing pending feedback remains marked.

The dated sections below retain earlier implementation history; this mapping supersedes their Shift/slot policies.

## Module Edit controls — 15 November2026

- PLUG-IN toggles Module Edit and the existing18-assigned-knob view; dim when available, bright when active.
- In Module Edit, main dial /4D up-down / screen buttons1-2 select modules; page arrows /4D left-right select parameter pages; eight knobs edit the displayed actual module parameters.
- PRESET up/down and BROWSER retain their patch functions. Button5 restores the previous assigned page. Top6-8 stay unassigned; current Track-held slot selection is described above.
- PAGE X/Y counts parameters in Module Edit and remains1/3–3/3 in assigned mode. Voice indicator is unchanged. Native controls stay reserved.

[Module Edit details](module-edit-mode.md).

## Page indicator and proposed Module Edit — 15 November 2026

Existing assigned-knob pages display PAGE1/3,2/3,3/3 on the Nord Modular screen. Voice count remains on the patch screen. No button changes in this update.

Proposed module editing uses a free screen button to toggle a real module/parameter view, with eight encoders paged across that module rather than the eighteen assigned knobs. [Native Edit call-chain and official interface boundary](module-edit-mode.md). PLUG-IN is now bound; do not reinstate native SETUP/SHIFT or slot bindings to implement it.

## Current S49 mapping — 13 November 2026

BROWSER toggles the patch browser; PRESET up/down load previous/next patches. Dedicated page arrows and 4D left/right outside Browser scan knob pages. Eight encoders control the displayed parameters. Direct slot bindings remain removed; Track-held slot selection is described above. PLUG-IN opens Module Edit and Top5–8 stay unassigned. Keybed MIDI remains independent.

At that earlier stage there was no Function/SHIFT binding; the corrected capture above now enables it. The prior byte6 mask0x04 capture was a dial event, not SHIFT, and is ignored. BROWSER remains report0x01 byte5 mask0x04. SETUP is native hardware setup, not synth settings. PLUG-IN, MIXER, transport, Smart Play, SHIFT, MIDI and INSTANCE have no synth assignments. Detected SETUP/MIDI/INSTANCE presses pause our surface until an NI ownership handover back to Encdr. See the [active mapping and native-control policy](komplete-keyboard.md#native-controls-and-conflicts).

Browser closes after confirmed successful patch loading; failed loads and folder navigation keep it open. Available assigned buttons with verified lamp offsets are dim, active source buttons bright, unavailable buttons off. PLUG-IN reflects Module Edit; Top5–8 lamps are off. The BROWSER lamp is calibrated. Historical Maschine mappings below do not apply to the active S49 workflow.

## Maschine MK3 with the official VST

Double-click **Maschine.command** to select Maschine MK3. The background bridge starts when the official G1-Emu VST is available. **KompleteKontrol.command** switches the same service to S49 Mk2.

If another app has the Maschine panel, hold **SHIFT** and press **PLUG-IN** to open the instance selector. Turn the 4D encoder to **Encdr** and press it to select. **SHIFT + CHANNEL** is the hardware's MIDI mode shortcut. [Native Instruments Maschine MK3 quick reference](https://docs.native-instruments.com/ni-tech-manuals/maschine-mk3-manual/en/quick-reference.html)

| Maschine MK3 control | Action |
|---|---|
| Eight display knobs | Edit the parameters shown on the two displays |
| CHANNEL | Return Home. Hold with display buttons 1–4 for slots, 7 for emulator Settings or 8 for Display settings |
| GROUP A–D | Select slot A–D directly |
| BROWSER | Open or close the patch browser |
| 4D encoder in Browser | Rotate to highlight, press to open a folder or load a patch, tilt left to go up a folder |
| < / > | Switch to the previous or next knob page |
| SHIFT + < / > | Load the previous or next patch from the selected browser source |
| PLUG-IN | Toggle Module Edit |
| MIXER | Edit supported per-slot patch output levels |
| SETTINGS | Toggle emulator Settings |
| ARRANGER and display button 5 | Open Sequencers and toggle Edit Sequence. Use the 4D encoder to navigate and edit |
| ERASE | Restore changed parameters to the slot baseline, as with CLEAR on S49 |
| SHIFT | Hold the bridge Function modifier |

Available buttons are dim. The selected slot, open modes, selected tabs and held controls light brightly. Unassigned buttons, including MACRO and PATTERN, stay off.

Performance MIDI uses the DAW's usual routing. The official DirectLink adapter offers patch and parameter control. It has no direct pad-note or master-volume command.

The mappings below document the earlier modified standalone and custom-plugin workflow.

# Key assignments — agreed 11 November 2026

These assignments implement the user's requested layout. Further assignments can be defined later; unused controls stay inactive.

## Layout and controls

| Control | Action |
|---|---|
| Screen encoders 1–4 | Four matching parameter tiles along the bottom of the left screen |
| Screen encoders 5–8 | Four matching parameter tiles along the bottom of the right screen |
| Groups A / B / C / D | G1 slots A / B / C / D |
| Macro | Cycle the three G1 knob pages: 1–8, 9–16, 17–18; return to the parameter view |
| Browser | Open/close the patch browser |
| Main encoder turn in Browser | Highlight a file, folder or stored patch |
| Main encoder press / Top 4 in Browser | Open the folder or load the highlighted patch into the active slot |
| Physical < / > buttons | Immediately load the previous/next preset in the selected browser source |
| Four-way Up / Down in Browser | Move the highlight without loading |
| Four-way Left in Files | Parent folder |
| Four-way Right in Browser | Move the highlight forward |
| Shift, held | The emulator's real Function/Shift modifier; release on button-up |
| Settings | Toggle the real emulator-settings page |
| Mixer | Toggle the master-volume page |
| Pad Mode | Drum layout: pads 1–16 send MIDI notes 36–51 |
| Keyboard | Chromatic layout: pads 1–16 send MIDI notes 48–63 |
| Pads | Strike velocity, one note-on per press; note-off on release |
| Stop (live G1 mode) | Release notes started by the pads; do not exit the controller |

`<` / `>` use the current Files/Banks source. In Files they use `.pch` entries in the current folder and skip directories; in Banks they use nonempty bank entries. They stop at the list boundaries. Browser selection alone never loads a patch.

Group selection is confirmed by the OS before the active-slot light moves. Holding Shift forwards the same row 2 / bit 7 panel input used by G1-Emu's own Shift button, including its slot-selection behavior. Ordinary patch knob movements while held follow the ROM's behavior. Release is retried if the command queue is full; handover, device loss and controller disconnection release the modifier too.

## Settings

Settings is an actual read/write view backed by G1-Emu's host and SynthSettingsLink APIs. It does not use fixture values. The bottom encoders correspond to the controls shown directly above them.

| Settings page | Encoders 1–4, left screen | Encoders 5–8, right screen |
|---|---|---|
| Top 1: Output / Clock / MIDI | Host output gain, clock tempo, master tune, global sync | MIDI channels for slots A–D |
| Top 2: Synth | Clock source, local control, knob mode, minimum velocity | Maximum velocity, program-change receive, program-change send, keyboard mode |
| Top 3: More | Pedal polarity, synth LEDs, unassigned, unassigned | Unassigned |

Top 4 opens **the existing G1-Emu Emulator Settings card on the Mac**, including its ROM, audio-device and MIDI-device selectors. The hardware page shows the current model, ROM filename, audio status and DirectLink state. This reuses the emulator's real UI; no replacement patch editor is provided.

Host gain uses the same 0–60 dB range as the existing Settings UI. Synth tempo/tune ranges and global-sync formatting follow that UI. Values update from confirmed readback; pending changes are indicated. Synth-setting persistence follows the existing G1 OS/Store behavior, rather than a separate controller patch format.

## Mixer

The Mixer page displays the real master-volume ADC position as a percentage, plus actual output information. Encoder 1 controls master volume; encoder 2 controls the emulator's host output gain. Other encoders are unassigned. Master volume and host gain are different controls; both stay in G1-Emu. The standalone keeps its master knob position using its existing close/save behavior.

## Browser and remaining buttons

While Browser is open, Top 1 selects Files, Top 2 selects Banks, Top 3 refreshes, and Top 4 opens/loads. Settings uses Top 1–3 for its three settings pages and Top 4 for the Mac settings card. Outside these pages, the top buttons have no new assignments. Top 5–8 no longer select slots.

Groups E–H and Play have no G1 action. STOP releases tracked pad notes in live G1 mode, and still exits the separate hardware-test mode. Normal controller exit remains terminal `quit` or Ctrl+C. Scales, octave controls, sustain, note repeat and further performance assignments remain for later definition.

## Button lights

- Active slot: bright green group button. Other OS-selected slots: dim green. Unselected groups: off.
- Current mode: Browser, Settings, Mixer or Macro lit. Other mode buttons: off.
- Shift: lit while the bridge confirms Function held.
- Page/source buttons: the selected tab is lit. Available refresh/load/settings/preset actions have a lower light level; unavailable actions are off.
- Pad Mode or Keyboard lights for the selected layout; held pads light after bridge confirmation. STOP is available while notes are held. Unassigned controls are cleared. An unavailable emulator clears the G1 lights; NI ownership handover pauses our hardware output until ownership returns.

The state sent to the LED driver and screen layout are automatically checked. Physical brightness, alignment and the new Settings/Function interaction still need normal use on Maschine; earlier physical control and bank-loading passes remain accepted.

## Visual direction — Inter screens

The user's preliminary screen designs now drive the live G1 UI: #141414 background, Inter headings, large light parameter numbers, smaller baseline-aligned units, four flat columns per display and minimal chrome. The left header shows slot / active patch; the right header shows Nord Modular. The top-right indicator shows allocated voices (requested voices), read from the real emulator; unavailable counts show dashes. Encoder slices remain tied to their working endpoints. On hardware the patch/browser and Nord Modular presentation roles are swapped independently, so the first physical screen has the patch/browser without changing knob alignment.

Browser uses a full-height left list, with the highlighted candidate occupying the main patch-name position and subdued neighbors below it. The selected row moves into the main-name position; the list slides vertically over 180ms and each row fades with distance from selection and retargets during rapid turns; merely scrolling still does not load. Clicking the main encoder loads the candidate; closing Browser restores the loaded patch name. Knob controls are hidden on both screens in Browser mode. Opening/closing Browser restores the corresponding display layout. Settings/Mixer share the same type and bottom-control styling. Errors/loading feedback stays available without a permanent footer. Unknown/unassigned values stay dim rather than displaying example values from the reference.

Inter is bundled unmodified under OFL1.1 in screens/fonts, embedded for offline use. List positions and opacity advance explicitly on the native snapshot clock, avoiding offscreen compositor-layer disappearance. Captures coalesce to a maximum of 30fps. Changing values briefly use Inter weight 400 instead of 150; there is no glow. Unchanged per-screen output skips redundant captures; reduced-motion preference disables the transition. No key assignments changed with this visual revision.

Module and parameter names now occupy separate label lines above each value, preserving the value baseline and physical encoder alignment. Activity uses weight400 briefly; there is no glow. Hardware feel and brightness still require user acceptance.

## MIDI performance and encoder rate

Pad numbering follows the physical labels: pad 1 bottom-left, pad 4 bottom-right, pad 13 top-left, pad 16 top-right. Pad Mode starts at MIDI 36 (standard drum note range); Keyboard starts at 48 and proceeds chromatically. Scales will be added later. Default startup is Pad Mode. These layouts send notes to the selected slot’s configured MIDI channel through G1-Emu’s existing MIDI IN routing, including its own layering/octave settings. A channel set to Off must be enabled in Settings; the controller never silently changes it. Drum sounds depend on the loaded G1 patch.

Velocity uses a square-root curve over the normalized strike pressure, clamped 1–127. Subsequent pressure updates do not retrigger or send aftertouch. Pad-up sends NoteOff using the original channel/note; switching Pad Mode/Keyboard, STOP, device loss/handover and socket loss release tracked notes. STOP sends only releases for tracked pad notes, not a global CC panic. A separate panic signal bypasses a full ordinary command queue.

Patch and Settings/Mixer encoders now have 50% of their previous sensitivity, including accumulated fractional movements. Preset-browser scrolling is unchanged. Latest targets coalesce into paced writes (minimum 16ms between batches); status requests also have a 16ms minimum interval. This limits traffic during fast turns; it is not a claim that message volume was the sole crash cause.

## DAW plugin mode

The same hardware mapping applies to the selected plugin instance. Mixer gain/master controls reconcile with host parameters, and active slot/settings/patch state belongs to that plugin. The native Settings action requires its DAW editor to be open. Select the instance with `--plugin --port PORT`; see [plugin guide](plugin-guide.md). Direct bridge pad notes play the plugin; DAW clip recording for those notes is a separate routing milestone.

## Komplete Kontrol S49 Mk2 alternative

The user requested the connected keyboard as another interface. `--controller komplete` selects its eight encoders and dual screens; PLUG-IN/pages navigate knob pages, PRESET buttons switch patches, 4D press opens Browser/confirms loading, and top 5–8 select slots A–D. See [keyboard setup and full mapping](komplete-keyboard.md). The existing descriptor does not establish Browse/Shift/Mixer button inputs; no guessed bits are added. Keys go through the existing MIDI input/DAW track, independently of panel controls.
