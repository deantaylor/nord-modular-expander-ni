> Current workflow: official unmodified G1-Emu VST plus the automatic external S49 bridge. See [keyboard guide](komplete-keyboard.md). The earlier custom-plugin/Maschine implementation below is historical.

# Test on return

For the historical standalone checks below, save any unsaved work in G1-Emu, close it normally, then run `python3 scripts/run_machine.py --controller maschine` or launch the legacy app in `build/package`. A native bridge update needs a normal emulator restart. Keep NI agents running.

1. Confirm the first physical screen has slot/patch/browser, the second Nord Modular, and encoders still match the controls directly above them.
2. Turn assigned knobs slowly and quickly in both directions. Check final values, bold activity and perceived response. Macro cycles three pages; inactive controls stay dim.
3. Open Browser. Both knob strips disappear. Reverse direction repeatedly: the focused row should move into `A /`, with fading neighboring rows. Scrolling previews; pressing loads.
4. Load a local patch, navigate a subfolder/parent, switch Files/Banks and return to controls. Check long names and the selected slot. File loading must leave source files and stored banks alone.
5. Use Settings, Mixer and held Shift; verify the lights match the actual state. Unplug/reconnect MK3 during a session and confirm state returns without a freeze.

If a freeze returns, note the last action and approximate time. Preserve `g1-state/controller-session.log` and its numbered backups (packaged app: `~/Library/Application Support/Modular Machine/`), plus any matching macOS crash report. Controller exit leaves G1-Emu running.

Existing-editor synchronization remains on hold. No external editor needs opening for these checks. Automated coverage and its limits are recorded in [CONTEXT.md](CONTEXT.md).

6. Load `MK3-MIDI.pch` from the project library. Pad Mode should play drum-range notes; Keyboard should play chromatically, with louder/softer strikes changing velocity on patches that respond to it. Confirm pad-up, STOP and switching modes release held notes. Scales are intentionally left for the next definition. Try fast knob turns again with the halved sensitivity and report any remaining freeze.
