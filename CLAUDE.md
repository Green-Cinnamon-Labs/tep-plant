# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.


## Architecture

The repo is split into two Rust crates under `tennessee-eastman-service/`:

### `core/` — Pure math library (`te_core`)
No I/O, no threads, no notion of real time. Knows only: plant, state, EDOs, integrator.

- `dynamics/tep/` — TEP model: state equations, thermodynamics, disturbance channels (IDV), constants, initial state
- `method/` — Integrators: `Integrator` trait, RK4 and Euler implementations
- `plant.rs` — `Plant<M, I>` generic struct: holds `State`, `Bus`, model, integrator. `plant.step(dt)` advances physics one tick.
- `bus.rs` — `Bus` with `Inputs { mv, dv }` and `Outputs { xmeas }`. `mv` = 12 manipulated variables (XMV), `dv` = 20 disturbance channels (IDV), `xmeas` = 22 process measurements (XMEAS).
- `snapshot.rs` — `SimulationSnapshot` returned by `plant.snapshot()` for logging and alarms.

### `service/` — Executable runtime (`te_service`)
Knows: loop, clock, interfaces, gRPC. Does NOT know plant internals.

- `main.rs` — Experiment setup: controller injection, `Config` construction, spawns simulation thread + async gRPC server.
- `runtime.rs` — Simulation loop: cold-start ramp, `plant.step(dt)`, controller dispatch, CSV logging, ISD detection, snapshot-on-exit.
- `controllers/` — `Controller` trait + `ControllerBank`. Currently only `PController`. Implement `Controller`, call `bank.add(Box::new(...))` in `main.rs` to wire new loops.
- `grpc_server.rs` — tonic gRPC on `:50051`. Exposes: `StreamMetrics`, `GetPlantStatus`, `ListControllers`, `UpdateController`. Cannot add/remove controllers or set disturbances at runtime (those are compile-time / env-var configuration).
- `shared.rs` — `SharedState` behind `Arc<Mutex<_>>`: `ControllerBank` + `MetricsSnapshot`. Bridge between simulation thread and gRPC async tasks.
- `dashboard.rs` — ratatui TUI (disabled with `--headless`).

### Proto

`service/proto/tep/v1/plant.proto` — single source of truth for the gRPC API. Compiled by `tonic-build` at `build.rs`. Add or change RPCs here first, then update `grpc_server.rs`.

## Key Invariants

- **Plant is deterministic** — no control logic inside `core/`. All closed-loop logic lives in `service/controllers/`.
- **Controllers run after `plant.step(dt)`** — they read `xmeas` from the just-completed tick and write into `mv` which the next tick will consume. See `docs/01-premissas.md`.
- **ISD (Incipient Shutdown)** — detected when `deriv_norm == 0.0` and any alarm is active. Simulation loop freezes; gRPC keeps serving the last snapshot.
- **Snapshots** — TOML files under `cases/` (e.g. `te_exp3_snapshot.toml`) provide warm-start initial conditions. Written automatically on clean exit if `snapshot_path` is set in `Config`.
- **Disturbances** — set via `ACTIVE_IDV` env var (1-based index, comma-separated). Held off during cold-start ramp, then enabled when ramp completes.

## Adding a New Controller

1. Create `service/src/controllers/<name>.rs` implementing `trait Controller`.
2. `pub use` it in `controllers/mod.rs`.
3. Instantiate and `bank.add(Box::new(...))` in `main.rs`.

## Agent Rules

- Keep `core/` deterministic: no I/O, no threads, no wall-clock time, no async code, no logging side effects.
- Closed-loop control logic belongs in `service/controllers/`, not inside the TEP model equations.
- Do not change `plant.proto` without stating the impact on `tep-operator` and `tep-ihm`.
- If `plant.proto` changes, mention that stubs must be regenerated in all affected repositories.
- Do not run long simulations unless explicitly authorized; suggest the command instead.
- Do not run Docker, Kind, kubectl, deployment, or environment-changing commands unless explicitly authorized.
- Before editing, provide a short plan and list the files to be modified.
- Prefer minimal, localized changes over broad refactors.
- When fixing bugs, first identify whether the issue is in `core`, `service`, `controllers`, `grpc_server`, or runtime configuration.
- When touching numerical code, explain the expected effect on stability, determinism, or reproducibility.
- When adding a controller, implement the `Controller` trait and wire it through `ControllerBank`; do not special-case controller logic in the simulation loop.
- Preserve the tick order: `plant.step(dt)` first, then controllers read the completed `xmeas` and update `mv` for the next tick.
- Do not edit generated files or build artifacts.
- Keep responses short unless explicitly asked for a detailed analysis.
- For cross-repository decisions, follow the root `CLAUDE.md` / `AGENTS.md` propagation rules.