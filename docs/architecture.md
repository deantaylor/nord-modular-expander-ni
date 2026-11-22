# Architecture and development

Nord Modular Expander for Ni Controllers connects one Komplete Kontrol S49 Mk2 (`17cc:1610`) or Maschine MK3 (`17cc:1600`) to the official G1-Emu VST on Apple Silicon macOS. The plugin owns synthesis, audio, patch data and DAW state. Existing G1 tools handle full patch editing. Performance MIDI follows the DAW's routing. G2, Linux and Raspberry Pi are outside the current scope.

## Process boundaries

```text
NI surface ⇄ Encdr ⇄ Rust controller ⇄ loopback JSON ⇄ C++ adapter ⇄ DirectLink / PC Port ⇄ G1-Emu VST
                                                           ↑
                                                   passive S49 MIDI activity
```

The per-user launchd watcher in [scripts/keyboard_service.py](../scripts/keyboard_service.py) discovers an official plugin and starts the adapter plus the selected controller. It holds one service lock and cooperates with the manual launcher lock. When the plugin closes, it tears down its own processes and waits for another instance. The installer stages a versioned runtime outside the checkout and retains the previous registration for rollback.

| Area | Start here |
| --- | --- |
| Input handling, device profiles and LEDs | [src/controller.rs](../src/controller.rs), [src/input_dispatch.rs](../src/input_dispatch.rs) and the [Encdr descriptors](../vendor/encdr/descriptors/) |
| Session, engine state and JSON client | [src/session.rs](../src/session.rs), [src/g1.rs](../src/g1.rs), [src/connection.rs](../src/connection.rs) |
| Page selection and browsing | [src/browser_state.rs](../src/browser_state.rs), [src/patch_browser.rs](../src/patch_browser.rs), [src/sequencer.rs](../src/sequencer.rs) |
| WebKit screen lifecycle and views | [src/main.rs](../src/main.rs), [screens/g1.html](../screens/g1.html), [Encdr View changes](../vendor/encdr-view/PATCHES.md) |
| PC Port model, writes and readback | [integrations/g1-official/](../integrations/g1-official/), including the pinned adapter and protocol notes |
| Older standalone and custom-plugin builds | [integrations/g1-emu/](../integrations/g1-emu/), retained for maintenance and existing projects |

## Invariants for changes

- Treat G1-Emu as authoritative. Validate slot, patch and assignment identity as well as parameter ranges. Discard stale work on patch replacement, slot change or disconnect. Keep pending feedback separate from confirmed state. The voice badge shows allocated voices with requested voices in parentheses.
- Keep hardware input independent of screen capture. Coalesce encoder writes, bound background work, and let due readback proceed during sustained turns. WebKit snapshots are asynchronous. Keep the last good frame on capture failure, skip unchanged frames, and preserve the vendor-specific partial/full transfer policies.
- Respect NI ownership and leave its services running. Cleanly release held modifiers, clear owned LEDs and retarget views after reconnect. Keep availability, selection and press lamps distinct. Button mappings belong in the [controls reference](key-assignments.md).
- Keep browser navigation inside the configured library, skipping symlinks. Wait for confirmed patch loads before closing the browser. Loading a file leaves the source patch and stored banks intact.
- Extend the existing adapter only where the pinned upstream protocol supports the operation. Keep audio/ROM handling in the plugin and keep native controls separate from bridge mappings.

## Build and verify locally

Use the [setup instructions](../README.md#setup) for release builds. For changes to the Rust controller or Python watcher, start with the smallest applicable check.

```sh
cargo test --locked
python3 -m unittest discover -s scripts -p 'test_keyboard_service.py'
cargo build --locked
python3 scripts/check_mock.py
```

The mock check runs the real WebKit renderer with generated data and simulated hardware input. Focused checks under [scripts/](../scripts/) cover browser, module and sequencer views. The module and sequencer screenshot checks use Pillow. After a release build, run `python3 scripts/benchmark_bridge.py --expect-retained` to check that playhead updates retain the sequencer grid and report component timings for LED state and sequencer updates.

The [adapter notes](../integrations/g1-official/README.md#verification) describe the isolated official-VST fixtures. Use disposable state, patch libraries, banks and ports, and keep live DAW/emulator sessions out of tests. Keep evidence under ignored `g1-state/`. Renderer captures and successful host transfers establish software behavior. Physical screen orientation, latency, brightness and feel need hardware observations.

Earlier prototype records and diagnostic chronology are available in [Git history](https://github.com/deantaylor/nord-modular-ni-interface/tree/7e5fce4f36bbfbef10d03cf99b4867ac33c29692/docs).
