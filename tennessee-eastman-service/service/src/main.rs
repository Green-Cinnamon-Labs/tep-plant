// main.rs

mod config;
mod controllers;
mod dashboard;
mod grpc_server;
mod resolver;
mod runtime;
mod shared;

use std::sync::{Arc, Mutex};

use config::{Config, ModelKind, IntegratorKind};
use controllers::PController;
use shared::SharedState;
use grpc_server::PlantServiceImpl;
use grpc_server::pb::plant_service_server::PlantServiceServer;

#[tokio::main]
async fn main() {

    let headless = std::env::args().any(|a| a == "--headless");

    // ── Env var: STEP_DELAY_MS ────────────────────────────────────────────────
    // Controls simulation speed relative to real time.
    // 0 (default) = as fast as CPU allows.
    // 36           = 100x real time  (1 simulated hour = 36s wall clock)
    // 360          = 10x  real time  (1 simulated hour =  6min wall clock)
    let step_delay_ms: u64 = std::env::var("STEP_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let real_time     = step_delay_ms > 0;
    let step_delay_secs = step_delay_ms as f64 / 1000.0;

    if real_time {
        eprintln!("Simulation speed: {:.0}x real time (STEP_DELAY_MS={})",
            3.6 / step_delay_secs, step_delay_ms);
    } else {
        eprintln!("Simulation speed: maximum CPU (STEP_DELAY_MS not set)");
    }

    // Disturbances are now controlled via gRPC UpdateDisturbances endpoint (no startup config)
    let active_idv: Vec<usize> = vec![];

    let config = Config {
        dt: 0.001,
        real_time,
        step_delay_secs,
        initial_state_path: "cases/te_exp3_snapshot.toml".into(),
        model: ModelKind::TennesseeEastman,
        integrator: IntegratorKind::RK4,
        ramp_duration: 0.0,
        active_idv,
        max_sim_time_h: None,
        snapshot_path: Some("cases/te_exp11_snapshot.toml".into()),
        headless,
    };

    // ── Controllers: 3-loop baseline — parameters identical to Exp 10 ─────────
    let mut bank = controllers::ControllerBank::default();
    bank.add(Box::new(PController::new("pressure_reactor", 6,  5, 0.1, 2705.0, 40.06))); // reactor P → purge
    bank.add(Box::new(PController::new("level_separator",  11, 6, 1.0, 50.0,   38.1 ))); // sep level → underflow
    bank.add(Box::new(PController::new("level_stripper",   14, 7, 1.0, 50.0,   46.5 ))); // strip level → product

    let shared = Arc::new(Mutex::new(SharedState::new(bank)));

    // ── Simulation thread ─────────────────────────────────────────────────────
    let sim_shared = shared.clone();
    let sim_handle = std::thread::spawn(move || {
        runtime::run(config, sim_shared);
    });

    // ── gRPC server ───────────────────────────────────────────────────────────
    let addr = "[::]:50051".parse().unwrap();
    let svc = PlantServiceImpl::new(shared.clone());

    eprintln!("gRPC server listening on {}", addr);

    tonic::transport::Server::builder()
        .add_service(PlantServiceServer::new(svc))
        .serve_with_shutdown(addr, async {
            // Shutdown when the simulation thread finishes
            tokio::task::spawn_blocking(move || {
                let _ = sim_handle.join();
            })
            .await
            .ok();
        })
        .await
        .unwrap();
    
}
