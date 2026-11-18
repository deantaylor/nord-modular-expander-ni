## 18 November 2026 — Maschine MK3

- Add Maschine MK3 as a selectable surface for the official G1-Emu VST background service alongside the S49 Mk2.
- Map CHANNEL to Home, PLUG-IN to Module Edit and ERASE to slot parameter restore. Keep direct slot selection on GROUP A–D.
- On Maschine, use the arrow buttons for knob pages, SHIFT + arrows for presets and ARRANGER for Sequencers.
- Use Maschine’s native dim and bright lamp values. Available controls are dim, and active or held controls brighten. Disable MACRO and clear its lamp.

## 17 November 2026 — Bridge performance pass

- Retain sequencer cells for playhead-only updates; update dot classes instead of rebuilding the grid.
- Read button LEDs without allocating full screen state; copy connection state directly rather than creating a development-panel snapshot.
- Add an isolated repeatable benchmark. Median LED work: 188 ms → 3.2 ms per 2,000 reads; sequencer scripting/layout: 103 ms → 23 ms per 500 updates. These are component timings, not physical display latency.

## 17 November 2026 — Sequencer lock and quiet UI

- Light PATTERN only for patches containing supported sequencers, using a read-only catalogue check.
- Require Edit Sequence for right-screen navigation/editing; disabling it returns focus to the sequencer list.
- Apply the outlined/filled header button, blank title bar and counter. Match Module Editor list styling while retaining parameters and graphs.
- Remove help, waiting and success text from screens; retain errors and actual patch/parameter data.
- Verify isolated wheel UP/DOWN produces HID controls but no MIDI messages on physical or NI DAW ports; preserve routing.

## 17 November 2026 — Sequencer circles and live playhead

- Show circles for every step, with filled/outlined event on/off states and a red live-playhead marker separate from edit selection.
- Forward the official editor meter stream for all four sequencer types using the pinned module-layout mapping; no estimated clock, VST modification or extra polling requests.

## 17 November 2026 — Wheel step browsing

- Add separate sequencer-list and step-browsing focus: tilt right to browse steps with wheel rotation, tilt left to return to the list. Display5 locks rotation to numeric editing; switching it off returns to step browsing. Event rotation browses steps and centre press toggles them.

## 17 November 2026 — PATTERN sequencer page

- Select a sequencer on the left display and toggle Sequence Edit with display button5. Wheel tilts navigate steps/rows; rotation edits note/value steps; centre press toggles events.
- Support NoteSeqA/B, CtrlSeq and both EventSeq rows through existing parameter writes, including CLEAR restoration and safe page exits.
- Calibrate PATTERN input/lamp and PRESET UP/DOWN lamps; keep unused controls off.

## 17 November 2026 — CLEAR availability

- Keep CLEAR unlit and inactive for unchanged slots; enable it only for parameter changes and switch it off after confirmed restoration.

## 17 November 2026 — Restore changed parameters only

- CLEAR skips unchanged parameters and sends no writes for a clean slot. Pending/unconfirmed writes remain covered; readback is limited to affected sections.

## 17 November 2026 — Restore slot parameter values with CLEAR

- Capture immutable per-slot parameter baselines on confirmed loads; CLEAR restores assigned-knob and Module Editor values without reloading the patch.
- Pace restore writes, block interleaved edits, reject stale identities, and confirm values before reporting completion. Dim/active CLEAR lamp follows availability/progress.

## 17 November 2026 — Deferred standalone split design

- Document S49 KEY MODE split, independent display/slot controls and slot-coloured Light Guide for future standalone software. No split implementation or VST MIDI-routing change applied.

## 17 November 2026 — Consistent page titles

- Patch Browser now uses the same header typography and title placement as Module Editor.

## 17 November 2026 — Module Editor title and corners

Module Editor title now replaces voice count in the left header; right header keeps page count without duplicated title. Rectangular header badges, parameter labels and source tabs use2px corners; numeric values/progress bars unchanged. Slot badge isolated with theme-colour surround to expose its rounded corners. Module graph clipping matches2px panel radius. Release build and existing hidden-WebKit module fixture passed; both ADSR screen captures visually inspected. Evidence /tmp/module-editor-title-webkit.log and g1-state/module-visuals-1794910634167419000.

## 17 November 2026 — Smaller logo and rare saver messages

Eastlab artwork scaled to60% on both axes, centred on unchanged480×272/#141414 frame; source render script uses288×144 at96,64. At full glitch intensity, idle saver adds FEED ME/CONFORM on shared40s epochs (ALERT remains8s). Each new message is static2s, suppressing palette jumps, then glitches180ms and disappears. Messages only appear during saver; waking cancels them. Wall-clock seeded schedule keeps displays synchronized. Pure-JS timing/rarity/word/threshold checks passed (/tmp/eastlab-rare-message-test.log); logo preview visually checked.

## 17 November 2026 — Eastlab loading logo

Use the supplied Eastlab SVG on #141414, rasterized to480×272, for S49 startup and the default exit screen. Blob idle screensaver unchanged. Original Encdr asset and attribution retained. Source screens/assets/eastlab-logo-dark.svg; regenerate with python3 scripts/render_loading_logo.py (macOS Quick Look/Pillow). PNG dimensions/background/visible artwork verified.

