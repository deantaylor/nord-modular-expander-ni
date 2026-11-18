# CODEX HANDOFF — Modular Machine MK3

**Project:** Turn a Native Instruments Maschine MK3 into a dedicated, dual-screen front panel for emulated Clavia Nord Modular G1 and (eventually) Nord Modular G2 instruments running on an Apple Silicon Mac.  
**Document date:** 11 November 2026  
**Current phase:** Verify and harden the existing MK3 hardware prototype, then integrate G1 emulation.  
**Intended reader:** OpenAI Codex working inside the project repository.

> **Codex: this document is an implementation brief and work order.** Inspect the existing code and upstream projects; do not assume that earlier prototype claims imply a successful build or physical-device test. Make small, verifiable changes and record evidence of what works. Start the first milestone immediately without waiting for further product decisions.

## Mandatory operating policy — Spotify-style model routing and token efficiency

**Codex must use the cheapest sufficiently capable model for routine context gathering, and reserve expensive reasoning models for high-complexity work.** This is a project-level instruction, not an optional suggestion. Use Spotify Engineering's cost-aware agent/context approach as the reference pattern: <https://www.engineering.atspotify.com/2026/9/portal-by-spotify-cut-my-claude-code-token-usage-by-90> and <https://engineering.atspotify.com/2025/11/context-engineering-background-coding-agents-part-2>.

- **Lightweight/simple model by default for context and I/O:** reading or locating files, summarizing documentation, inspecting logs, scanning API references, collecting repository facts, finding relevant tests, extracting dependency versions, and producing brief factual context notes. Prefer deterministic shell tools (`rg`, `find`, `git diff`, selective `sed`/`cat`) before invoking any model.
- **Stronger reasoning model only when justified:** architecture or protocol design, complex multi-file edits, subtle concurrency/USB/audio bugs, compatibility and licensing tradeoffs, and difficult test failures that simpler methods cannot resolve. Avoid using a frontier model to repeatedly reread a repository.
- **Context handoff format:** let the lightweight worker produce a compact, structured handoff containing: goal; exact file paths and line numbers; verified facts; relevant API signatures; blockers/uncertainties; the smallest necessary excerpts; and the suggested next action. Pass this to the implementation model instead of whole documents, logs, or large code dumps. Preserve source links and avoid speculative summaries.
- **Cache and reuse context:** maintain a concise `docs/CONTEXT.md` (or equivalent) with verified project facts, decisions, dependency versions, and last successful commands. Update it only on material changes. Search it before re-discovering facts. Work in small, testable increments with explicit acceptance criteria.
- **Escalation:** if a simple model cannot resolve a question with evidence, escalate that *specific question* to a stronger model. Do not silently switch all routine work to the strongest model. Do not sacrifice correctness or safety just to save tokens.
- **Capability limitation:** if the Codex runtime cannot choose models per step or spawn differently configured subagents, **do not pretend it did**. Use concise deterministic retrieval, restricted context windows, and short tool outputs; document the limitation. No model-switching API or automation should be invented.
- **Reporting:** state briefly which tasks used lightweight versus stronger reasoning where the environment exposes that information, and give a concise token-efficiency note. Do not invent cost or token-savings figures.

This policy applies to every milestone, including initial repository inspection, upstream research, debugging, and documentation updates.

## 1. User goal and definition of success

The user owns an original Nord Modular G1 and Nord Modular G2, has an **Apple Silicon Mac** and a **Maschine MK3** (full MK3, not Mikro or Maschine+). They want the MK3 to behave as a custom hardware Nord Modular instrument:

- The **Mac runs the actual sound engines**, producing audio through CoreAudio or a suitable supported host/interface.
- The **MK3 acts as the custom front panel**: two 480×272 displays, eight encoders, buttons, pads, LEDs, module and patch navigation.
- The instrument supports **G1 patches and real-time editing** initially; **G2 is a second, separately validated phase**.
- When an editor is running on the same Mac, it can edit the active Nord patch **bidirectionally**: changes from editor update engine and hardware display; changes from MK3 update engine and, if supported, editor state.
- The system remains playable when the external editor window is closed. The initial Mac setup does **not** need to be standalone without a computer.
- Preserve the ability to migrate the **Rust control/state layer to Raspberry Pi / ARM64 Linux** in a later phase, without committing to that port now.

**Do not claim G2 emulation or original G2-editor interoperability is working until independently demonstrated.**

## 2. Existing starting point

An untested proof-of-concept source archive accompanies this handoff:

- `modular-machine-mk3-prototype.zip`
- Project root inside archive: `modular-machine-mk3/`
- Primary files: `Cargo.toml`, `src/main.rs`, `screens/left.html`, `screens/right.html`, `README.md`.

