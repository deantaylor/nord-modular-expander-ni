# Bridge performance — 17 November 2026

This pass removes redundant work without changing encoder decoding, control pacing, MIDI routing, or the official VST.

- The hardware LED loop previously generated the entire screen payload to read its LED field. It now uses the same LED computation directly, avoiding sequence, parameter and browser allocations under the input-state mutex.
- Connection metadata no longer constructs a complete development-panel snapshot.
- Playhead-only updates retain the right sequencer grid and change dot classes. Value, edit, selection and layout changes invalidate the cache and rebuild correctly.

## Repeatable benchmark

Build with `cargo build --release --locked`, then run `python3 scripts/benchmark_bridge.py --expect-retained`. The script starts only a local EventSeq fixture and hidden WebKit views. It neither attaches to Live nor changes user patches. The benchmark commands are available only in the development mock command loop.

Three trials on the same machine and release configuration:

| Workload | Before median | After median |
| --- | ---: | ---: |
| 2,000 LED-state reads | 187.746 ms | 3.171 ms |
| 500 playhead updates, scripting and forced layout | 103 ms | 23 ms |
| Grid replacements in 500 updates | 500 | 0 |

The renderer benchmark includes state handling, DOM work and synchronous layout. It excludes native capture, NI transfer, LCD scanout and real-world input latency. Timings vary; node-retention is asserted rather than enforcing a machine-dependent speed threshold.

Evidence: `/tmp/bridge-perf-before.log`, `/tmp/bridge-perf-after.log`, `/tmp/bridge-perf-final.log`. Final verification: release build, focused G1 Rust tests (44 passed, one existing ignored), and `scripts/check_sequencer.py` covering edits, bounds, playheads and page cleanup. See `/tmp/bridge-perf-state-tests.log` and `/tmp/bridge-perf-sequencer.log`.
