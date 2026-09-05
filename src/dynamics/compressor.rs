/* tep/dynamics/compressor.rs */


use crate::physics::constants::TepConstants;
use monjolo::chemistry::temperature_from_enthalpy;

const COMPRESSOR_VESSEL_VOLUME: f64 = 5000.0; /* volume do vaso do compressor/condensador [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */

#[monjolo::dynamic_model(after = ["Stripper"])]
pub struct Compressor {
    #[state]
    #[config(prefix = "state.compressor_vapor", components = ["A", "B", "C", "D", "E", "F", "G", "H"])]
    #[offer(prefix = "compressor.state", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    vapor: [f64; 8],

    #[state]
    #[config(key = "state.compressor.energy")]
    #[offer(key = "compressor.state.8")]
    enthalpy: f64,

    #[need(key = "separator.temperature")]
    separator_temperature: f64,

    #[offer(key = "compressor.temperature")]
    temperature: f64,
    #[offer(key = "compressor.pressure")]
    pressure: f64,

    #[offer(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    vapor_composition: [f64; 8],

    constants: TepConstants,
}

impl Compressor {
    fn compute(&self) {
        let separator_temperature = self.separator_temperature();

        let vapor_group = self.vapor();
        let mut vapor_moles = [0.0f64; 8];
        for i in 0..8 { vapor_moles[i] = vapor_group[i]; }
        let total_enthalpy = self.enthalpy();

        let total_vapor_moles: f64 = vapor_moles.iter().sum();
        let mut vapor_composition = [0.0f64; 8];
        for i in 0..8 { vapor_composition[i] = vapor_moles[i] / total_vapor_moles; }

        let specific_enthalpy = total_enthalpy / total_vapor_moles;
        let temperature = temperature_from_enthalpy(&vapor_composition, separator_temperature, specific_enthalpy, 2, &self.constants);
        let temperature_k = temperature + 273.15;
        let pressure = total_vapor_moles * GAS_CONSTANT * temperature_k / COMPRESSOR_VESSEL_VOLUME;

        self.set_temperature(temperature);
        self.set_pressure(pressure);
        self.set_vapor_composition(vapor_composition);

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

        assert_eq!(compressor.vapor(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(compressor.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let compressor = Compressor::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(compressor.vapor(), [0.0; 8]);
        assert_eq!(compressor.enthalpy(), 0.0);
    }
}