## 17 November 2026 — Keyboard instance name

- Rename the S49 Mk2 instance from Encdr to **Nord Modular**.

## 17 November 2026 — Display controls and idle screensaver

Display: Theme1, Blobs2, Glitch intensity3 (Light/Dark only), Colour intensity4 (Colour only). Temporary theme glitches alternate Light/Dark. After300s idle, native state enters screensaver, hides UI with700ms fade and forces blob rendering even if disabled. Effective intensity ramps from saved level to1 over3600s; saved preferences and underlying page remain untouched. Colour saver uses Dark temporarily. Raw owned S49 surface events (including touch/unmapped buttons) and passive channel MIDI activity wake it; held notes prevent idle. No MIDI routing, plugin or NI ownership changes. Mock `idle-for SECONDS` accelerates isolated tests.

## Glitch intensity and extreme effects — 16 November 2026

- Replace the Glitches switch with a 0–100% intensity control on Display knob4. Default50% reproduces the existing glitch behaviour;0% disables it, higher levels increase frequency, duration and displacement. Persist the level; migrate old Off to0 and On to50.
- Add a live Display preview so Blobs switching and small intensity movements give immediate visible feedback. Knob3 still controls Blobs.
- Above90%, temporarily jump among the other themes and automatically restore the selected theme, without changing the saved choice.
- At100%, occasionally flash ALERT across both screens for three rendered frames: clear text, glitched text, clear text, then hidden. Broaden the100% frame guard to accept valid centre-text flashes while still rejecting blank frames.

## Blob and glitch switches — 16 November 2026

- Add Blobs On/Off on Display knob3 and Glitches On/Off on knob4, both enabled by default. Turn left for Off, right for On.
- Persist both switches with the local display preferences. Hiding blobs cancels animation/capture leases on both screens; disabling glitches keeps smooth blob motion and RGB fringes.

## Note-held animation and occasional glitches — 16 November 2026

- Keep blob expansion/intensity while notes are held; start a smooth release after the final note-off. Handle polyphony, MIDI channels, velocity-zero note-offs, sustain pedal and All Notes Off/reset messages. Retain the phrase's peak velocity/note count while held.
- Add occasional short horizontal tears, dot dropouts and stronger RGB separation, synchronized across both screens. Idle breathing continues; controls and labels do not glitch.

## Exact VST parameter names — 16 November 2026

- Use the verified local rack OS 3.03 short-name table for assigned knobs and Module Edit, preserving spelling, abbreviations and case exactly as the official VST does. Generate only a private local label cache during installation; no ROM is bundled or modified. Unsupported images or unmatched parameter ranges retain editor metadata rather than guessed capitalisation.

## Stronger RGB separation — 16 November 2026

- Widen RGB separation to 1.2px at idle and up to 2.5px on MIDI activity, increase channel opacity, and give green its own vertical offset. Keep shaded grey cores and the existing frame cap. Captured pixels verify visible colour separation.

## Grey depth shading — 16 November 2026

- Replace pure-white blob cores with six grey tones, using the field depth and soft upper-left lighting for a brighter centre and darker edges. Adapt contrast for Light/Colour versus Dark themes; retain RGB fringes and the unfaded control areas.

## Animation fade preview — 16 November 2026

- Remove the animation attenuation behind headers and knob controls. Keep the organic edge falloff and RGB splitting.

## Subtle RGB splitting — 16 November 2026

- Add soft red/green/blue fringes to the animated dots, with slightly wider separation during MIDI activity. Preserve the original dot core, readable controls and existing frame cap.

## MIDI-reactive breathing dots — 16 November 2026

- Add the supplied halftone-dot style across both Home screens: slow organic idle breathing, velocity-driven expansion and movement on S49 notes, and a soft release back to idle. Theme colours follow Light/Dark/Colour.
- Use a passive S49 Mk2 performance-port MIDI subscriber; keybed/DAW routing remains unchanged. This monitors keyboard notes, not MIDI clips inside the VST, audio levels or allocated voices.
- Run only on Home, drive canvas redraws from the native capture clock at20fps and coalesce captures. Stop animation timers on module/browser/mixer/settings pages; reduced-motion mode stays static.

## Display knob and module graph update — 16 November 2026

- Put Theme on Display knob1 and Colour intensity on knob2; labels match the physical controls.
- Expand module graphs to a 464×132 pane with 8px side insets. Remove graph captions, summaries, legends and axis labels, retaining accessible descriptions and pending-value indicators.
- Show FilterE type as LP/BP/HP/BR and slope as 12/24 dB; FilterF slope as 12/18/24 dB in module parameter controls, including pending edits.

## Light / Dark / Colour themes and Settings hotkey — 16 November 2026

- Restore access to emulator Settings with TRACK + display button 7; opening it does not change MIDI assignments or any synth setting.
- Add Light and Dark themes matching the supplied references, alongside the existing slot-coloured Colour theme. Light is the default for new/older preferences without a theme.
- TRACK + display button 8 opens Display: knob2 selects Light/Dark/Colour; knob1 adjusts intensity only in Colour. Persist theme with the existing local preferences. TRACK returns Home.

