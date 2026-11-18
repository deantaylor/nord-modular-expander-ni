## Calibrated S49 controls and lamps — 16 November 2026

PLUG-IN toggles Module Edit; display buttons5–8 are unassigned/off. Hold TRACK + display buttons1–4 for slots A–D. SHIFT holds the existing Function modifier. Eight knobs and preset/page navigation keep their existing actions.

The corrected Encdr descriptor uses **indexed white** for display buttons1–8 at payload offsets2–9 (dim69, bright71). Monochrome lamps use SHIFT14, TRACK23, LEFT32, RIGHT33, BROWSER35 and PLUG-IN36; user confirmed white at158. Output15 is SCALE,31 STOP,34 CLEAR and37 MIXER; these stay unassigned/off. Inactive/available actions are dim, active choices/held buttons bright, unavailable actions off. Preset up/down lamp positions remain unverified and are not programmed; their input actions still work.

Ordered physical input capture and isolated lamp observations are recorded in `g1-state/s49-physical-input-map-20261116.json` and `g1-state/s49-led-calibration-20261116.json`. The inherited map had both wrong names/positions and the wrong display-button encoding; sending monochrome158 to display buttons caused green lights.

# Keyboard module Edit page — path assessment, 15 November 2026

The existing G1 UI **Edit** button is a front-panel synth OS mode, distinct from the eighteen assigned patch knobs and from the external NME graphical patch editor.

## Existing UI path

Inspected local G1 source: `app/gui/Panel.cpp` maps Edit to button matrix row1/bit2 (lines26–30,391–394). `PanelButton::setBit` forwards press/release to the emulated microcontroller (1338–1347). Mode LEDs identify Edit (1580–1592). The OS LCD identifies poly/common module pages with P/C and morph pages with T; native navigation walks modules/parameters and the dial changes the current value (1598–1603,2680–2717). This is existing synth functionality, not a new patch editor to reproduce.

## Current official-plugin boundary

Our production path is the unmodified official VST plus external `integrations/g1-official/Adapter.cpp`, using pinned NME protocol/model code. Legacy Mk3ControlBridge and KnobMap RAM helpers are not available to this external adapter.

NME exposes `sendParameterFromUser(slot,section,module,param,value)`, which sends parameter focus when needed and a real parameter edit. The ParamFocus protocol comment says it tells the synth which parameter the editor is working on; it is not a generic front-panel Edit-button command. No supported API was found for pressing Edit, querying the native module-selection cursor, or navigating that cursor. Do not claim native Edit selection synchronization.

The adapter already owns the full parsed patch, including modules, descriptors and parameter values. Its JSON interface currently exposes only the eighteen knob assignments. Module browsing can therefore be bridge-owned while values/edits remain actual emulator data.

## Keyboard Module Edit (implemented)

| Control | Module Edit action |
| --- | --- |
| PLUG-IN | Toggle Module Edit / assigned-knob view; dim when available, bright while active |
| Main4D dial | Select the previous/next real module; selection wraps |
| Screen buttons1/2 or4D up/down | Previous/next module |
| Page arrows or4D left/right | Previous/next page of the selected module's parameters |
| Encoders1–8 | Edit the corresponding displayed parameters, four per screen |
| BROWSER / PRESET up/down | Existing patch browser / previous-next patch actions |

The Nord Modular header becomes Module Edit. Its upper area shows neighboring module names with selection emphasis and a Poly/Common area indicator. PAGE X/Y refers to parameter pages of the selected module. Modules with no supported editable parameters have one empty page and do not accept encoder writes. Page buttons are off for single-page modules. The patch/voice header is retained.

Exit with PLUG-IN to restore the previous assigned-knob page. Module editing is a bridge-owned view of actual module parameters, not native Edit-cursor synchronization. It does not change saved knob assignments or enable patch construction/cabling. Browser overlays the mode; after a patch changes, module metadata/selection refresh from the new generation. On connection loss the controller returns safely to assigned mode.

## Implementation boundary

The adapter adds modules (catalogue), module_select/module_exit and set_parameters requests. Catalogue is requested lazily on entering the mode or when patch identity changes; status includes only the selected module's parameters. Normal7-bit parameter-class controls use actual descriptor names, ranges and NME value formatting; custom/morph entries and unsupported ranges are excluded. Module and parameter targets validate slot/generation/section/module/parameter identity. Parameter writes retain coalescing and confirmed readback. No native button/memory injection or modified VST.

## Existing knob-page indicator implemented now

The Nord Modular screen shows **PAGE1 /3**, **PAGE2 /3**, **PAGE3 /3** (with normal spacing). Existing page contents remain1–8,9–16,17–18. The patch screen retains allocated(requested) voices. Page number is hidden in Browser/Settings/Mixer and page identity participates in render invalidation even when all tiles match.

Module Edit and the existing assigned-page indicator are now wired. Parameter-driven ADSR/Mod-Env and Filter A–F diagrams are now available on the physical right-hand screen.

## Right-hand parameter diagrams

Module Edit shows a diagram above the physical right-hand encoder tiles for these types (custom module titles do not affect recognition):

| Family | Diagram |
| --- | --- |
| ADSR | Attack/decay/sustain/release, Log/Lin/Exp attack and inversion |
| Mod-Env | Base ADSR shape and inversion |
| Filter A/B |6dB low/high-pass cutoff shape |
| Filter C/D |12dB LP/BP/HP outputs together, with line-style legend |
| Filter E |LP/BP/HP/BR,12/24dB slope, cutoff, resonance, gain control and bypass |
| Filter F |Classic LP,12/18/24dB slope, cutoff, resonance and bypass |

Graphs use parameters from the entire selected module, including controls on another encoder page. During encoder edits, requested values update the curve immediately with a PENDING label; engine-confirmed values remain separate and reconcile the preview. They redraw only when data changes; capture cadence and parameter transport are unchanged. Module navigation remains on the physical left; parameter tiles stay on both screens. Browser/assigned mode and unsupported modules clear the graph.

These are base-parameter schematics, not an oscilloscope, measured frequency response, live gate timing or external modulation. ADSR segment widths follow normalized parameter geometry; numeric time labels use the existing emulator/NME formatters. Modulation depth, key tracking and live input signals are not synthesized by the graph. Filter A/B do not gain guessed Hz units.

Renderer/attribution: screens/module-visuals.js and screens/THIRD_PARTY_NOTICES.md.

## Visual presentation and modifiers — 16 November 2026

Parameter tiles omit encoder numbers/repeated module names in Module Edit. Serum-inspired dark graph panels use filled colored curves and envelope nodes; no copied graphics or live animation. OscA waveform4/width3 and OscB waveform3/width7 use the upstream oscillator enum Sine/Tri/Saw/Square (not LFO enums). Pulse duty uses fmtPulseWidth’s percent conversion; modulation inputs are not inferred. OscC is sine; OscSlvB uses pulse/width2; fixed OscSlvC/D/E are saw/triangle/sine. Unknown oscillator types keep their parameter controls without a fabricated waveform. Track+display1–4 selects A–D; holding Shift too retains existing Function slot pinning.
