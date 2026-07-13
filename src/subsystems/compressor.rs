// tep/compressor.rs
//
// Mesmo padrão do Separator/Stripper: precisa de separator.temperature via
// Proxy. evaluate() não recebe nada — só lê/escreve via Proxy.

use monjolo::dynamic_model::DynamicModel;
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::{Proxy, StateRegistry};

use crate::physics::constants::TepConstants;
use crate::physics::thermo::temperature_from_enthalpy;

const COMPRESSOR_VESSEL_VOLUME: f64 = 5000.0; // volume do vaso do compressor/condensador [m³]
const GAS_CONSTANT: f64 = 998.9; // R em [mmHg·m³/(kmol·K)]

pub struct Compressor {
    constants: TepConstants,
    own_state: Vec<Proxy>, // ucvv[0..8] vapor A-H, etv[8] entalpia
    separator_temperature: Proxy,
    temperature: Proxy,
    pressure: Proxy,
    vapor_composition: Vec<Proxy>,
}

impl Compressor {
    pub fn new(registry: &mut StateRegistry, initial: &Snapshot) -> Self {
        let mut offer_keys: Vec<String> = Vec::new();
        for i in 0..9 { offer_keys.push(format!("compressor.state.{i}")); }
        offer_keys.push("compressor.temperature".into());
        offer_keys.push("compressor.pressure".into());
        for i in 0..8 { offer_keys.push(format!("compressor.vapor_composition.{i}")); }

        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();
        let (offered, requested) = registry.subscribe(&offer_refs, &["separator.temperature"]);

        // Semeia o estado próprio com a condição inicial recebida — mesma
        // ordem de offer_keys: vapor A-H, entalpia. Chave ausente no
        // Snapshot vira 0.0 (mesmo default que o slot já teria).
        offered[0].set(initial.get("state.compressor_vapor.A").unwrap_or(0.0));
        offered[1].set(initial.get("state.compressor_vapor.B").unwrap_or(0.0));
        offered[2].set(initial.get("state.compressor_vapor.C").unwrap_or(0.0));
        offered[3].set(initial.get("state.compressor_vapor.D").unwrap_or(0.0));
        offered[4].set(initial.get("state.compressor_vapor.E").unwrap_or(0.0));
        offered[5].set(initial.get("state.compressor_vapor.F").unwrap_or(0.0));
        offered[6].set(initial.get("state.compressor_vapor.G").unwrap_or(0.0));
        offered[7].set(initial.get("state.compressor_vapor.H").unwrap_or(0.0));
        offered[8].set(initial.get("state.compressor.energy").unwrap_or(0.0));

        Self {
            constants: TepConstants::new(),
            own_state: offered[0..9].to_vec(),
            separator_temperature: requested[0].clone(),
            temperature: offered[9].clone(),
            pressure: offered[10].clone(),
            vapor_composition: offered[11..19].to_vec(),
        }
    }
}

impl DynamicModel for Compressor {
    fn name(&self) -> &'static str {
        "Compressor"
    }

    fn evaluate(&self) {
        let separator_temperature = self.separator_temperature.get();

        let mut vapor_moles = [0.0f64; 8];
        for i in 0..8 { vapor_moles[i] = self.own_state[i].get(); }
        let total_enthalpy = self.own_state[8].get();

        let total_vapor_moles: f64 = vapor_moles.iter().sum();
        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 { vapor_composition[i] = vapor_moles[i] / total_vapor_moles; }

        let specific_enthalpy = total_enthalpy / total_vapor_moles;
        let temperature = temperature_from_enthalpy(&vapor_composition, separator_temperature, specific_enthalpy, 2, &self.constants);
        let temperature_k = temperature + 273.15;
        let pressure = total_vapor_moles * GAS_CONSTANT * temperature_k / COMPRESSOR_VESSEL_VOLUME;

        self.temperature.set(temperature);
        self.pressure.set(pressure);
        for i in 0..8 {
            self.vapor_composition[i].set(vapor_composition[i]);
        }

        // Decisão de modelagem: a derivada real do próprio estado (yp —
        // quanto own_state muda por tempo) não é calculada aqui. Quem
        // calcula é `Flows`, uma DynamicModel separada que roda depois
        // deste na sequência (só ela tem os 4 subsistemas termodinâmicos ao
        // mesmo tempo, necessário pra saber o que entra/sai daqui) —
        // `Flows::evaluate()` escreve direto nos slots de derivada deste
        // componente. Este `evaluate()` só produz valores termodinâmicos
        // derivados do estado atual (temperatura, pressão, composição
        // etc.), nunca a derivada do estado em si.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_seeds_own_state_with_initial_condition() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[
            ("state.compressor_vapor.A", 1.0),
            ("state.compressor_vapor.B", 2.0),
            ("state.compressor_vapor.C", 3.0),
            ("state.compressor_vapor.D", 4.0),
            ("state.compressor_vapor.E", 5.0),
            ("state.compressor_vapor.F", 6.0),
            ("state.compressor_vapor.G", 7.0),
            ("state.compressor_vapor.H", 8.0),
            ("state.compressor.energy", 42.0),
        ]);

        let compressor = Compressor::new(&mut registry.borrow_mut(), &initial);

        let values: Vec<f64> = compressor.own_state.iter().map(Proxy::get).collect();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 42.0]);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let compressor = Compressor::new(&mut registry.borrow_mut(), &initial);

        let values: Vec<f64> = compressor.own_state.iter().map(Proxy::get).collect();
        assert_eq!(values, vec![0.0; 9]);
    }
}