## Saturated-screen recovery fix — 16 November 2026

- Fix screens remaining on Display after TRACK: Home state changed correctly, but capture validation rejected saturated red/green frames and repeatedly reloaded the views. Accept bright ink in any RGB channel while continuing to reject dark blank frames.

## Colour intensity correction — 16 November 2026

- Replace UI brightness with Colour intensity on TRACK+8: knob1 enriches the supplied colours from their original pastel appearance toward saturated hues, keeping the brightest channel and white tracks unchanged. Old brightness preferences are ignored; colour intensity defaults to the original design.
- Reduce all S49 display-button LED intensities: available/active now use palette intensity 0/2 instead of 1/3, retaining the selected-slot distinction and slot hues.

## Slot lamps and display brightness — 16 November 2026

- TRACK slot buttons 1–4 use the S49 Mk2 indexed red/green/orange/yellow palette, dim when selectable and bright for the confirmed selected slot.
- Hold TRACK + display button 8 to open Display. Knob 1 adjusts UI brightness from 35–100% on both screens; TRACK returns Home. Brightness scales the slot colours and white value tracks, and is saved locally when leaving the page or exiting the bridge. Existing preferences default to 100%.

## Slot colours — 16 November 2026

- Apply selected-slot themes to both screens: A red (`#ff8082`), B green (`#ebffdb`), C existing orange (`#ffca87`), D yellow (`#fff39b`). Headers, labels and module diagrams share the theme; Mixer screen2 remains empty and follows the background colour.

## Right-screen page badge — 16 November 2026

- Match the supplied header: separate black page badge at top right, formatted `1 / 3`, with a peach gap from the main header strip. Page state continues to follow the active knob page.

## Screen design refresh — 16 November 2026

- Apply the supplied peach/black design to Home and Browser: compact slot/patch header, single-line parameter labels, thin numeric values with units, and normalized value bars aligned to physical knobs.
- Browser uses a vertically scrolling, fading list and a sparse Files/Synth source indicator on screen2. Existing button assignments remain unchanged (Files/Synth correspond to Files/stored banks).
- Extend the same theme to module diagrams, Settings and Mixer; Mixer screen2 stays empty. Voice/page counters and explicit pending-value marks remain. All slots initially share the supplied peach colour pending the remaining palette.

## Exclusive pages and TRACK Home — 2026-11-16

- Browser no longer repeats the selected patch name on screen2.
- Opening another page exits the old page; Browser/Mixer close to assigned-knob Home.
- TRACK returns Home from any bridge page while retaining held slot selection.
- Clear obsolete page targets and retry backend cleanup safely on queue pressure. Focused state/hidden-WebKit checks pass.

## Screen headers and completely blank Mixer screen2 — 2026-11-16

- Move slot/patch and voice count to screen1; page counter to screen2. Remove Nord Modular branding.
- Keep Mixer screen2 blank even during modifiers/notices; allow intentional blank hardware captures instead of retaining the previous page.
- Focused hidden-WebKit header/Mixer checks and release build pass.

## Controls-only Mixer and stalled-readback fix — 2026-11-16

- Remove the duplicate descriptions above Mixer knobs and the entire right-hand Master section.
- Avoid redrawing the blank display for knob changes on the other screen.
- Give active edits priority over stalled inactive-slot reads; bound retries and discard inactive work on Mixer exit.
- Keep failed slot data unavailable pending a fresh read, with PID diagnostics instead of an infinite retry loop.
- Protected real-VST fault test verifies active feedback despite a lost inactive response; screen checks pass.

## Mixer input recovery and simpler layout — 2026-11-16

- Fix a reproduced lost knob update after inactive-slot replacement by using per-slot confirmed patch IDs for parameter writes/readback.
- Remove Mixer’s background full-patch fetch loop; give slot activation priority over reads/writes. Restore normal/module knobs on mode changes.
- Map A–D to knobs1–4 and show all slots on the left; remove meter-style bars and controls for ambiguous/missing Outputs.
- Place deferred Nord master section on the right. User chose unchanged official VST; no DAW-volume substitution.
- Expanded real official-VST regression and screen/state checks pass; shared banks unchanged.

## Physical MIXER page and slot Output levels — 2026-11-16

- Bind captured S49 Mk2 MIXER input byte5/01 and calibrated lamp37 to the mixer page.
- Knobs1/3/5/7 adjust A/B/C/D Output-module levels, with actual slot-bound readback and marked pending feedback. Preserve focus; disable ambiguous/missing outputs.
- Show A/B left, C/D right, patch names and level-setting bars. Host master/final output VU are unavailable through this interface; use DAW metering.
- Focused controller/state/WebKit and protected real official-VST four-slot edits/guards pass; banks unchanged.

## Active-slot feedback and uncached patch recovery — 2026-11-16

