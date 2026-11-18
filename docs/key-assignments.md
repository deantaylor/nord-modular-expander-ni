# Controller reference

The labels below refer to the physical buttons. Display buttons and knobs are numbered 1–8 from left to right.

| Action | Komplete Kontrol S49 Mk2 | Maschine MK3 |
| --- | --- | --- |
| Return to Home | TRACK | CHANNEL |
| Select slot A–D | Hold TRACK + display 1–4 | GROUP A–D, or hold CHANNEL + display 1–4 |
| Open emulator Settings | Hold TRACK + display 7 | SETTINGS, or hold CHANNEL + display 7 |
| Open Display settings | Hold TRACK + display 8 | Hold CHANNEL + display 8 |
| Previous/next knob page | Dedicated page arrows | Dedicated < / > buttons |
| Previous/next patch | PRESET UP / DOWN | SHIFT + < / > |
| Toggle patch browser | BROWSER | BROWSER |
| Toggle Module Edit | PLUG-IN | PLUG-IN |
| Toggle Mixer | MIXER | MIXER |
| Toggle Sequencers | PATTERN | ARRANGER |
| Restore edited parameters | CLEAR | ERASE |
| Hold Function | SHIFT | SHIFT |

On both controllers, holding SHIFT with the Home + display 1–4 shortcut uses Function slot pinning. Available controls are dim, active or pressed controls are bright, and unavailable controls are dark. The slot lamps follow confirmed focus. Maschine MACRO and PATTERN are unassigned.

## Home and Browser

Home shows the patch's 18 assignments across three knob pages. Four knobs sit below each display. The voice badge shows allocated voices, with requested voices in parentheses. Home, Browser, Module Edit, Mixer, Settings, Display and Sequencers are exclusive views. Returning Home restores the last assigned-knob page.

Press the 4D encoder from Home or press BROWSER to open the browser. Display buttons 1–4 choose **Files**, **Banks**, **Refresh** and **Load**. Turn the encoder to highlight an item, press it to enter a folder or load a patch, or tilt left in Files to go up one level. A confirmed load returns Home. A failed load keeps Browser open. Preset shortcuts load the adjacent patch in the current source and stop at the boundaries.

## Module Edit and Mixer

Turn the 4D encoder, tilt it up/down or press display 1/2 to select a module. The dedicated page arrows or 4D left/right page through its parameters. The eight knobs edit the values shown. Supported filter, envelope and oscillator diagrams show how base parameter changes shape the response or waveform. They are schematics rather than audio or modulation meters.

Mixer uses knobs 1–4 for slots A–D. A slot offers a level control when its patch has one supported Output module and the adapter has its data. Select an uncached slot to read it. The right display is blank. G1-Emu's master and DAW faders are controlled in their own interfaces.

## Sequencers and restore

The sequencer page lists Note Sequencer A/B, Control Sequencer and Event Sequencer modules from the active patch. Turn or tilt up/down to select a module on the left. Press **display 5** to unlock editing on the right.

- Tilt left/right to select one of the 16 stored steps. Turn to change the selected note, interval or control value.
- On an Event Sequencer, turn or tilt left/right to select a step, tilt up/down to choose its row, and press the encoder to toggle the event.
- Press display 5 again to return to the module list. Press the sequencer button, Home or another page button to leave. Parameter knobs are inactive while the sequencer page is open.

The red playhead follows the plugin's meter stream independently of the selected step. Both Event rows use the same playhead. Length comes from the patch and is independent of the 16 stored positions.

Release SHIFT before pressing CLEAR or ERASE. The bridge restores changed parameters in the active slot, including sequencer and Module Edit changes, with confirmed readback. Each baseline starts at a confirmed patch load or the first complete snapshot after connecting. The operation leaves assignments, patch structure, other slots and saved banks intact. A clean slot offers no restore action.

## Settings and native shortcuts

Emulator Settings uses display buttons 1–3 for its sections. Available settings depend on what the official protocol exposes. Display settings are local. Knob 1 chooses **Light**, **Dark** or **Colour**, knob 2 controls Blobs, knob 3 changes Glitch intensity, and knob 4 adjusts the intensity of the Colour theme. Glitch controls are available in Light and Dark. Slot colours are A red, B green, C orange and D yellow.

- **S49 Mk2.** Use INSTANCE to return to **Nord Modular**. SETUP and MIDI are NI functions. SHIFT + PLUG-IN is reserved for the native MIDI shortcut. Native mode changes can hand off ownership until you select the bridge instance again.
- **Maschine MK3.** Hold SHIFT and press PLUG-IN to open the NI instance selector. Turn the encoder to **Encdr** and press to select. SHIFT + CHANNEL opens native MIDI mode.
