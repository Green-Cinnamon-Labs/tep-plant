/* tep/dynamics/separator.rs */


use crate::physics::constants::TepConstants;
use monjolo::chemistry::{liquid_density, temperature_from_enthalpy};

const SEPARATOR_VOLUME: f64 = 3500.0; /* volume total do separador vapor/líquido [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */

#[monjolo::dynamic_model(after = ["Reactor"])]
pub struct Separator {
    /* Estado próprio (9 números) — mesmo split de Reactor, mesmo motivo (chave de config não
    uniforme entre vapor/líquido e entalpia).
    */
    #[state]
    #[config(prefix = "state.separator_vapor", components = ["A", "B", "C"])]
    #[offer(prefix = "separator.state", components = ["vapor_a", "vapor_b", "vapor_c"])]
    vapor: [f64; 3],

    #[state]
    #[config(prefix = "state.separator_vapor", components = ["D", "E", "F", "G", "H"])]
    #[offer(prefix = "separator.state", components = ["liquid_d", "liquid_e", "liquid_f", "liquid_g", "liquid_h"])]
    liquid: [f64; 5],

    #[state]
    #[config(key = "state.separator.energy")]
    #[offer(key = "separator.state.enthalpy")]
    enthalpy: f64,

    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,

    #[offer(key = "separator.temperature")]
    temperature: f64,
    #[offer(key = "separator.pressure")]
    pressure: f64,
    #[offer(key = "separator.liquid_volume")]
    liquid_volume: f64,
    #[offer(key = "separator.liquid_density")]
    liquid_density: f64,
    #[offer(key = "separator.total_vapor_kmol")]
    total_vapor_kmol: f64,

    #[offer(prefix = "separator.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    liquid_composition: [f64; 8],
    #[offer(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    vapor_composition: [f64; 8],

    constants: TepConstants,
}

impl Separator {
    fn compute(&self) {
        let reactor_temperature = self.reactor_temperature();

        let vapor_group = self.vapor();
        let liquid_group = self.liquid();
        let mut vapor_moles = [0.0f64; 8];
        let mut liquid_moles = [0.0f64; 8];
        for i in 0..3 { vapor_moles[i] = vapor_group[i]; }
        for i in 3..8 { liquid_moles[i] = liquid_group[i - 3]; }
        let total_enthalpy = self.enthalpy();

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

        self.set_temperature(temperature);
        self.set_pressure(pressure);
        self.set_liquid_volume(volume_liquid);
        self.set_liquid_density(density);
        self.set_total_vapor_kmol(total_vapor_moles);
        self.set_liquid_composition(liquid_composition);
        self.set_vapor_composition(vapor_composition);

        /* [DECISÃO DE MODELAGEM]: a derivada real do próprio estado (yp — quanto own_state muda por
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
            ("state.separator_vapor.A", 1.0),
            ("state.separator_vapor.B", 2.0),
            ("state.separator_vapor.C", 3.0),
            ("state.separator_vapor.D", 4.0),
            ("state.separator_vapor.E", 5.0),
            ("state.separator_vapor.F", 6.0),
            ("state.separator_vapor.G", 7.0),
            ("state.separator_vapor.H", 8.0),
            ("state.separator.energy", 42.0),
        ]);

        let separator = Separator::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(separator.vapor(), [1.0, 2.0, 3.0]);
        assert_eq!(separator.liquid(), [4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(separator.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let separator = Separator::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(separator.vapor(), [0.0; 3]);
        assert_eq!(separator.liquid(), [0.0; 5]);
        assert_eq!(separator.enthalpy(), 0.0);
    }
}