- TRACK slot lamps indicate confirmed focus only, independently of enabled/pinned slots or button holds.
- Keep the active slot visible on screens while its patch data loads; show its letter in TRACK hints.
- Fetch missing metadata after settings-only slot confirmation; allow returning to another slot during loading, retaining edit guards.
- Verified S49 Mk2 PID1610 profile. Focused state/WebKit checks and official-VST focus+metadata round trips pass; banks unchanged.

## Slot selection confirmation and native Shift guard — 2026-11-16

- Fix TRACK+1 returning to A while another selection is pending; clear stale selection status after confirmation.
- Space mask/activation writes by300ms as the official emulator does, rather than sending back-to-back or waiting globally for a reply the ROM does not send. Keep authoritative active-slot readback and bounded confirmation.
- Reserve native SHIFT+PLUG-IN MIDI-mode switching; suppress simultaneous Module Edit and respect NI ownership.
- Protected official-VST B/A,C/A,D/A switches pass in624–810ms; shared bank unchanged. Focused controller/client tests and builds pass.

## Physically calibrated S49 LED map — 2026-11-16

- Correct indexed display-button outputs2–9; use white palette dim69/bright71, eliminating green caused by monochrome values.
- Correct measured SHIFT14,TRACK23,page arrows32/33,BROWSER35 and PLUG-IN36. Move Module Edit to PLUG-IN; unused controls off.
- Preserve preset actions; their lamps remain uncalibrated/off. Restore normal controls after ordered input and isolated lamp capture. Packet regressions/release build pass; no VST/DAW restart.

## Correct S49 lamp names and move Module Edit — 2026-11-16

- Move Module Edit to physical PLUG-IN, verified by isolated input capture; unassign/clear display button5.
- Correct vendor Encdr LED descriptor: payload23 is observed TRACK and28 is KEY MODE. Remove wrong page-arrow output names so paging cannot light unrelated controls. Arrow lamps remain unprogrammed pending verified locations; keep input paging functional. No speculative global offset/brightness changes.

## S49 modifiers, contextual lamps and module graphs — 2026-11-16

- Capture and bind real SHIFT (byte2/80) and TRACK (byte4/10); Track+display1–4 selects slots, independently of Function. Preserve existing Shift/Function slot pinning with both modifiers. Release/ownership cleanup prevents stuck modifiers.
- Give page arrows independent availability and momentary brightness; use dim available/bright active/off unavailable lamps. Browser/Track lamp offsets remain unverified.
- Simplify Module Edit tiles to parameter names/values. Add filled cyan/green envelope/filter graphs with nodes/grid and parameter-driven OscA/B/C/fixed slave waveforms; retain immediate marked pending feedback and existing capture policy.

## Immediate pending encoder feedback — 2026-11-16

- Show requested encoder values/curves before engine readback; format accepted numbers using upstream descriptors. Mark previews with a dot/bold value and PENDING on diagrams.
- Keep confirmed state separate; reconcile confirmation and clear previews on rejection, timeout, changed binding/patch/module or disconnect. Preserve audio delivery and capture/transfer scheduling.

## Reduce screen transfer and capture overhead — 2026-11-15

- Disable the S49 full-frame trial after screen-only lag feedback; restore partial changed-region updates while retaining startup/recovery redraws.
- Replace numeric activity’s repeated capture burst with one timer that clears bold activity. Preserve browser/notice animations and diagram updates.
- Focused curve/WebKit/activity checks and release build pass. Audio/parameter delivery unchanged; physical response still needs comparison.

## ADSR and filter module diagrams — 2026-11-15

- Draw ADSR/Mod-Env and Filter A–F schematics on the physical right screen in Module Edit; preserve parameter controls and left-side module navigation.
- Reflect confirmed values across all parameter pages, including shape/invert, modes/slopes/resonance/gain and bypass. Multimode filters show separate LP/BP/HP lines. No continuous graph animation or additional engine polling.
- Reuse attributed GPLv3 NME envelope/filter geometry; include notices/license. Curves show base parameters, not measured DSP or live modulation.
- Focused curve/data/hidden-WebKit tests, screenshot review and release build pass. Physical LCD/feel remains for user assessment.

## Module Edit wired — 2026-11-15

- Screen button5 toggles module-parameter editing. Dial/top1-2/up-down select modules; arrows page parameters; eight encoders edit actual module values. Previous assigned page is restored on exit; saved knob assignments, preset/browser and key MIDI remain independent.
- Add lazy module catalogue, selected-module status and validated/coalesced direct parameter targets bound to patch generation/section/module/parameter. Exclude unsupported/custom/morph controls; handle empty modules and stale selections.
- Show module context and per-module page counts; use dim/active button lights.
- Focused module/profile tests and protected official-VST/controller/WebKit check pass, including unassigned-parameter readback and unchanged bank. Physical hardware entry/feel remains for acceptance; diagrams are not part of this change.

## Knob page numbers and native Edit assessment — 2026-11-15

- Show PAGE1/3–3/3 on the Nord Modular screen, preserving the voice indicator and hiding page numbers outside the parameter view. Ensure identical/empty page changes redraw.
- Trace native Edit-button/OS module editing and document a keyboard module-parameter counterpart plus current official-interface limitations. Module mode is not wired yet.
- Focused page-label/signature check and release build pass.