It currently uses `encdr = "0.8.0"` and `encdr-view = "0.8.0"`, renders mock Nord-style pages, listens for `Event::EncoderFine` and buttons, changes eight demonstration parameters, and drives two MK3 screen views. It has **no real synthesis, no ROM loading, no audio, no editor bridge and no persistence**. It was not compiled or tested on the user's Mac at creation.

Before editing, **read every file**, `git status`, `cargo metadata`, installed toolchain, and the relevant upstream API. Preserve the initial code as a baseline or make a commit; do not overwrite it without inspection. If the archive isn't already in the workspace, ask the user to attach or extract it rather than reconstructing it from a description.

### Existing provisional control mapping

| Control | Current prototype behavior | Future behavior |
|---|---|---|
| Screen encoders 1–8 | Edit eight mock values | Edit active module parameters |
| Top buttons 1–3 | Demo parameter pages | Switch module/page |
| Top buttons 5–8 | Select slots A–D | Actual slot selection |
| Group A/B | G1/G2 mock view | Engine selection only after engine verified |
| Pads and LEDs | Minimal LED demonstration | Note trigger, module navigation, morphs, variation/patch selection |

Treat future mapping as provisional: test ergonomics on physical MK3 before freezing it.

## 3. Sources to inspect (source of truth = upstream code)

1. Encdr repository: https://github.com/RufusIbiza/Encdr
2. Encdr MK3 quick start: https://github.com/RufusIbiza/Encdr/blob/main/docs/quickstart.md
3. Encdr API reference and examples under its `docs/` and `examples/` directories
4. Nord G1 emulator: https://github.com/animatek/G1-Emu
5. Gearmulator base project: https://github.com/dsp56300/gearmulator

**Specific cautions:**

- Encdr quick-start has shown a dependency example using `0.6.0`, while the present project specifies `0.8.0`. Inspect the **exact resolved crate version**, release notes and current signatures rather than copying examples blindly.
- Encdr quick-start currently says NI services (`NIHardwareAgent`, `NIHostIntegrationAgent`) can hold exclusive USB interfaces and may need to be stopped/suspended on macOS. The prototype README says to keep them running. **This is a known contradiction**. Determine the correct procedure for the exact installed Encdr version and user's NI setup, document both safe failure modes, and restore services on exit if the app stops them. Never silently kill processes without asking the user.
- MK3 is identified in Encdr documentation as USB VID:PID **`17cc:1600`**. Confirm using device detection; don't confuse it with Maschine+ (`17cc:1820`).
- G1-Emu's upstream README describes **pre-alpha** real-time G1 emulation, compatible ROM requirement, editor connectivity through a PC Port / Animatek NME, and GPLv3 licensing. Verify its macOS distribution, virtual MIDI, audio, implementation and integration points.
- G1-Emu's license / Gearmulator dependencies can affect redistribution. Document third-party license requirements; do not embed proprietary ROMs or copyrighted firmware. Do not assume all G1 or G2 patches load faithfully.
- Any G2 project, including prior mentions of “Nodal Mod,” is **research-only until its availability, license, architecture, editor protocol and usable API have been checked from primary sources**.

## 4. Architecture target

```text
+------------------------ Apple Silicon Mac ------------------------+
|                                                                    |
|  [Nord editor: Animatek NME / compatible editor]                  |
|               ^  patch, params, routing, sync  v                   |
|       [Editor/engine bridge, versioned typed events]              |
|               ^                           v                        |
|    [Shared state + controls] <----> [EngineAdapter interface]       |
|               ^                           |                        |
|    [MK3 UI renderer + hardware]       [G1-Emu first] ---> audio     |
|        via Encdr/Encdr-view          [G2 later: unverified]        |
+---------------|----------------------------------------------------+
                | USB HID + dual screen frames
          [Maschine MK3 front panel]
```

Suggested Rust modules (adapt to repository):

- `hardware/`: device discovery, ownership, MK3 input events, LED controls, screen output, graceful reconnection.
- `model/`: typed `Engine`, `Slot`, `Patch`, `Module`, `ParameterId`, parameter metadata (min/max, units, stepped vs continuous), immutable snapshots/event updates.
- `ui/`: screen models and render rate limiting; presentation independent of synth engine.
- `engine/`: `EngineAdapter` trait with `MockEngine` and eventually `G1EngineAdapter` / `G2EngineAdapter`. Avoid inventing functionality absent from actual backends.
- `editor/`: patch/editor protocol bridge and state synchronization.
- `audio_midi/`: MIDI notes, CoreMIDI routing and engine-specific audio selection.
- `diagnostics/`: structured logs, device status, engine connected/error states.

