// tep/separator.rs
//
// Separator como DynamicModel real. Diferente do Reactor, precisa de um
// input de outro componente (reactor.temperature) — por isso `new()` declara
// esse `need` em subscribe() além dos `offers`, e guarda o Proxy resolvido
// (por StateRegistry::resolve(), chamado depois que todo mundo se inscreveu)
// como campo. evaluate() só lê/escreve via Proxy, nunca por nome.

use simulation_framework::dynamic_model::DynamicModel;
use simulation_framework::state_registry::{EvaluationState, Proxy, StateRegistry};

use crate::constants::TepConstants;
use crate::thermo::{liquid_density, temperature_from_enthalpy};

const SEPARATOR_VOLUME: f64 = 3500.0; // volume total do separador vapor/líquido [m³]
const GAS_CONSTANT: f64 = 998.9; // R em [mmHg·m³/(kmol·K)]

pub struct Separator {
    constants: TepConstants,
    reactor_temperature: Proxy,
    temperature: Proxy,
    pressure: Proxy,
    liquid_volume: Proxy,
    liquid_density: Proxy,
    total_vapor_kmol: Proxy,
    liquid_composition: Vec<Proxy>,
    vapor_composition: Vec<Proxy>,
}

impl Separator {
    pub fn new(registry: &mut StateRegistry) -> Self {
        let mut offer_keys: Vec<String> = vec![
            "separator.temperature".into(),
            "separator.pressure".into(),
            "separator.liquid_volume".into(),
            "separator.liquid_density".into(),
            "separator.total_vapor_kmol".into(),
        ];
        for i in 0..8 { offer_keys.push(format!("separator.liquid_composition.{i}")); }
        for i in 0..8 { offer_keys.push(format!("separator.vapor_composition.{i}")); }

        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();
        let (offered, requested) = registry.subscribe(&offer_refs, &["reactor.temperature"]);

        Self {
            constants: TepConstants::new(),
            reactor_temperature: requested[0].clone(),
            temperature: offered[0].clone(),
            pressure: offered[1].clone(),
            liquid_volume: offered[2].clone(),
            liquid_density: offered[3].clone(),
            total_vapor_kmol: offered[4].clone(),
            liquid_composition: offered[5..13].to_vec(),
            vapor_composition: offered[13..21].to_vec(),
        }
    }
}

impl DynamicModel for Separator {
    fn name(&self) -> &'static str {
        "Separator"
    }

    fn evaluate(&self, state: &[f64], eval: &EvaluationState) {
        let reactor_temperature = eval.get(&self.reactor_temperature);

        let mut vapor_moles = [0.0f64; 8];
        let mut liquid_moles = [0.0f64; 8];
        for i in 0..3 { vapor_moles[i] = state[i]; }
        for i in 3..8 { liquid_moles[i] = state[i]; }
        let total_enthalpy = state[8];

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 { liquid_composition[i] = liquid_moles[i] / total_liquid_moles; }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, reactor_temperature, specific_enthalpy, 0, &self.constants);
        let temperature_k = temperature + 273.15;
        let density = liquid_density(&liquid_composition, temperature, &self.constants);
        let volume_liquid = total_liquid_moles / density;
        let volume_vapor = SEPARATOR_VOLUME - volume_liquid;

        let mut partial_pressures = [0.0f64; 8];
        let mut pressure = 0.0f64;
        for i in 0..3 {
            partial_pressures[i] = vapor_moles[i] * GAS_CONSTANT * temperature_k / volume_vapor;
            pressure += partial_pressures[i];
        }
        for i in 3..8 {
            partial_pressures[i] = (self.constants.avp[i] + self.constants.bvp[i] / (temperature + self.constants.cvp[i])).exp() * liquid_composition[i];
            pressure += partial_pressures[i];
        }

        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 { vapor_composition[i] = partial_pressures[i] / pressure; }
        let total_vapor_moles = pressure * volume_vapor / GAS_CONSTANT / temperature_k;

        eval.set(&self.temperature, temperature);
        eval.set(&self.pressure, pressure);
        eval.set(&self.liquid_volume, volume_liquid);
        eval.set(&self.liquid_density, density);
        eval.set(&self.total_vapor_kmol, total_vapor_moles);
        for i in 0..8 {
            eval.set(&self.liquid_composition[i], liquid_composition[i]);
            eval.set(&self.vapor_composition[i], vapor_composition[i]);
        }
    }
}