## S49 full-frame delivery diagnostic — 2026-11-13

- User-reproduced trace shows prompt parameter/readback/JS/host transfers; physical scanout remains unobserved. Preserve trace and enable a reversible full-frame S49 service trial.
- Suppress identical full-only frames across periodic keyframe boundaries; trace full/byte counts. Other devices, raw USB and sound/control paths unchanged.
- Focused screen-manager lifecycle check and release build pass. Partial-update failure is a hypothesis requiring physical comparison.

## Knob and screen pipeline trace — 2026-11-13

- Add opt-in timestamped stage tracing from decoded knob events to host screen-transfer completion, including input/target/state sequences, engine readback and frame content hashes. Preserve coalescing semantics and distinguish transfer completion from physical scanout.
- Enable ten-minute trace windows through service preferences, preserve3 prior log sessions, and add read-only scripts/trace_keyboard.py summaries.
- Focused Rust/WebKit/service checks and release/native builds pass; live stage records verified. Trace enabled for user reproduction; no crash diagnosis claimed yet.

## Keep hidden hardware renderers active — 2026-11-13

- Disable WebKit background throttling for hardware display views on supported macOS; default hidden-view throttling could delay/suspend rendering.
- Log slow state-script completion separately from snapshot timing. Hidden real-WebKit value-update smoke check16–21ms; release build passes. Long-idle and physical panel response remain unverified.
- Parameter/audio transport and33ms capture interval unchanged.

## Latest confirmed value on keyboard screens — 2026-11-13

- Remove the display-only cached value/direction filter that could hide new readbacks while continuous turns renewed pending edits. Render latest authoritative engine values during reversals.
- Retain pending input accumulation/confirmation and hide unconfirmed targets. Two focused state tests and release build pass; adapter/audio transport and33ms capture rate unchanged. Physical feel remains for user assessment.

## Mixed preset/control transfer follow-up — 2026-11-13

- Defer file uploads until in-flight parameter reads/SDK work drain, clear old parameter state, and reject unsupported modules before accepting a busy operation.
- Retry unknown startup settings independently without the global reply lock. Add rate-limited read/fetch/upload wait diagnostics.
-24 protected mixed edit/preset loads pass, maximum493ms, bank unchanged. Remaining physical inconsistency is not declared resolved.
- Isolated plugin tests now hold the launcher lock or refuse while production owns it, preventing watcher attachment to a test host.

## Four-second knob stall reproduced — 2026-11-13

- Background parameter readback used NME’s global3000ms reply lock, blocking knob writes when replies were missing. Use the adapter’s existing read deadline/assembly independently and remove automatic2s settings polls; retain startup/explicit refresh and full-patch safeguards.
- Add a test-only dropped-reply relay. Old adapter emitted1 parameter frame over4s; fixed adapter136 frames with maxgap59ms and confirmed recovery. Shared bank unchanged; native build passes. Screen pacing stays33ms.

## S49 responsiveness regression correction — 2026-11-13

- Restore33ms capture and NME’s supported parameter sender after the user reported worse physical response.
- Give coalesced parameter writes priority over background confirmation reads; remove the adapter’s full-section write gate, retain full patch/protocol guards, and cap section-read starts at25/s.
- Clear completed knob confirmation notices and report expiry instead of leaving “awaiting G1” indefinitely.
- Actual Defunkt continuous/reversing check and two focused status tests pass; shared bank unchanged, release/native builds pass. Hardware responsiveness remains unverified; earlier fixture timings did not establish it.

## S49 standalone latency fixes carried forward — 2026-11-13

- Use the standalone's captured direct parameter framing without the generic sender's extra focus/checksum, with first readback after16ms.
- Replace repeated timed AppKit pumping in the external adapter with a persistent native CF loop and coalesced wake-up; preserve main-thread NME ownership and clean shutdown.
- Reduce S49 capture interval33→16ms; retain one capture in flight, latest-frame coalescing, recovery and backpressure.
- Protected old/new official-VST measurement: median confirmation60→32ms, max87→36ms; continuous/reversing movement passes and shared bank unchanged. Fix timing probe's unbuffered socket overhead. Rust release and adapter builds pass. Physical panel latency remains unmeasured.

## S49 screen response and browser controls — 2026-11-13

- Fix confirmed-value starvation during sustained encoder movement: subsequent writes no longer postpone targeted readback; due reads have priority.
- Remove top5–8 slot actions and clear their lamps. Close Browser after confirmed successful file/bank loads; retain it on failure or folder navigation.
- Use NI Dim/Bright encodings for available/active assigned buttons; dim inactive Browser sources and available page/preset/load actions. Browser lamp offset remains unknown.
- Eight focused control/browser tests and release/native builds pass. Protected official-VST regression:144 writes,72 confirmed changes during movement, maximum confirmation gap81ms; shared bank unchanged. This is engine readback evidence, not physical scanout timing.
- Correct official-plugin test isolation: alpha.30 ignores legacy G1_DATA_DIR, so macOS enforces shared flash/settings filesystem protection. Fix a test counter shadowing the checksum variable, which caused false bank-change failures.

