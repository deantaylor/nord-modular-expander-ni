# Open work

## Hardware acceptance

- **Maschine MK3.** Verify the official-VST workflow on the physical panel, including the arrow and SHIFT mappings, sequencer editing, ERASE restoration, and monochrome/group lamp intensity. Discovery, ACTIVE ownership and ARRANGER input have been observed with this workflow. Earlier standalone hardware checks verified dual screens, encoder directions, reconnect and cleanup.
- **Komplete Kontrol S49 Mk2.** Review sequencer navigation and readback, CLEAR, the display themes and slot colours, and idle screensaver wake during real playing. Compare physical knob-to-LCD response after the rendering changes. If stalls recur, capture the on-demand pipeline trace and note the action/time. Host-transfer timing stops short of LCD scanout.

## Integration and release

- **Official DirectLink.** Concurrent editor use, external assignment changes and morph controls remain unverified. Revisit editor arbitration or instance switching when there is a concrete need and upstream support.
- **Public release.** Document terms for original code, fixtures and the Eastlab artwork before public distribution. See [licensing](docs/licensing.md).

## Deferred standalone feature

The S49 two-zone split is reserved for future standalone software. KEY MODE should select a physical midpoint split with independent A–D slot choices per display, four knobs per zone and the 49-key Light Guide in the selected slot colours. Keep TRACK shortcuts available. Verify a standalone polyphonic MIDI path first: the DirectLink editor note path is monophonic. Keep note-off destinations stable through slot changes, sustain, mode exit and reconnect. Confirm centre-key and octave/transpose behaviour on the physical keybed before shipping. The current VST and DAW MIDI workflow does not implement this feature.
