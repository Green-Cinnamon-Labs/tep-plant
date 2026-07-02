// core/src/method/rk4.rs

use crate::model::DynamicModel;

pub struct RK4;

impl RK4 {
    pub fn step(&self, model: &mut dyn DynamicModel, state: &mut Vec<f64>, dt: f64) {
        let n = state.len();

        // k1 = f(x)
        let k1 = model.dynamics(state);

        // k2 = f(x + dt/2 * k1)
        let s2: Vec<f64> = (0..n).map(|i| state[i] + 0.5 * dt * k1[i]).collect();
        let k2 = model.dynamics(&s2);

        // k3 = f(x + dt/2 * k2)
        let s3: Vec<f64> = (0..n).map(|i| state[i] + 0.5 * dt * k2[i]).collect();
        let k3 = model.dynamics(&s3);

        // k4 = f(x + dt * k3)
        let s4: Vec<f64> = (0..n).map(|i| state[i] + dt * k3[i]).collect();
        let k4 = model.dynamics(&s4);

        // x = x + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
        for i in 0..n {
            state[i] += dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
    }
}