## S49 native-control correction — 2026-11-13

- Remove the incorrect dial-to-Function binding; actual SHIFT remains native/unassigned. Remove SETUP-to-synth-settings and PLUG-IN-to-page shortcuts.
- Allow only explicit S49 controls, limit application LED updates to assigned lamps, and pause surface actions/output on detected SETUP/MIDI/INSTANCE until NI hands ownership back to Encdr.
- Document native/DAW controls and conditional panel ownership using NI and Encdr documentation. Preserve key MIDI and NI services.
- Six focused controller tests and the release build pass. Physical native-menu/DAW coexistence remains for user acceptance.

## S49 buttons and responsiveness — 2026-11-13

- Capture/map physical BROWSER and SHIFT. Preserve PRESET up/down as previous/next patches, dedicated arrows as knob pages, and add 4D left/right page navigation outside Browser.
- Implement Function-held parameter gating and shifted slot pinning through supported external controls, with release on disconnect; no VST or key-MIDI changes.
- Fix unready status metadata and ordinary rejection handling that caused controller reconnects. Stop unsolicited bank scans in Files mode and prioritize parameter writes over bank listing.
- Focused button/Function, paced encoder/lazy-bank and real official-VST checks pass; shared bank file unchanged. Physical latency remains for user assessment.

## Official VST / keyboard-only migration — 2026-11-13

- Replaced the custom-plugin dependency with an external adapter using official G1-Emu DirectLink and unchanged NME protocol/model code. Installed the verified official alpha.30 VST3 instrument/FX alongside legacy builds.
- Read actual patch/knob/voice data, coalesce parameter writes, confirm values through targeted readback and reject stale bindings. Incomplete patch downloads do not replace confirmed state or count as successful loads.
- Added a single-user background watcher, hidden/Dock-free controller mode, plugin discovery/cleanup, staged service installation and dedicated diagnostics. Key MIDI and the DAW's synth remain independent.
- Keyboard is now the default development target. Existing custom-plugin/Maschine artifacts are retained as historical recovery material.

## S49 plugin launch clarification — 2026-11-12

- Added Komplete Keyboard Plugin.command for direct S49/VST attachment and clarified that opening the plugin alone does not start the external controller. Existing launchers retain their standalone/Maschine defaults.

## 0.1.0 Komplete keyboard integration — 2026-11-12

- Added an explicit S49 Mk2 controller profile and launcher, reusing G1 standalone/plugin state and screens while preserving default Maschine routing.
- Captured actual native keyboard reports and corrected knob byte positions/decoding; added captured-report, profile and navigation regressions. Documented standalone Keyboard selection and ordinary DAW MIDI recording.
- Verified NI-service connection/control and screen transfers; user confirmed corrected physical knob feedback. Keys remain decoupled, using the existing independent MIDI route; additional button/long-session acceptance remains.

# Changelog

## 0.1.0 DAW plugin versions — 2026-11-12

- Built upstream G1-Emu instrument/FX targets as locally branded Apple Silicon VST3 and AU, with separate plugin identities and per-instance Maschine ports. The DAW retains audio, transport and project state.
- Shared the controller bridge through a plugin adapter using existing lifecycle/engine locks off the audio callback. Added host-parameter reconciliation, MIDI routing/release cleanup, generation-bound targets, state-cache refresh and native Settings port display.
- Added plugin-only launcher/discovery, installer and packaging; plugin launch never starts the standalone and ambiguous instances require explicit selection. Added G1_DATA_DIR for isolated verification.
- Verified two-instance VST3 controls/parameters, MIDI/audio, pause/resume, state save/restore, stale-target rejection and FX audio input; both installed AUs pass auval. Ten launcher tests pass. Native DAW user acceptance and direct-pad clip recording remain separate.

## 0.1.0 captured display/LED freeze — 2026-11-12

- Captured a live main-thread stall in repeated Tao/AppKit event-loop entry while pads remained active. Replaced frame-by-frame run_return calls with one persistent AppKit loop, scheduled ticks and autorelease pools.
- Create views before loop entry and bind them on device discovery; preserve capture guards on retarget. Make preview captures asynchronous and gate initial mock interaction on page readiness.
- Passed hidden UI/shutdown, snapshot/fault recovery, MIDI/audio, page/preview and 39 unit checks. Restarted only the controller against the same live emulator and observed continued heartbeats/captures beyond the previous freeze interval.

## 0.1.0 responsiveness follow-up — 2026-11-12

- Compared the requested Maschine MD-MM native input/engine/render separation. Moved MK3 live input dispatch off the WebKit loop and native bridge I/O off the JUCE UI timer; synth access stays on the bounded engine queue.
- Preserved ownership/release behavior, made native Settings callback access thread-safe, and coalesced host gain saves away from the engine thread. Avoided repeated catalogue copies/polling and idle LED snapshot construction.
- Added hardware-dispatch and deliberately stalled-UI regressions; separated direct readback from desktop feedback timing and removed 100ms timing-probe polling artifacts. WebKit and physical display latency remain separate acceptance concerns.

