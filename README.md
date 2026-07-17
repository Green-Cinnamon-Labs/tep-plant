# Tennessee Eastman CPS Lab

Executable Cyber-Physical System (CPS) implementation of the Tennessee Eastman plant (TEP) in Rust, based on the classic Downs & Vogel (1993) model. Not a "digital twin" — there is no physical TEP plant this simulator mirrors.

**Focus:** dynamic fidelity, deterministic simulation, clear separation between the TEP-specific chemistry and the generic simulation runtime.

---

## Architecture

`tep-plant` is a thin, TEP-specific crate on top of [`monjolo`](https://github.com/Green-Cinnamon-Labs/monjolo), a sibling repository that provides the generic simulation runtime: model composition (`DynamicModel`/`CompositeDynamicModel`), the `StateRegistry`, the RK4 integrator, reusable actuator/sensor/disturbance blocks, and the OPC-UA adapter. `tep-plant` itself knows nothing about networking, threads, or protocols — it only knows the TEP's chemistry, thermodynamics, and subsystem topology (reactor, separator, stripper, compressor). See `monjolo`'s own README for what it provides, and [`spec-tennessee-eastman/CONTRIBUTING.md`](https://github.com/Green-Cinnamon-Labs/spec-tennessee-eastman/blob/main/CONTRIBUTING.md) for the full, numbered record of the architecture decisions behind this split.

The plant is exposed over **OPC-UA** (`opc.tcp://0.0.0.0:4840/tep/server/` by default) via `monjolo`'s adapter — there is no gRPC server in this codebase anymore; the earlier gRPC/`ControllerBank`-based architecture (still described in `docs/07-controle.md` and `docs/08-grpc-architecture.md`, kept for historical reference) was superseded by this OPC-UA-based composite model when the `composite` branch was merged into `main`.

---

## Repository Structure

```
src/
  ├── lib.rs                 ← module declarations only; TEP-specific code, no framework logic
  ├── model.rs                ← TennesseeEastmanModel — the composite DynamicModel, wires subsystems + declares sensors
  ├── subsystems/              ← the plant's chemical blocks (Reactor, Separator, Stripper, Compressor, plus
  │                              Flows/Heat/Measurements — currently unimplemented stubs, see Status below)
  ├── disturbance/             ← Disturbance and its internal state (IDV 1-20)
  ├── physics/                 ← shared thermodynamic constants and correlations
  └── bin/tep_plant.rs         ← the actual binary: builds the Simulation, loads the initial snapshot, configures OPC-UA
docs/
  ├── 01-09-*.md               ← technical documentation (see table below)
  ├── cases/                   ← initial-condition snapshots (TOML)
  ├── fortran-original/        ← original FORTRAN reference model (Downs & Vogel)
  └── _deprecated_*.rs          ← physics preserved as comments from the pre-refactor code, not part of the crate
```

## Documentation

| File                                                     | Content                                                                     |
| --------------------------------------------------------- | ---------------------------------------------------------------------------- |
| [01-premissas.md](docs/01-premissas.md)                 | Modeling assumptions: valves, cold start, loop order, decoupling            |
| [02-glossario.md](docs/02-glossario.md)                 | Glossary of TEP terms and nomenclature                                      |
| [03-falhas.md](docs/03-falhas.md)                       | Failure report and simulation troubleshooting                               |
| [05-disturbios.md](docs/05-disturbios.md)               | Reference for the TEP's 20 IDV disturbances                                 |
| [06-ruidos.md](docs/06-ruidos.md)                       | Measurement noise model (Gaussian, per-XMEAS)                               |
| [07-controle.md](docs/07-controle.md)                   | ⚠️ Historical — describes the pre-merge gRPC/`ControllerBank` control layer, no longer present in `main` |
| [08-grpc-architecture.md](docs/08-grpc-architecture.md) | ⚠️ Historical — describes the pre-merge gRPC API; the plant is exposed via OPC-UA now |
| [09-diagrama-variaveis.md](docs/09-diagrama-variaveis.md) | Canonical reference for the TEP diagram and its variables (XMEAS/XMV mapping) |

For the current architecture (composition, `StateRegistry`, sensors, OPC-UA adapter), the authoritative source is [`spec-tennessee-eastman/CONTRIBUTING.md`](https://github.com/Green-Cinnamon-Labs/spec-tennessee-eastman/blob/main/CONTRIBUTING.md), not the docs above.

## Build & Run

```bash
cargo build --release --bin tep-plant
cargo run --bin tep-plant
```

Connects an OPC-UA client to `opc.tcp://0.0.0.0:4840/tep/server/`.

```bash
# Docker: assumes the binary was already built locally (see Dockerfile)
cargo build --release --bin tep-plant
docker build -t tep-plant .
docker run --rm -p 4840:4840 tep-plant
```

## References

- Downs, J. J., & Vogel, E. F. (1993). *A Plant-Wide Industrial Process Control Problem*. Computers & Chemical Engineering, 17(3), 245-255.

## Status

The `composite` branch (OPC-UA over `monjolo`) has been merged into `main` — this is now the only architecture in this repository, the earlier gRPC service is gone. Reactor/Separator/Stripper/Compressor are wired and integrated by RK4; `Flows`/`Heat`/`Measurements` (the subsystems that would close the chemical core's derivatives, and produce the rest of XMEAS) are still `todo!()` stubs, so `own_state` for the four wired subsystems stays frozen at its seeded initial value. The OPC-UA server comes up and is browsable, but reading a node's `Value` currently returns `BadNodeIdUnknown` (pre-existing bug isolated to the `async-opcua` 0.18 dependency, not a regression of this merge). No supervisory Controller layer exists yet. Full status and next steps: see issues [#55](https://github.com/Green-Cinnamon-Labs/spec-tennessee-eastman/issues/55)-[#58](https://github.com/Green-Cinnamon-Labs/spec-tennessee-eastman/issues/58) and `CONTRIBUTING.md` in `spec-tennessee-eastman`.
