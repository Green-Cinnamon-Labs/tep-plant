// shared.rs — bridge between simulation thread and gRPC server

use std::sync::{Arc, Mutex};
use crate::controllers::ControllerBank;

/// Snapshot of plant metrics written by the simulation thread each tick.
#[derive(Clone)]
pub struct MetricsSnapshot {
    pub t_h: f64,
    pub xmeas: Vec<f64>,
    pub xmv: Vec<f64>,
    pub alarms: Vec<AlarmSnapshot>,
    pub deriv_norm: f64,
    pub isd_active: bool,
}

#[derive(Clone)]
pub struct AlarmSnapshot {
    pub variable: String,
    pub condition: String,
    pub active: bool,
}

/// Shared state protected by a mutex.
pub struct SharedState {
    pub bank: ControllerBank,
    pub metrics: MetricsSnapshot,
    pub active_idv: Vec<usize>,
    pub paused: bool,
}

impl SharedState {
    pub fn new(bank: ControllerBank) -> Self {
        Self {
            bank,
            metrics: MetricsSnapshot {
                t_h: 0.0,
                xmeas: vec![0.0; 22],
                xmv: vec![0.0; 12],
                alarms: Vec::new(),
                deriv_norm: 0.0,
                isd_active: false,
            },
            active_idv: Vec::new(),
            paused: false,
        }
    }
}

pub type SharedPlant = Arc<Mutex<SharedState>>;
