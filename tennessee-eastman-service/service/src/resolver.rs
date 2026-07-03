// service/src/resolver.rs

use te_core::model::DynamicModel;
use te_core::tep::TennesseeEastmanModel;
use te_core::tep::initial_state::InitialState;
use te_core::method::integrator::Integrator;
use te_core::method::rk4::RK4;

use crate::config::{Config, ModelKind, IntegratorKind};

pub struct ResolvedPlant {
    pub model: Box<dyn DynamicModel>,
    pub integrator: Box<dyn Integrator>,
    pub initial_state: Vec<f64>,
}

pub fn resolve(config: &Config) -> ResolvedPlant {
    let integrator = resolve_integrator(&config.integrator);

    match &config.model {
        ModelKind::TennesseeEastman => {
            let initial = InitialState::from_file(&config.initial_state_path).unwrap();
            let flat = initial.flatten().to_vec();
            let model = Box::new(TennesseeEastmanModel::new(&initial));
            ResolvedPlant { model, integrator, initial_state: flat }
        }
    }
}

fn resolve_integrator(kind: &IntegratorKind) -> Box<dyn Integrator> {
    match kind {
        IntegratorKind::RK4 => Box::new(RK4),
    }
}
