# Standalone keyboard split — deferred design

Recorded 17 November 2026. **Documentation only; not implemented.** The user deferred this feature to future standalone software. Do not add virtual MIDI routing to Live or modify the official VST for this feature.

## Agreed behaviour

- The bridge registers the Nord Modular instance and requests keyboard control through NI's service. Respect instance handovers; do not stop NI agents or repeatedly steal ownership.
- **KEY MODE** toggles between the normal keyboard view and a two-zone split view.
- Split the S49 Mk2 keybed in the middle. The lower zone belongs to display 1; the upper zone belongs to display 2.
- Each display shows its own selected slot, patch and controls. Its four physical knobs control that slot's parameters.
- Display buttons **1–4** select slots **A–D** for the left display/zone. Display buttons **5–8** select **A–D** for the right display/zone.
- Available slot buttons are dim in their slot colour; the selected slot on each side is brighter. KEY MODE reflects whether split mode is active.
- The 49 Light Guide LEDs above the keys show each zone in its selected slot colour: A red, B green, C orange, D yellow.
- Split/slot selection and display logic live in the bridge. The synth remains responsible for synthesis.

## Implementation constraints for the standalone phase

Use the standalone software's supported performance MIDI path for polyphonic note routing. Route each zone to its chosen slot without a DAW setup requirement. Verify this path before implementation; do not assume the editor connection is a performance MIDI endpoint or resume the superseded custom VST approach.

Preserve note-on/off pairing through slot reassignment, mode changes, sustain and reconnects. A note-off must reach the destination that received its note-on. Release held/sustained notes cleanly when changing routing or relinquishing the keyboard. Wheels, pedals and pressure need explicit, tested zone behaviour.

Keep the split at the physical midpoint when the keyboard is transposed or octave-shifted, and keep Light Guide colours aligned with physical keys. With 49 keys, a proposed initial division is 24 lower keys and 25 upper keys, with the centre key in the upper zone; confirm the exact centre-key rule during standalone implementation. Do not silently equate the physical midpoint with a fixed MIDI note under all octave settings.

Retain TRACK's modifier shortcuts for Settings/Display. Resolve their precedence over the split page's eight slot buttons. Define per-zone page navigation, browsing/loading focus, and return from other pages before extending beyond the initial split view.

## Verified findings and limits

The S49 Mk2 descriptor already represents KEY MODE input (report 0x01, byte4/mask0x40), its button lamp, all eight display-button lamps, and a 49-element `light_guide` strip (report0x81). These are available building blocks, not physical verification of the new split feature. See `vendor/encdr/descriptors/ni_komplete_kontrol_s49_mk2.json`.

The current official VST receives performance MIDI from its DAW track. Its separate DirectLink connection represents the editor PC Port; the existing editor-note command handles one held key on the active slot and cannot implement the required polyphonic two-slot split. Official source checked: `animatek/G1-Emu@d05d8ab`, `app/plugin/Processor.cpp` (PC Port creation and host MIDI processing); pinned NME `source/midi/ConnectionManager.h` documents the editor-note limitation.

NME patch headers contain `keyRangeMin`/`keyRangeMax`, but the inspected API updates them through a full patch upload. Their external-MIDI filtering behaviour was not established. Do not use an unverified header rewrite/full upload as a shortcut, particularly against unsaved user patches.

The exploratory bridge MIDI-routing code was removed when the user deferred this work. No split binary was built or installed, no virtual split port was created, and the running VST/DAW routing was not changed.

## Acceptance checks for future work

- Both zones play polyphonically and independently with the intended patches.
- All eight slot buttons select the correct side; four knobs per display edit the correct slot with confirmed feedback.
- Light Guide, screen headers and button colours agree, including octave/transpose changes.
- No stuck or duplicate notes during sustain, slot changes, mode exit, instance handover or reconnect.
- Standalone split works without routing through a DAW; existing VST integration remains independent.
