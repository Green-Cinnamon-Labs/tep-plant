// tep/compressor.rs
//
// Mesmo padrão do Separator/Stripper: precisa de separator.temperature via
// Proxy.

use simulation_framework::dynamic_model::DynamicModel;
use simulation_framework::state_registry::{EvaluationState, Proxy, StateRegistry};

use crate::constants::TepConstants;
use crate::thermo::temperature_from_enthalpy;

const COMPRESSOR_VESSEL_VOLUME: f64 = 5000.0; // volume do vaso do compressor/condensador [m³]
const GAS_CONSTANT: f64 = 998.9; // R em [mmHg·m³/(kmol·K)]

pub struct Compressor {
    constants: TepConstants,
    separator_temperature: Proxy,
    temperature: Proxy,
    pressure: Proxy,
    vapor_composition: Vec<Proxy>,
}

impl Compressor {
    pub fn new(registry: &mut StateRegistry) -> Self {
        let mut offer_keys: Vec<String> = vec![
            "compressor.temperature".into(),
            "compressor.pressure".into(),
        ];
        for i in 0..8 { offer_keys.push(format!("compressor.vapor_composition.{i}")); }

        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();
        let (offered, requested) = registry.subscribe(&offer_refs, &["separator.temperature"]);

        Self {
            constants: TepConstants::new(),
            separator_temperature: requested[0].clone(),
            temperature: offered[0].clone(),
            pressure: offered[1].clone(),
            vapor_composition: offered[2..10].to_vec(),
        }
    }
}

impl DynamicModel for Compressor {
    fn name(&self) -> &'static str {
        "Compressor"
    }

    fn evaluate(&self, state: &[f64], eval: &EvaluationState) {
        let separator_temperature = eval.get(&self.separator_temperature);

        let mut vapor_moles = [0.0f64; 8];
        for i in 0..8 { vapor_moles[i] = state[i]; }
        let total_enthalpy = state[8];

        let total_vapor_moles: f64 = vapor_moles.iter().sum();
        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 { vapor_composition[i] = vapor_moles[i] / total_vapor_moles; }

        let specific_enthalpy = total_enthalpy / total_vapor_moles;
        let temperature = temperature_from_enthalpy(&vapor_composition, separator_temperature, specific_enthalpy, 2, &self.constants);
        let temperature_k = temperature + 273.15;
        let pressure = total_vapor_moles * GAS_CONSTANT * temperature_k / COMPRESSOR_VESSEL_VOLUME;

        eval.set(&self.temperature, temperature);
        eval.set(&self.pressure, pressure);
        for i in 0..8 {
            eval.set(&self.vapor_composition[i], vapor_composition[i]);
        }
    }
}
