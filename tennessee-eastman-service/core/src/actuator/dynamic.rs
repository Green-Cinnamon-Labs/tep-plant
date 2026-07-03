// actuator/dynamic.rs

use crate::model::DynamicModel;

// ── Valve ───────────────────────────────────────────────────────────
// Models a control valve with first-order lag: d(position)/dt = (command - position) / tau
// State: [position]  (one variable: current valve position, 0–100 %)

pub struct Valve {
    pub tau: f64,
    pub command: f64,
}

impl Valve {
    pub fn new(tau: f64) -> Self {
        Self { tau, command: 0.0 }
    }

    pub fn set_command(&mut self, command: f64) {
        self.command = command;
    }

    pub fn position(state: &[f64]) -> f64 {
        state[0]
    }
}

impl DynamicModel for Valve {
    fn state_size(&self) -> usize {
        1
    }

    fn dynamics(&mut self, state: &[f64]) -> Vec<f64> {
        let position = state[0];
        vec![(self.command - position) / self.tau]
    }

    fn name(&self) -> &'static str {
        "Valve"
    }
}

// ── Agitator ──────────────────────────────────────────────────────────────────
// Reactor agitator speed — same first-order dynamics as a valve but
// controls mixing intensity (agsp), not fluid flow.
// State: [speed]  (one variable: current agitator speed, 0–100 %)

pub struct Agitator {
    pub tau: f64,
    pub command: f64,
}

impl Agitator {
    pub fn new(tau: f64) -> Self {
        Self { tau, command: 0.0 }
    }

    pub fn set_command(&mut self, command: f64) {
        self.command = command;
    }

    pub fn speed(state: &[f64]) -> f64 {
        state[0]
    }
}

impl DynamicModel for Agitator {
    fn state_size(&self) -> usize {
        1
    }

    fn dynamics(&mut self, state: &[f64]) -> Vec<f64> {
        let speed = state[0];
        vec![(self.command - speed) / self.tau]
    }

    fn name(&self) -> &'static str {
        "Agitator"
    }
}
