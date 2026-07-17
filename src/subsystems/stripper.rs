// tep/stripper.rs
//
// Mesmo padrão do Separator: precisa de separator.temperature via Proxy.
// evaluate() não recebe nada — só lê/escreve via Proxy.

use monjolo::dynamic_model::DynamicModel;
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::{Proxy, StateRegistry};

use crate::physics::constants::TepConstants;
use crate::physics::thermo::{liquid_density, temperature_from_enthalpy};

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
    pub fn new(registry: &mut StateRegistry, initial: &Snapshot) -> Self {
        let mut offer_keys: Vec<String> = Vec::new();
        for i in 0..9 { offer_keys.push(format!("stripper.state.{i}")); }
        offer_keys.push("stripper.temperature".into());
        offer_keys.push("stripper.liquid_volume".into());
        offer_keys.push("stripper.liquid_density".into());
        for i in 0..8 { offer_keys.push(format!("stripper.liquid_composition.{i}")); }

        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();
        let (offered, requested) = registry.subscribe(&offer_refs, &["separator.temperature"]);

        // Semeia o estado próprio com a condição inicial recebida — mesma
        // ordem de offer_keys: líquido A-H, entalpia. Chave ausente no
        // Snapshot vira 0.0 (mesmo default que o slot já teria).
        offered[0].set(initial.get("state.stripper_liquid.A").unwrap_or(0.0));
        offered[1].set(initial.get("state.stripper_liquid.B").unwrap_or(0.0));
        offered[2].set(initial.get("state.stripper_liquid.C").unwrap_or(0.0));
        offered[3].set(initial.get("state.stripper_liquid.D").unwrap_or(0.0));
        offered[4].set(initial.get("state.stripper_liquid.E").unwrap_or(0.0));
        offered[5].set(initial.get("state.stripper_liquid.F").unwrap_or(0.0));
        offered[6].set(initial.get("state.stripper_liquid.G").unwrap_or(0.0));
        offered[7].set(initial.get("state.stripper_liquid.H").unwrap_or(0.0));
        offered[8].set(initial.get("state.stripper.energy").unwrap_or(0.0));

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
            ("state.stripper_liquid.A", 1.0),
            ("state.stripper_liquid.B", 2.0),
            ("state.stripper_liquid.C", 3.0),
            ("state.stripper_liquid.D", 4.0),
            ("state.stripper_liquid.E", 5.0),
            ("state.stripper_liquid.F", 6.0),
            ("state.stripper_liquid.G", 7.0),
            ("state.stripper_liquid.H", 8.0),
            ("state.stripper.energy", 42.0),
        ]);

        let stripper = Stripper::new(&mut registry.borrow_mut(), &initial);

        let values: Vec<f64> = stripper.own_state.iter().map(Proxy::get).collect();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 42.0]);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let stripper = Stripper::new(&mut registry.borrow_mut(), &initial);

        let values: Vec<f64> = stripper.own_state.iter().map(Proxy::get).collect();
        assert_eq!(values, vec![0.0; 9]);
    }
}
