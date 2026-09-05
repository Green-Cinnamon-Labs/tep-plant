/* tep/dynamics/reactor.rs */


use crate::physics::constants::TepConstants;
use monjolo::chemistry::{liquid_density, temperature_from_enthalpy};

const REACTOR_VOLUME: f64 = 1300.0; /* volume total do vaso do reator [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
const REACTION_ENTHALPIES: [f64; 2] = [0.06899381054, 0.05]; /* calor das reações 1 e 2 [kJ/kmol] */

const REACTION_FACTOR_1_NOMINAL: f64 = 1.0; /* TODO: devia ser um `need` do Disturbance (IDV 13) */
const REACTION_FACTOR_2_NOMINAL: f64 = 1.0; /* TODO: devia ser um `need` do Disturbance (IDV 13) */
const TEMPERATURE_SEED: f64 = 120.0; /* seed do Newton-Raphson — não afeta a raiz */

#[monjolo::dynamic_model]
pub struct Reactor {
    /* Estado próprio (9 números: vapor A/B/C, líquido D-H, entalpia total) — o que antes vinha de
    `state: &[f64]`, depois virou `own_state: Vec<Proxy>`. Split em 3 campos porque a chave de
    CONFIG não é uniforme entre os 9 (vapor/líquido usam "state.reactor_vapor.*", entalpia usa
    "state.reactor.energy" — um grupo por padrão de chave, não por fase físico-química).
    */
    #[state]
    #[config(prefix = "state.reactor_vapor", components = ["A", "B", "C"])]
    #[offer(prefix = "reactor.state", components = ["vapor_a", "vapor_b", "vapor_c"])]
    vapor: [f64; 3],

    #[state]
    #[config(prefix = "state.reactor_vapor", components = ["D", "E", "F", "G", "H"])]
    #[offer(prefix = "reactor.state", components = ["liquid_d", "liquid_e", "liquid_f", "liquid_g", "liquid_h"])]
    liquid: [f64; 5],

    #[state]
    #[config(key = "state.reactor.energy")]
    #[offer(key = "reactor.state.enthalpy")]
    enthalpy: f64,

    #[offer(key = "reactor.temperature")]
    temperature: f64,
    #[offer(key = "reactor.temperature_k")]
    temperature_k: f64,
    #[offer(key = "reactor.pressure")]
    pressure: f64,
    #[offer(key = "reactor.liquid_volume")]
    liquid_volume: f64,
    #[offer(key = "reactor.liquid_density")]
    liquid_density: f64,
    #[offer(key = "reactor.vapor_volume")]
    vapor_volume: f64,
    #[offer(key = "reactor.total_vapor_kmol")]
    total_vapor_kmol: f64,
    #[offer(key = "reactor.heat_of_reaction")]
    heat_of_reaction: f64,

    /* Composição líquida/vapor, kmol em fase vapor e taxa líquida de consumo/produção — um valor
    por componente (A-H), mesma convenção de chave de sempre (`reactor.liquid_composition.a` etc.).
    */
    #[offer(prefix = "reactor.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    liquid_composition: [f64; 8],
    #[offer(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    vapor_composition: [f64; 8],
    #[offer(prefix = "reactor.vapor_kmol", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    vapor_kmol: [f64; 8],
    #[offer(prefix = "reactor.reaction_rates", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    reaction_rates: [f64; 8],

    constants: TepConstants,
}

