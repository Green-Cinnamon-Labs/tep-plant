// tep/stripper.rs
//
// Mesmo padrão do Separator: precisa de separator.temperature via Proxy.
// evaluate() não recebe nada — só lê/escreve via Proxy.

use simulation_framework::dynamic_model::DynamicModel;
use simulation_framework::state_registry::{Proxy, StateRegistry};

use crate::constants::TepConstants;
use crate::thermo::{liquid_density, temperature_from_enthalpy};

pub struct Stripper {
    constants: TepConstants,
    own_state: Vec<Proxy>, // uclc[0..8] líquido A-H, etc[8] entalpia
    separator_temperature: Proxy,
    temperature: Proxy,
    liquid_volume: Proxy,
    liquid_density: Proxy,
    liquid_composition: Vec<Proxy>,
}

impl Stripper {
    pub fn new(registry: &mut StateRegistry) -> Self {
        let mut offer_keys: Vec<String> = Vec::new();
        for i in 0..9 { offer_keys.push(format!("stripper.state.{i}")); }
        offer_keys.push("stripper.temperature".into());
        offer_keys.push("stripper.liquid_volume".into());
        offer_keys.push("stripper.liquid_density".into());
        for i in 0..8 { offer_keys.push(format!("stripper.liquid_composition.{i}")); }

        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();
        let (offered, requested) = registry.subscribe(&offer_refs, &["separator.temperature"]);

        Self {
            constants: TepConstants::new(),
            own_state: offered[0..9].to_vec(),
            separator_temperature: requested[0].clone(),
            temperature: offered[9].clone(),
            liquid_volume: offered[10].clone(),
            liquid_density: offered[11].clone(),
            liquid_composition: offered[12..20].to_vec(),
        }
    }
}

impl DynamicModel for Stripper {
    fn name(&self) -> &'static str {
        "Stripper"
    }

    fn evaluate(&self) {
        let separator_temperature = self.separator_temperature.get();

        let mut liquid_moles = [0.0f64; 8];
        for i in 0..8 { liquid_moles[i] = self.own_state[i].get(); }
        let total_enthalpy = self.own_state[8].get();

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 { liquid_composition[i] = liquid_moles[i] / total_liquid_moles; }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, separator_temperature, specific_enthalpy, 0, &self.constants);
        let density = liquid_density(&liquid_composition, temperature, &self.constants);
        let volume_liquid = total_liquid_moles / density;

        self.temperature.set(temperature);
        self.liquid_volume.set(volume_liquid);
        self.liquid_density.set(density);
        for i in 0..8 {
            self.liquid_composition[i].set(liquid_composition[i]);
        }
    }
}