## 0.1.0 MIDI and fast-turn follow-up — 2026-11-12

- Halved patch and Settings/Mixer encoder sensitivity. Paced/coalesced latest encoder/control targets and status requests to reduce fast-turn traffic. A10,001-target transport stress test confirmed the final target in four native batches.
- Added Pad Mode drum notes 36–51 and Keyboard chromatic notes 48–63, with strike velocity and physical pad numbering. Scales remain deferred. Reuses EmuHost::queueMidiIn and actual slot MIDI channels; no additional MIDI daemon.
- Added owned-note release on pad-up, mode changes, STOP, device/client loss and queue pressure, retaining the original note/channel. Confirmed pad/mode lights and a project-owned gated MIDI test patch.
- Real-ROM tests verified MIDI bytes, duplicate suppression, original-channel releases, mode changes, gate/silence/pitch audio and 800 fast encoder reversals. Physical acceptance remains separate.

## 0.1.0 completion work — 2026-11-12

- Hardened native request execution with a bounded engine-thread queue, client lifetime guards, atomic running state and patch identity checks. Bank loads now wait for their matching firmware outcome.
- Retained one live HID API owner across reconnects; added missing-header capture rejection and reload/latest-state replay after renderer faults. The intermittent freeze is not definitively reproduced or claimed eliminated.
- Preserved relative encoder accumulation, improved low-range sensitivity, added two-line labels and kept activity bold without glow. Both control strips hide in Browser.
- Added clean-exit folder/source/selection restoration, numeric library sorting, rotating controller health logs and a versioned local app/source packaging path that excludes ROM and user patch/bank data.
- Verified 35 Rust tests, strict Clippy, real-ROM file/bank outcomes, reversal stress and injected blank/JavaScript recovery. Local package signatures/digests, hidden real-engine launcher, offline Cargo resolution and native source configuration passed. Physical feel/latest-layout acceptance remains pending; external editor testing is on hold.

## 0.1.0 development — 2026-11-11

- Preserved initial source under docs/baseline; pinned resolved Encdr dependencies to 0.8.5.
- Fixed initial JSON macro compilation failure and asynchronous screen polling.
- Added hardware-free dual-window macOS mock, shared state controls, independent slot values and reducer tests.
- Added help, backend tracing, connection diagnostics, optional pad LED test and clean signal/terminal shutdown.
- Clarified NI service ownership and current validation limits; no audio/engine integration claimed.
- Patched published encdr-view macOS second-window creation failure locally; included source provenance and GPL license. Added mock DOM diagnostics.
- Recorded user-confirmed physical dual-screen, encoder 1 and page-button operation; selected NI service screens by default on macOS. Recorded intermittent mock shutdown hang.
- Added `--hardware-test`: side/color/orientation patterns, individual encoder direction counters, top/group/pad observations, pad row RGB test and STOP-button exit; writes hardware-test-session.json on hardware exit.
- Prevented terminal simulation from populating real-device test counters; added three diagnostic-model regression tests.
- Added explicit event-loop wake events and close-window shutdown requests on macOS; verified bounded mock quit/SIGINT sessions. Per-event logs now use debug level.
- Fixed Encdr 0.8.5 normal-disconnect ordering that skipped output cleanup; included local GPL source/provenance and two lifecycle regression tests. A connected MK3 run logged both LED-group clears and both exit-screen transfers before exiting 0.
- User confirmed patched physical exit state: LEDs off, Encdr logo displayed.
- Added waiting/reconnect and ownership lifecycle with retained screen windows, demo state and counters; current device and ownership gate input/output, and control regain redraws/restores test LEDs.
- Restricted explicit discovery as well as hotplug to configured MK3 filter; corrected service-route startup messaging.
- Added connection/reconnect status on screens and in reports, four lifecycle unit tests, one backend filter test and a reproducible bounded mock GUI smoke script.
- User confirmed groups A–H, physical USB reconnect with preserved counters/RETURNS and STOP-button exit with neutral cleanup.

- Clarified scope with the user: existing Mac emulator/audio/editor plus Maschine parameter controls and patch browsing/loading. Removed the separate mock-editor window, CLI and IPC; remaining hardware edge checks no longer gate engine integration.
- Added typed mock-backend parameter metadata, module/page/slot navigation, EngineAdapter test double, three developer fixture presets and optional validated JSON fixture persistence. These remain development aids; real emulator connection is outstanding.

- Added real G1-Emu integration: optional bounded loopback bridge, live panel assignments/value feedback, eight encoders across three pages, slot selection and bank patch browse/load; retains the existing emulator/editor responsibilities.
- Built pinned Apple Silicon G1-Emu/CoreAudio with the user's local ROM. Added upstream settings ACK ownership fix and regression, reproducible build/launch scripts, project-owned .pch and real-ROM/DSP waveform integration check. Verified actual parameter changes, stored-patch reload and rendered feedback; physical integration/editor synchronization remain unverified. Firmware/banks/recordings/build outputs are ignored.

- User confirmed the real physical G1 integration: frequency/level controls with sound/value feedback, then Maschine patch browsing/loading. Updated scope/status; simultaneous external-editor synchronization remains unverified.

