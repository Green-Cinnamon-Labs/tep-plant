# Tennessee Eastman Digital Twin Lab

Executable digital twin implementation of the Tennessee Eastman plant (TEP) in Rust, based on the classic Downs & Vogel (1993) model.

**Focus:** dynamic fidelity, deterministic simulation, clear separation between the plant model and the control layer.

---

## Repository Structure

```
tennessee-eastman-process/    ← original FORTRAN model (reference)
tennessee-eastman-service/    ← digital twin in Rust
  ├── core/                   ← plant's mathematical model (ODEs, integrator)
  └── service/                ← executable: runtime, controllers, dashboard
       └── src/
            ├── controllers/  ← Controller trait + ControllerBank + implementations
            ├── runtime.rs    ← simulation loop (plant + control + CSV logger)
            └── main.rs       ← experiment configuration and controller injection
analysis/                     ← Python package for visualization (CSV plots)
docs/                         ← technical documentation of the project
```

## Documentation

| File                                                     | Content                                                                     |
| ------------------------------------------------------- | ---------------------------------------------------------------------------- |
| [01-premissas.md](docs/01-premissas.md)                 | Modeling assumptions: valves, cold start, loop order, decoupling |
| [02-glossario.md](docs/02-glossario.md)                 | Glossary of TEP terms and nomenclature                                   |
| [03-falhas.md](docs/03-falhas.md)                       | Failure report and simulation troubleshooting                          |
| [04-experimentos.md](docs/04-experimentos.md)           | Scientific experiment log (Obs → Hyp → Int → Res → Conc)          |
| [05-disturbios.md](docs/05-disturbios.md)               | Reference for the TEP's 20 IDV disturbances                                     |
| [06-controle.md](docs/06-controle.md)                   | Control layer: injectable architecture, active loops, XMEAS/XMV         |
| [07-grpc-architecture.md](docs/07-grpc-architecture.md) | gRPC API for supervisory control via Kubernetes                          |

## Analysis and Visualization

The `analysis/` directory contains the `tep-analysis` Python package for generating plots from simulation CSVs. Details in [analysis/README.md](analysis/README.md).

```bash
cd analysis
poetry install
poetry run plot --csv ../tennessee-eastman-service/simulation_log.csv
```

## Architecture Principles

- The **plant is deterministic** and contains no control logic
- **Control is injectable** via the `Controller` trait + `ControllerBank`
- Time advances explicitly via numerical integration (RK4)
- Controllers are configurable without modifying the runtime
- gRPC server (tonic, `:50051`) allows runtime observation and reconciliation
- Future: external controller management via Kubernetes CRDs

## References

- Downs, J. J., & Vogel, E. F. (1993). *A Plant-Wide Industrial Process Control Problem*. Computers & Chemical Engineering, 17(3), 245-255.

## Status

Stable baseline (Exp 10/11, 20h without ISD). Decoupled and injectable controllers. Live gRPC API on `:50051` with StreamMetrics, List/Add/Update/RemoveController, and SetDisturbance. Next milestone: integrate with the Kubernetes operator (`tep-operator`).
