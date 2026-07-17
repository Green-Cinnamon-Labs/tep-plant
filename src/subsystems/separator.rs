// tep/separator.rs

/** [REVISADO] | Separator como DynamicModel real. Diferente do Reactor, precisa de um
input de outro componente (reactor.temperature) — por isso `new()` declara
esse `need` em subscribe() além dos `offers`, e guarda o Proxy resolvido
(por StateRegistry::resolve(), chamado depois que todo mundo se inscreveu)
como campo. evaluate() não recebe nada — só lê/escreve via Proxy.
*/
use monjolo::dynamic_model::DynamicModel;
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::{Proxy, StateRegistry};

use crate::physics::constants::TepConstants;
use crate::physics::thermo::{liquid_density, temperature_from_enthalpy};

const SEPARATOR_VOLUME: f64 = 3500.0; // volume total do separador vapor/líquido [m³]
const GAS_CONSTANT: f64 = 998.9; // R em [mmHg·m³/(kmol·K)]

pub struct Separator {
    constants: TepConstants,
    own_state: Vec<Proxy>, // ucvs[0..3] vapor A/B/C, ucls[3..8] líquido D-H, ets[8] entalpia
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

    pub fn new(registry: &mut StateRegistry, initial: &Snapshot) -> Self {
        let mut offer_keys: Vec<String> = Vec::new();

        // Estado próprio (9 números): 3 componentes leves em vapor (A,B,C) +
        // 5 componentes pesados em líquido (D-H) + entalpia total do vaso.
        offer_keys.push("separator.state.vapor_a".into());
        offer_keys.push("separator.state.vapor_b".into());
        offer_keys.push("separator.state.vapor_c".into());
        offer_keys.push("separator.state.liquid_d".into());
        offer_keys.push("separator.state.liquid_e".into());
        offer_keys.push("separator.state.liquid_f".into());
        offer_keys.push("separator.state.liquid_g".into());
        offer_keys.push("separator.state.liquid_h".into());
        offer_keys.push("separator.state.enthalpy".into());

        offer_keys.push("separator.temperature".into());
        offer_keys.push("separator.pressure".into());
        offer_keys.push("separator.liquid_volume".into());
        offer_keys.push("separator.liquid_density".into());
        offer_keys.push("separator.total_vapor_kmol".into());

        // Composição líquida, um valor por componente (A-H).
        offer_keys.push("separator.liquid_composition.a".into());
        offer_keys.push("separator.liquid_composition.b".into());
        offer_keys.push("separator.liquid_composition.c".into());
        offer_keys.push("separator.liquid_composition.d".into());
        offer_keys.push("separator.liquid_composition.e".into());
        offer_keys.push("separator.liquid_composition.f".into());
        offer_keys.push("separator.liquid_composition.g".into());
        offer_keys.push("separator.liquid_composition.h".into());

        // Composição de vapor, um valor por componente (A-H).
        offer_keys.push("separator.vapor_composition.a".into());
        offer_keys.push("separator.vapor_composition.b".into());
        offer_keys.push("separator.vapor_composition.c".into());
        offer_keys.push("separator.vapor_composition.d".into());
        offer_keys.push("separator.vapor_composition.e".into());
        offer_keys.push("separator.vapor_composition.f".into());
        offer_keys.push("separator.vapor_composition.g".into());
        offer_keys.push("separator.vapor_composition.h".into());

        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();
        let (offered, requested) = registry.subscribe(&offer_refs, &["reactor.temperature"]);

        // Semeia o estado próprio com a condição inicial recebida — mesma
        // ordem de offer_keys: vapor A,B,C, líquido D-H, entalpia.
        offered[0].set(initial.get("state.separator_vapor.A").unwrap_or(0.0));
        offered[1].set(initial.get("state.separator_vapor.B").unwrap_or(0.0));
        offered[2].set(initial.get("state.separator_vapor.C").unwrap_or(0.0));
        offered[3].set(initial.get("state.separator_vapor.D").unwrap_or(0.0));
        offered[4].set(initial.get("state.separator_vapor.E").unwrap_or(0.0));
        offered[5].set(initial.get("state.separator_vapor.F").unwrap_or(0.0));
        offered[6].set(initial.get("state.separator_vapor.G").unwrap_or(0.0));
        offered[7].set(initial.get("state.separator_vapor.H").unwrap_or(0.0));
        offered[8].set(initial.get("state.separator.energy").unwrap_or(0.0));

        Self {
            constants: TepConstants::new(),
            own_state: offered[0..9].to_vec(),
            reactor_temperature: requested[0].clone(),
            temperature: offered[9].clone(),
            pressure: offered[10].clone(),
            liquid_volume: offered[11].clone(),
            liquid_density: offered[12].clone(),
            total_vapor_kmol: offered[13].clone(),
            liquid_composition: offered[14..22].to_vec(),
            vapor_composition: offered[22..30].to_vec(),
        }
    }
}

impl DynamicModel for Separator {
    fn name(&self) -> &'static str {
        "Separator"
    }

    fn evaluate(&self) {
        let reactor_temperature = self.reactor_temperature.get();

        let mut vapor_moles = [0.0f64; 8];
        let mut liquid_moles = [0.0f64; 8];
        for i in 0..3 { vapor_moles[i] = self.own_state[i].get(); }
        for i in 3..8 { liquid_moles[i] = self.own_state[i].get(); }
        let total_enthalpy = self.own_state[8].get();

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

        self.temperature.set(temperature);
        self.pressure.set(pressure);
        self.liquid_volume.set(volume_liquid);
        self.liquid_density.set(density);
        self.total_vapor_kmol.set(total_vapor_moles);
        for i in 0..8 {
            self.liquid_composition[i].set(liquid_composition[i]);
            self.vapor_composition[i].set(vapor_composition[i]);
        }

        // [DECISÃO DE MODELAGEM]: a derivada real do próprio estado (yp —
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

        let values: Vec<f64> = separator.own_state.iter().map(Proxy::get).collect();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 42.0]);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let separator = Separator::new(&mut registry.borrow_mut(), &initial);

        let values: Vec<f64> = separator.own_state.iter().map(Proxy::get).collect();
        assert_eq!(values, vec![0.0; 9]);
    }
}