- Added Maschine Browser-button file loading: local .pch folders, main encoder selection/press, Files/Banks sources, refresh, active-slot upload and firmware-confirmed status. Added --patch-dir, bounded file list/layout and integration/regression coverage. Rebuilt the optional G1 bridge to expose existing upload outcomes. Prior physical control/bank passes remain accepted; new Browser flow awaits physical use.

- Added combined G1/controller launcher and executable Finder .command entry point; compatible-instance attachment, readiness wait, duplicate locking, controller launch preferences and actionable bridge/startup errors. Controller exit preserves G1-Emu. Demo seeding uses a separate temporary port so startup cannot attach to the transient helper. Added focused launcher and isolated real-host lifecycle checks. Recorded the user key-assignment decision checkpoint; no new control mappings introduced.

- Applied the user's key assignments: four bottom encoder tiles on each screen, Groups A–D slots, Macro pages, < / > immediate preset loading, held Shift mapped to the actual G1 Function/Shift input, real Settings/Mixer views and state-driven button lights with unused controls off. Added real host/synth settings and master-volume ADC read/write to the optional bridge; Top 4 in Settings opens the existing Mac Settings card. Added modifier release on connection/handover and queue-pressure retry. Verified real engine controls, OS Function gating, layout and mapping/LED state; new physical layout/light use remains unconfirmed.

- Fixed missing patch-name letters by translating G1 LCD custom descenders (g/j/p/q/y), mirrored CGRAM codes and arrows into UTF-8 instead of replacing them with spaces. Verified codes against a controlled real-ROM patch and added a text regression target.
- Changed macOS screen polling to bounded asynchronous snapshots, allowing both screens to capture concurrently and preserving newer dirty state. Latest UI state no longer waits behind the transfer queue. Physical MK3/service timing check passed: capture maxima about36/52ms and command-loop median14ms; no before/after or panel scanout claim. Updated real-ROM check requires the complete MK3-Bridge LCD name. Emulator left running; controller reopened with the new renderer.

- Applied the supplied Inter design direction to live G1 screens: flat #141414 background, slot/patch and Nord Modular headings, light large values with small units, four bottom columns and a full-height preset list. Bundled unmodified Inter/OFL1.1 for offline consistency. Added 180ms retargetable list motion, reduced-motion support, coherent header settling, bounded native animation capture and per-screen visual-change detection. Verified actual font loading, intermediate/settled list transforms, preserved engine values, rendered PNG previews and physical MK3 capture timing. Key assignments unchanged.

- Corrected real G1 screen routing on MK3 and moved the browser candidate into the main name position while scrolling; click still confirms loading. Replaced the page counter with actual allocated (requested) voices, querying the patch header for requested voices. Removed the fixed control-worker sleep, refreshed screen state every 16ms, avoided common value-layout measurements, and averaged 2× Retina pixels for smoother text. Built both components and passed a focused isolated real-ROM check for 8 (8), browser header position, unchanged sound while scrolling, and click-to-load. Physical orientation/feel awaits user feedback.

- Removed the screen-only routing swap to keep encoders 1–4 under the first screen and 5–8 under the second. Browser selection now uses a fixed header and a separate following-row list with 120ms transitions. Reuse native RGBA bitmap representations and optimize Encdr conversion code in developer builds. Focused real-WebKit browser/load check passed; hardware timing attempt could not detect MK3, so physical responsiveness remains unverified.

- Preserved the user-confirmed knob alignment while swapping only hardware header/browser roles. Restored a vertically moving preset list with per-entry focus opacity. Suppress stale backward confirmed-value reports during pending encoder writes and keep numeric sizing stable within a parameter. One focused regression test and isolated real-WebKit browser movement/gradient/load check passed; physical smoothness remains subject to user feedback.

- Replaced offscreen CSS transform animation and delayed browser row cleanup with explicit snapshot-clock positions/opacity and retained virtual rows. Focus stays beside A /; rapid direction changes and reopen no longer remove current rows in the focused check. Added a brief stroke/glow on changed values and coalesced native captures to at most30fps. Added scripts/check_browser_motion.py: 44 rapid turns, reopen, gradient, click-to-load and value-activity check passed. Physical MK3 capture/NI transfer check also passed (latest captures about20ms); user smoothness feedback remains open.

- Investigated relative-encoder translation with isolated real-ROM timing. Replaced steady-state emulated-pot ADC writes with verified editor parameter messages through PresetsLink; median confirmation improved from95.5ms to11.6ms in comparable probes. Preserved assignment validation, patch-operation barriers, morph values, Function gating and identity-unknown ADC fallback; added native capability detection to prevent old-bridge attachment. Real audio/safety check passed; mixed-run latency included one118.6ms outlier.
- Replaced glow with bold activity and hid both knob strips in Browser. Added hidden native-frame rapid-reversal checks (124 direction changes, no blank captured frames). MK3 service upper animation frames are self-contained while bottom values keep fast partial transfers; identical frames are skipped. Physical service timing and screen-manager regression passed. Details and limitations: docs/encoder-response.md.