A simple in-process Rust core + `encdr-view`/HTML displays is fine for phase 1. **Do not introduce Tauri, a web server, a GUI framework or a daemon until needed.** Keep macOS-specific hardware/audio bindings behind interfaces.

## 5. Implementation plan — in order

### Milestone 0: Reproduce and audit the starting prototype

1. Extract/open the provided prototype and initialize Git if necessary.
2. Confirm `rustc`, `cargo`, Xcode Command Line Tools and Mac target `aarch64-apple-darwin`.
3. Run `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings` where practicable. Fix actual errors; don't suppress them indiscriminately.
4. Inspect the relevant Encdr version and compare event variants, device enumeration, view creation, `send`, `poll`, display ownership and LED calls.
5. Add `--mock` (no MK3 required) or equivalent simulator that renders exactly the same state and screens to desktop windows, enabling development without the device.
6. Make `--help` and `--diagnose` useful. Add diagnostics for USB device identification, backend version, NI services and screen-open failures.

**Acceptance:** `cargo check` and unit tests pass on an Apple Silicon Mac; the mock application launches and encoder/page input can be simulated, with documentation of what was actually executed. If unable to run on hardware, explicitly say so.

### Milestone 1: Hardware validation on MK3

1. Probe VID:PID `17cc:1600`, display human-readable device information and USB claim status.
2. Show a unique test pattern on **both** 480×272 hardware screens, with correct orientation, colors and no clipping.
3. Confirm all eight screen encoders are reported individually and map to displayed values with sensible sensitivity.
4. Confirm top buttons, groups and 16 pad events; add a safe RGB pad / button LED test and reset lights on exit.
5. Avoid flooded USB screen transfers: coalesce state changes; render only when dirty; use queue/backlog monitoring if supported by the resolved Encdr release.
6. Handle unplug/replug, missing NI services / exclusive claim failure, and clean shutdown without freezing or hard-crashing.
7. Record a hardware test matrix with pass/fail, Mac model, macOS version, Encdr version, screenshots/photos or relevant console logs (do not fabricate evidence).

**Acceptance:** Both MK3 screens show custom content, eight encoders modify the correct on-screen parameters, buttons/pages work, LED tests run, and app exits/restores cleanly.

### Milestone 2: Realistic mock Nord front panel

1. Replace hardcoded single 3×8 values with typed patch/module/parameter state.
2. Support slots A–D and navigable parameter pages populated by the active mock patch, with parameter-specific scaling and units.
3. Add a preset list, patch loading/saving as project-owned JSON (NOT falsely named `.pch`/`.prf`), and last-state restore.
4. Add a bidirectional **mock editor endpoint or local UI** to prove sync: change a parameter in mock editor -> MK3 updates; turn MK3 encoder -> mock editor updates.
5. Test rapid encoder movement, multiple page switches, limits, integer/enum parameters, and no feedback loops or divergent state.

**Acceptance:** Automated state/sync tests pass and the physical or simulated screens accurately reflect changes initiated from either side.

### Milestone 3: G1-Emu engine and editor integration

1. Clone/build or install a verified Apple Silicon G1-Emu release **without redistributing its ROM**.
2. Let the user supply their own compatible G1 ROM locally; detect it and show a clear configuration error if absent or incompatible. Do not bypass rights controls or download copyrighted ROMs.
3. Run G1-Emu independently first; confirm sound from at least one legal test patch and functioning editor link through **Animatek NME** or another verified compatible editor.
4. Inspect G1-Emu's source to identify a maintainable route to observe/control current slot, patch, module, parameter and real-time changes. Decide between a documented integration API, external process/IPC, MIDI/SysEx bridge, or a small clearly isolated upstream patch. **Do not assume standard MIDI CC exposes arbitrary module parameters**.
5. Write an architecture decision record comparing tradeoffs, stable interfaces, patch handling, license implications and synchronization behavior.
6. Implement smallest end-to-end integration: one known G1 patch, one module, at least two parameters, audio active; hardware encoder changes engine state and editor changes reflect on the MK3. Expand only after this passes.
7. Handle patch transfer/slot changes, morph controllers, parameter limits, save/load, source-of-truth semantics, and reconnection without silent corruption.

**Acceptance:** An actual G1-Emu patch plays audio; a parameter change on MK3 changes audio and appears in the editor if the editor protocol supports it; an editor parameter change updates MK3 graphics; no mock values are shown as though they represent the engine.

### Milestone 4: G2 feasibility gate (do not promise an engine prematurely)

Produce a concise evidence-backed technical report verifying:

- Existence and availability of a functioning Nord Modular **G2** emulation engine (distinct from Nord Lead 2x).
- macOS Apple Silicon support, patch compatibility, performance, license and dependencies.
- Whether the original G2 editor, a modern editor, or an equivalent patch editor can communicate with that engine.
- Whether the engine exposes module parameter state and supports bidirectional editing.
- Whether it is practical to integrate using the shared Rust model and MK3 interface.

