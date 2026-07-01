// core/src/method/rk4.rs

use crate::dynamics::model::DynamicModel;
use crate::method::integrator::Integrator;
use crate::state::State;

pub struct RK4;

impl Integrator for RK4 {
    fn name(&self) -> &'static str {
        "RK4"
    }

    fn step(&self, model: &mut dyn DynamicModel, state: &mut State, dt: f64) {
        let n = state.x.len();

        // k1 = f(x)
        let k1 = model.derivatives(state);

        // k2 = f(x + dt/2 * k1)
        let mut s2 = State { x: state.x.clone() };
        for i in 0..n {
            s2.x[i] += 0.5 * dt * k1[i];
        }
        let k2 = model.derivatives(&s2);

        // k3 = f(x + dt/2 * k2)
        let mut s3 = State { x: state.x.clone() };
        for i in 0..n {
            s3.x[i] += 0.5 * dt * k2[i];
        }
        let k3 = model.derivatives(&s3);

        // k4 = f(x + dt * k3)
        let mut s4 = State { x: state.x.clone() };
        for i in 0..n {
            s4.x[i] += dt * k3[i];
        }
        let k4 = model.derivatives(&s4);

        // x = x + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
        for i in 0..n {
            state.x[i] += dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
    }
}
