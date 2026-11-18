# Official G1-Emu adapter

The external adapter translates the Rust controller's loopback JSON messages into DirectLink/PC Port operations against the stock G1-Emu VST. [Adapter.cpp](Adapter.cpp) uses Animatek NME's connection manager, codecs, patch model, upload serializer and parameter formatters. Its CMake target links NME and JUCE. Synthesis, audio and ROM execution stay in the plugin.

Follow the [main setup guide](../../README.md#setup) to build and install. The builder verifies the [NME and JUCE revisions](../../docs/licensing.md#installed-or-fetched-separately) before compiling. Use `--nme-dir PATH --juce-dir PATH` to build with existing checkouts at those revisions.

## Protocol and state

- The adapter exposes newline-delimited JSON to a local client. Requests carry an `id` and `method`; responses return the ID with `result` or `error`. [Adapter.cpp](Adapter.cpp) and [src/g1.rs](../../src/g1.rs) define both sides.
- Patch downloads populate the module catalogue, parameters and 18 knob assignments. Partial downloads leave the previous confirmed snapshot in place. A mutation acknowledgement means the request was accepted. Readback establishes the resulting state.
- `modules` returns the patch-bound catalogue. `module_select` checks patch identity, section and module index. `set_parameters` validates bindings and the supported descriptor ranges within 0–127, and `status.edit` reports the current selection. Module Edit selection is local to this bridge. PC Port exposes neither the plugin's Edit button nor its module cursor.
- Writes are coalesced and prioritized over targeted section reads. The first read deadline is retained during continuous turns and starts are limited to one per 40 ms. Targeted reads have their own reply deadline and retry budget. Only full-patch operations use the SDK's serialized fetch/upload path. Mixer readback prioritizes the active slot and abandons obsolete inactive reads on exit.
- `restore_slot` validates the active slot and `patchBinding`. Baselines come from the first complete snapshot or a confirmed load. The adapter writes only changed or unconfirmed parameters, then reads them back. `status.restore` reports availability, progress and errors.
- NoteSeqA/B, CtrlSeq and EventSeq edits use ordinary parameter writes. [SequencerMeters.h](SequencerMeters.h) uses the NME classic meter layout and the NMInfo stream to report playheads by slot and patch generation. Stale samples expire. The stream requires no extra polling.
- [MidiActivity.h](MidiActivity.h) observes S49 performance MIDI to animate the displays. It tracks notes and sustain locally, with no MIDI forwarding or DAW-clip visibility. Parameter names may come from a private cache extracted by the installer from a compatible local rack OS 3.03 ROM, with upstream metadata as the fallback.

The stock DirectLink endpoint accepts one editor client. Existing clients retain ownership. The bridge can adjust supported patch Output levels, but the stock interface does not expose host master gain or the native Settings card.

## Verification

The protected fixtures require the local compatible ROM at `roms/NORD-MODULAR-RACK-VER-3.03.BIN`, an unchanged official VST at `build/g1-official/alpha30/G1-Emu.vst3`, a release controller/adapter, and the test host from `python3 scripts/build_g1.py --plugins`. That last command builds the **test host** alongside the legacy plugins. Use the unchanged official VST as the fixture input.

```sh
python3 scripts/check_official_plugin.py --module-edit
python3 scripts/check_official_plugin.py --sequencer-only
python3 scripts/check_official_plugin.py --restore-only
python3 scripts/check_official_plugin.py --lost-reply
python3 scripts/check_keyboard_service.py --controller maschine
```

Choose the relevant probe. `--playhead-only` checks a clocked fixture; `--mixer-only --inactive-read-stall` checks that a lost inactive-slot reply leaves active control responsive. The official plugin ignores the legacy `G1_DATA_DIR` override. These fixtures use `sandbox-exec` to protect shared bank/settings data and check the shared bank digest. The direct adapter check refuses a live bridge lock. For the service test, use isolated ports and keep the installed watcher idle or stopped so it cannot select the fixture. Logs go to ignored `g1-state/`.