If unsupported, **stop at a clean G1 release plus documented G2 blocker**. Do not fake G2 functionality or substitute a different Nord synthesizer without explicit product approval.

### Milestone 5 (later): macOS app packaging / Raspberry Pi planning

Package a self-contained, user-friendly `.app` when hardware and G1 integration are stable. Address signing/notarization separately; unsigned developer build is acceptable in early testing. Raspberry Pi port is a future work item, not acceptance criteria for the Mac prototype.

## 6. Explicit non-goals for the first iteration

- No Maschine+ firmware replacement, jailbreak, standalone NI engine integration or internal computer modification.
- No hardware DSP emulation written from scratch.
- No G2 claims until a real engine and editor are proven.
- No automatic ROM dump, firmware bypass, ROM bundling or distribution.
- No requirement that standard Maschine desktop software run simultaneously with direct USB control.
- No requirement to mimic every Nord front-panel function in the first pass.

## 7. Engineering quality constraints

- **Testability:** `MockEngine` and simulated hardware events; hardware integration separated from business logic; deterministic state reducer tests.
- **Observability:** logs with timestamps, device/engine IDs, clear errors, explicit “DEMO / NO ENGINE” on mock screens.
- **Performance:** avoid allocations or expensive HTML redraws in critical input loops; throttle display updates and drop stale frames; no MIDI or audio processing on slow UI threads.
- **Robustness:** validate bounds/parameter types; debounce or coalesce where appropriate; avoid data races; clean exit; restore LEDs/USB ownership.
- **Portability:** keep Apple-only plumbing isolated; support eventual ARM64 Linux build of state/controller core.
- **Safety / licensing:** don't ship proprietary Clavia ROMs, firmware or commercial patches; follow GPL obligations if incorporating/modifying G1-Emu/Gearmulator; include attribution and license notices.
- **No false success:** distinguish *compiles*, *runs in mock*, *runs with physical MK3*, *plays G1 audio*, and *editor bidirectional sync*.

## 8. Concrete deliverables Codex must create

1. Working and documented source repository with `Cargo.lock` and reproducible build instructions.
2. Updated `README.md` for Apple Silicon and physical MK3, including ownership/service troubleshooting.
3. Executable hardware diagnostic / screen-and-encoder test mode.
4. Automated tests for state, controls and mock-editor synchronization.
5. `docs/architecture.md` plus `docs/hardware-test-results.md` with evidence, unresolved issues and actual version numbers.
6. `docs/g1-integration-research.md` giving exact G1-Emu integration APIs or justified bridge choice.
7. A small `CHANGELOG.md` and a prioritized `TODO.md`, with blockers differentiated from next steps.
8. Eventually a G1 demonstration that genuinely produces audio and works with an editor; do not present that as complete before testing.

## 9. Definition of done per report

At the end of every coding session, report:

- Files changed / new commits.
- Exact commands run, their results and the host OS/architecture.
- Hardware observations **only if tested on a real MK3**.
- What's working vs simulated vs blocked.
- The single next best engineering task and how to test it.

If no MK3, Nord ROM or Mac build environment is available to the agent, complete all possible code/static/mock tests, document limitations and provide exact user-run verification commands. Do not fabricate hardware results.

## 10. First task to execute now

**Implement Milestone 0, then attempt the simplest useful portion of Milestone 1.** Do this in this order:

1. Inspect project files and upstream Encdr signatures/version; identify conflicts and bugs.
2. Make the code compile on the user's Mac, adding version pinning as needed.
3. Add mock mode that renders both screens without MK3 and unit tests for parameter state transitions.
4. Add diagnostics and clear startup help (including how to recover from NI USB ownership conflicts).
5. Provide exact `cargo run -- --mock` and `cargo run -- --diagnose` commands (or their documented equivalents), report what was verified, and request user hardware results only where indispensable.

Do not jump to G2 or change frameworks before completing the MK3 hardware proof of concept.

---

## Copy-and-paste opening instruction for Codex

> You are the implementation engineer for **Modular Machine MK3**. Read `CODEX_HANDOFF_Modular_Machine_MK3.md`, then inspect the accompanying `modular-machine-mk3` source tree. Start Milestone 0 immediately. Verify upstream Encdr APIs rather than trusting the untested prototype. Make the smallest buildable improvements, add a hardware-free mock mode and tests, and document any NI service/USB ownership conflict accurately. Work towards a real Nord G1-Emu/editor bridge, but do not claim G1/G2 integration or physical MK3 verification until demonstrated. Summarize changed files, commands run, evidence, blockers and next actions.