impl Reactor {
    fn compute(&self) {
        let vapor_group = self.vapor();
        let liquid_group = self.liquid();

        let mut vapor_moles = [0.0f64; 8]; /* kmol A,B,C na fase vapor (estado) */
        let mut liquid_moles = [0.0f64; 8]; /* kmol D,E,F,G,H na fase líquida (estado) */
        for i in 0..3 { vapor_moles[i] = vapor_group[i]; }
        for i in 3..8 { liquid_moles[i] = liquid_group[i - 3]; }
        let total_enthalpy = self.enthalpy();

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 { liquid_composition[i] = liquid_moles[i] / total_liquid_moles; }

        let specific_enthalpy = total_enthalpy / total_liquid_moles;
        let temperature = temperature_from_enthalpy(&liquid_composition, TEMPERATURE_SEED, specific_enthalpy, 0, &self.constants);
        let temperature_k = temperature + 273.15;
        let density = liquid_density(&liquid_composition, temperature, &self.constants);
        let volume_liquid = total_liquid_moles / density;
        let volume_vapor = REACTOR_VOLUME - volume_liquid;

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
        for i in 3..8 { vapor_moles[i] = total_vapor_moles * vapor_composition[i]; }

        /* Cinética de Arrhenius — taxas brutas das 4 reações */
        let mut rates = [0.0f64; 4];
        rates[0] = (31.5859536 - 40000.0 / 1.987 / temperature_k).exp() * REACTION_FACTOR_1_NOMINAL;
        rates[1] = (3.00094014 - 20000.0 / 1.987 / temperature_k).exp() * REACTION_FACTOR_2_NOMINAL;
        rates[2] = (53.4060443 - 60000.0 / 1.987 / temperature_k).exp();
        rates[3] = rates[2] * 0.767488334;
        if partial_pressures[0] > 0.0 && partial_pressures[2] > 0.0 {
            let rf1 = partial_pressures[0].powf(1.1544);
            let rf2 = partial_pressures[2].powf(0.3735);
            rates[0] *= rf1 * rf2 * partial_pressures[3];
            rates[1] *= rf1 * rf2 * partial_pressures[4];
        } else {
            rates[0] = 0.0;
            rates[1] = 0.0;
        }
        rates[2] *= partial_pressures[0] * partial_pressures[4];
        rates[3] *= partial_pressures[0] * partial_pressures[3];
        for r in rates.iter_mut() { *r *= volume_vapor; }

        /* Estequiometria: consumo/produção por componente */
        let mut reaction_rates = [0.0f64; 8];
        reaction_rates[0] = -rates[0] - rates[1] - rates[2];
        reaction_rates[2] = -rates[0] - rates[1];
        reaction_rates[3] = -rates[0] - 1.5 * rates[3];
        reaction_rates[4] = -rates[1] - rates[2];
        reaction_rates[5] = rates[2] + rates[3];
        reaction_rates[6] = rates[0];
        reaction_rates[7] = rates[1];
        let heat_of_reaction = rates[0] * REACTION_ENTHALPIES[0] + rates[1] * REACTION_ENTHALPIES[1];

        self.set_temperature(temperature);
        self.set_temperature_k(temperature_k);
        self.set_pressure(pressure);
        self.set_liquid_volume(volume_liquid);
        self.set_liquid_density(density);
        self.set_vapor_volume(volume_vapor);
        self.set_total_vapor_kmol(total_vapor_moles);
        self.set_heat_of_reaction(heat_of_reaction);
        self.set_liquid_composition(liquid_composition);
        self.set_vapor_composition(vapor_composition);
        self.set_vapor_kmol(vapor_moles);
        self.set_reaction_rates(reaction_rates);

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
            ("state.reactor_vapor.A", 1.0),
            ("state.reactor_vapor.B", 2.0),
            ("state.reactor_vapor.C", 3.0),
            ("state.reactor_vapor.D", 4.0),
            ("state.reactor_vapor.E", 5.0),
            ("state.reactor_vapor.F", 6.0),
            ("state.reactor_vapor.G", 7.0),
            ("state.reactor_vapor.H", 8.0),
            ("state.reactor.energy", 42.0),
        ]);

        let reactor = Reactor::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(reactor.vapor(), [1.0, 2.0, 3.0]);
        assert_eq!(reactor.liquid(), [4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(reactor.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let reactor = Reactor::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(reactor.vapor(), [0.0; 3]);
        assert_eq!(reactor.liquid(), [0.0; 5]);
        assert_eq!(reactor.enthalpy(), 0.0);
    }
}
