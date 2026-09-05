/* tep/dynamics/stripper.rs */


use crate::physics::constants::TepConstants;
use monjolo::chemistry::{liquid_density, temperature_from_enthalpy};

#[monjolo::dynamic_model(after = ["Separator"])]
pub struct Stripper {
    #[state]
    #[config(prefix = "state.stripper_liquid", components = ["A", "B", "C", "D", "E", "F", "G", "H"])]
    #[offer(prefix = "stripper.state", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    liquid: [f64; 8],

    #[state]
    #[config(key = "state.stripper.energy")]
    #[offer(key = "stripper.state.8")]
    enthalpy: f64,

    #[need(key = "separator.temperature")]
    separator_temperature: f64,

    #[offer(key = "stripper.temperature")]
    temperature: f64,
    #[offer(key = "stripper.liquid_volume")]
    liquid_volume: f64,
    #[offer(key = "stripper.liquid_density")]
    liquid_density: f64,

    #[offer(prefix = "stripper.liquid_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    liquid_composition: [f64; 8],

    constants: TepConstants,
}

impl Stripper {
    fn compute(&self) {
        let separator_temperature = self.separator_temperature();

        let liquid_group = self.liquid();
        let mut liquid_moles = [0.0f64; 8];
        for i in 0..8 { liquid_moles[i] = liquid_group[i]; }
        let total_enthalpy = self.enthalpy();

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 { liquid_composition[i] = liquid_moles[i] / total_liquid_moles; }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, separator_temperature, specific_enthalpy, 0, &self.constants);
        let density = liquid_density(&liquid_composition, temperature, &self.constants);
        let volume_liquid = total_liquid_moles / density;

        self.set_temperature(temperature);
        self.set_liquid_volume(volume_liquid);
        self.set_liquid_density(density);
        self.set_liquid_composition(liquid_composition);

        /* Decisão de modelagem: a derivada real do próprio estado (yp — quanto own_state muda por
        tempo) não é calculada aqui. Quem calcula é `Flows`, uma DynamicModel separada que roda
        depois deste na sequência (só ela tem os 4 subsistemas termodinâmicos ao mesmo tempo,
        necessário pra saber o que entra/sai daqui) — `Flows::evaluate()` escreve direto nos slots
        de derivada deste componente. Este `evaluate()` só produz valores termodinâmicos derivados
        do estado atual (temperatura, pressão, composição etc.), nunca a derivada do estado em si.
        */
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

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

        assert_eq!(stripper.liquid(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(stripper.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let stripper = Stripper::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(stripper.liquid(), [0.0; 8]);
        assert_eq!(stripper.enthalpy(), 0.0);
    }
}
