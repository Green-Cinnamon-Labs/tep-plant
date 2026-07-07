// tep/stripper.rs
//
// Mesmo padrão do Separator: precisa de separator.temperature via Proxy.

use simulation_framework::dynamic_model::DynamicModel;
use simulation_framework::state_registry::{EvaluationState, Proxy, StateRegistry};

use crate::constants::TepConstants;
use crate::thermo::{liquid_density, temperature_from_enthalpy};

pub struct Stripper {
    constants: TepConstants,
    separator_temperature: Proxy,
    temperature: Proxy,
    liquid_volume: Proxy,
    liquid_density: Proxy,
    liquid_composition: Vec<Proxy>,
}

impl Stripper {
    pub fn new(registry: &mut StateRegistry) -> Self {
        let mut offer_keys: Vec<String> = vec![
            "stripper.temperature".into(),
            "stripper.liquid_volume".into(),
            "stripper.liquid_density".into(),
        ];
        for i in 0..8 { offer_keys.push(format!("stripper.liquid_composition.{i}")); }

        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();
        let (offered, requested) = registry.subscribe(&offer_refs, &["separator.temperature"]);

        Self {
            constants: TepConstants::new(),
            separator_temperature: requested[0].clone(),
            temperature: offered[0].clone(),
            liquid_volume: offered[1].clone(),
            liquid_density: offered[2].clone(),
            liquid_composition: offered[3..11].to_vec(),
        }
    }
}

impl DynamicModel for Stripper {
    fn name(&self) -> &'static str {
        "Stripper"
    }

    fn evaluate(&self, state: &[f64], eval: &EvaluationState) {
        let separator_temperature = eval.get(&self.separator_temperature);

        let mut liquid_moles = [0.0f64; 8];
        for i in 0..8 { liquid_moles[i] = state[i]; }
        let total_enthalpy = state[8];

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 { liquid_composition[i] = liquid_moles[i] / total_liquid_moles; }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, separator_temperature, specific_enthalpy, 0, &self.constants);
        let density = liquid_density(&liquid_composition, temperature, &self.constants);
        let volume_liquid = total_liquid_moles / density;

        eval.set(&self.temperature, temperature);
        eval.set(&self.liquid_volume, volume_liquid);
        eval.set(&self.liquid_density, density);
        for i in 0..8 {
            eval.set(&self.liquid_composition[i], liquid_composition[i]);
        }
    }
}
