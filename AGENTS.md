# Repository instructions for coding agents

Apply these instructions to the entire repository. They carry forward the operating rules in [CODEX_HANDOFF_Modular_Machine_MK3.md](CODEX_HANDOFF_Modular_Machine_MK3.md), updated for the user's subsequent scope and workflow decisions.

## Sources of truth and starting work

- Follow current user instructions first. Use [docs/scope.md](docs/scope.md) for product scope and [docs/key-assignments.md](docs/key-assignments.md) for agreed controls. These supersede conflicting provisional mappings and mock-editor deliverables in the original handoff.
- Read relevant sections of [docs/CONTEXT.md](docs/CONTEXT.md) and [TODO.md](TODO.md) before rediscovering project facts. Read the original handoff when its detail is needed.
- Inspect `git status --short` and the affected files before editing. Preserve existing work; do not reset, overwrite, or commit unrelated changes.
- Continue from recorded progress. The original handoff's initial audit and hardware milestones are not instructions to restart completed work every session.

## Scope and architecture

- G1-Emu on the Mac owns synthesis, audio, patch data and full patch editing. This application connects S49 Mk2 and Maschine MK3 controls and displays to actual emulator state.
- Do not build a custom or mock patch editor, another synth engine, or a parallel production patch format. Hardware-free screen simulation and explicit developer fixtures remain development tools.
- Implement only backend-supported controls; inspect the exact pinned upstream source/API rather than guessing protocol details or assuming MIDI CC exposes arbitrary parameters.
- Keep the Rust state/transport layer separate from platform-specific hardware, rendering and audio plumbing. Do not add frameworks, servers or daemons without a concrete need.
- G2 and Raspberry Pi/Linux are later work. Do not claim G2 compatibility or editor synchronization without independent evidence.

## Context, model use and efficiency

- Prefer deterministic retrieval: `rg`, targeted file reads, diffs and short log excerpts. Batch independent reads and avoid repeatedly dumping whole files or rediscovering verified facts.
- When delegation and model selection are available, delegate substantial routine reading, repository searches, documentation lookup and log inspection to a cheaper, sufficiently capable agent. Do not spend the implementation agent's context repeatedly reading raw files and logs. Tiny lookups should use direct deterministic tools when delegation would cost more than it saves.
- Give the reading agent a bounded, read-only question and the relevant paths. Require a compact factual report with source paths/line numbers, minimal excerpts and uncertainties; do not request whole-file dumps. Reuse that report and cached project context instead of repeating its exploration.
- Select an explicitly cheaper available model for reading work; spawning another agent with the same expensive model does not satisfy cost-aware routing. Reserve stronger reasoning for protocol/design decisions, concurrency, compatibility and difficult failures. Escalate only the unresolved question when a cheaper agent cannot establish an answer with evidence.
- Follow runtime restrictions on delegation. If model selection or differently configured workers are unavailable, state that limitation briefly and use concise deterministic retrieval with restricted context. Do not invent model switching, cost figures or token savings.
- If handing work between agents, provide a compact goal, exact paths/line references, verified facts, relevant API signatures, uncertainties, minimal excerpts and next action. Escalate the specific unresolved question rather than expanding all context.
- Update `docs/CONTEXT.md` only for material changes: decisions, verified behavior, dependency versions, useful commands and remaining limitations. Keep it factual and reusable.

## Implementation and performance

- Make small, reviewable changes tied to the current task and explicit acceptance criteria. Reuse the established launcher, bridge and screen renderer.
- Keep input handling responsive: coalesce encoder writes, bound queues, discard stale frames and redraw only changed output. Avoid blocking snapshots, expensive layout or audio/MIDI work on the control/UI loop.
- Validate parameter bounds, types and assignment identity. Treat the emulator as authoritative; do not present a pending load or guessed value as confirmed engine state.
- Preserve clean shutdown, modifier release, LED cleanup, device ownership and reconnect behavior. Keep selection, loading and active-slot state coherent.
- Keep inactive controls unlit. Preserve agreed mappings unless the user changes them. The voice indicator is **allocated voices (requested voices)**, not a page count or sounding-note count.

## Verification and iteration speed

- The user explicitly requests fewer checks and faster iterations. For ordinary changes, run the relevant build and the smallest focused check that verifies the changed behavior. Documentation-only edits need no application build.
- Run broader tests, Clippy or real-engine/hardware suites when a change's scope, a failure or a specific unresolved risk justifies them. Do not rerun every suite for routine UI changes.
- Accept recorded and user-confirmed hardware passes. Do not ask the user to repeat the generic Encdr checklist as a prerequisite for integration work.
- Use isolated patch libraries, bank files, ports and state for automated emulator checks. Do not overwrite the user's patches or banks, or restart their live emulator and lose unsaved work for a test.
- Distinguish compilation, simulated UI, real ROM/DSP/audio checks, physical MK3 observations and existing-editor synchronization. An automated check does not prove physical feel, screen orientation or audible behavior unless actually observed.
- Record commands and results where useful. Never fabricate test evidence, hardware observations or performance comparisons.

## Hardware, user state and licensing

- Target Komplete Kontrol S49 Mk2 (`17cc:1610`) and Maschine MK3 (`17cc:1600`) with one selected surface per bridge session. Keep NI agents running.
- Use the official, unmodified G1-Emu VST as the synth. Attach an external keyboard bridge through supported upstream interfaces; do not continue the locally branded/custom synth-plugin approach.
- Start the keyboard bridge automatically in the background while an official plugin is available. Keep keyboard notes/performance MIDI decoupled from surface integration and preserve existing DAW routing.
- Do not silently kill NI services or other user processes. Investigate ownership failures against the installed Encdr version and recorded project evidence.
- Preserve the user's running emulator session and unsaved patch state. Explain when a native bridge update needs an emulator restart; controller exit normally leaves G1-Emu running.
- Use only the user's compatible local ROM. Do not download, bundle or redistribute proprietary ROMs, firmware or commercial patches, or add rights bypasses.
- Preserve third-party attribution, licenses and GPL obligations for modified Encdr/G1-Emu/Gearmulator code. Keep upstream integration patches reproducible and dependency versions pinned.

## Documentation and reporting

- Update README/setup notes, key assignments, TODO, changelog and context when their content materially changes. Keep remaining work and blockers distinct from completed work.
- Give concise progress updates and a short final report: what changed, relevant verification and limitations, and the next necessary action. Include exact commands or evidence paths when they help reproduce a result.
- State model-routing limitations when relevant; do not claim unavailable routing or measured savings. Keep reports proportional to the change.
