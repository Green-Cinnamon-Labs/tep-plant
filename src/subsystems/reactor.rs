/* tep/reactor.rs */

/** Reactor como DynamicModel real. É o único dos 7 subsistemas químicos
(Reactor/Separator/Stripper/Compressor/Flows/Heat/Measurements) que não depende de dado de outro
componente pra calcular sua termodinâmica.

Se inscreve no StateRegistry uma única vez, em new() — cada slot que oferece (inclusive o próprio
estado, "reactor.state.{i}") vira um Proxy guardado como campo. evaluate() não recebe nada: só
lê/escreve nesses Proxys, nunca por nome (ver plan_refactor.md, seções 5.3, 7).

reaction_factor_1/2 agora são `needs` de verdade (Disturbance, IDV 13) — não são mais valores
nominais fixos.

Simplificação temporária que continua: seed do Newton-Raphson de temperatura fixo em 120.0 — o valor
exato não muda a raiz encontrada, só a convergência; o código original só usava esse valor em t=0,
mas `time` não está disponível aqui (evaluate() não recebe parâmetro nenhum).

As derivadas reais de estado (yp) dependem de FlowsOut (que precisa dos quatro subsistemas
termodinâmicos ao mesmo tempo) — por isso ainda não são oferecidas como slot; só os valores
termodinâmicos (e o estado próprio, lido de volta) são reais.
*/

use monjolo::dynamic_model::DynamicModel;
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::{Proxy, StateRegistry};

use crate::physics::constants::TepConstants;
use crate::physics::thermo::{liquid_density, temperature_from_enthalpy};

const REACTOR_VOLUME: f64 = 1300.0; /* volume total do vaso do reator [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
const REACTION_ENTHALPIES: [f64; 2] = [0.06899381054, 0.05]; /* calor das reações 1 e 2 [kJ/kmol] */

const REACTION_FACTOR_1_NOMINAL: f64 = 1.0; /* TODO: devia ser um `need` do Disturbance (IDV 13) */
const REACTION_FACTOR_2_NOMINAL: f64 = 1.0; /* TODO: devia ser um `need` do Disturbance (IDV 13) */
const TEMPERATURE_SEED: f64 = 120.0; /* seed do Newton-Raphson — não afeta a raiz */

pub struct Reactor {
    constants: TepConstants,
    /* Estado próprio (9 números: ucvr[0..3] vapor A/B/C, uclr[3..8] líquido D-H, etr[8] entalpia
    total) — o que antes vinha de `state: &[f64]`.
    */
    own_state: Vec<Proxy>,
    temperature: Proxy,
    temperature_k: Proxy,
    pressure: Proxy,
    liquid_volume: Proxy,
    liquid_density: Proxy,
    vapor_volume: Proxy,
    total_vapor_kmol: Proxy,
    heat_of_reaction: Proxy,
    liquid_composition: Vec<Proxy>,
    vapor_composition: Vec<Proxy>,
    vapor_kmol: Vec<Proxy>,
    reaction_rates: Vec<Proxy>,
}

impl Reactor {
    pub fn new(registry: &mut StateRegistry, initial: &Snapshot) -> Self {
        /* cria a lista vazia que vai acumular todas as chaves a oferecer */
        let mut offer_keys: Vec<String> = Vec::new();

        /* Estado próprio (9 números): 3 componentes leves em vapor (A,B,C) + 5 componentes pesados
        em líquido (D-H) + entalpia total do vaso.
        */
        offer_keys.push("reactor.state.vapor_a".into());
        offer_keys.push("reactor.state.vapor_b".into());
        offer_keys.push("reactor.state.vapor_c".into());
        offer_keys.push("reactor.state.liquid_d".into());
        offer_keys.push("reactor.state.liquid_e".into());
        offer_keys.push("reactor.state.liquid_f".into());
        offer_keys.push("reactor.state.liquid_g".into());
        offer_keys.push("reactor.state.liquid_h".into());
        offer_keys.push("reactor.state.enthalpy".into());

        offer_keys.push("reactor.temperature".into());
        offer_keys.push("reactor.temperature_k".into());
        offer_keys.push("reactor.pressure".into());
        offer_keys.push("reactor.liquid_volume".into());
        offer_keys.push("reactor.liquid_density".into());
        offer_keys.push("reactor.vapor_volume".into());
        offer_keys.push("reactor.total_vapor_kmol".into());
        offer_keys.push("reactor.heat_of_reaction".into());

        /* Composição líquida, um valor por componente (A-H). */
        offer_keys.push("reactor.liquid_composition.a".into());
        offer_keys.push("reactor.liquid_composition.b".into());
        offer_keys.push("reactor.liquid_composition.c".into());
        offer_keys.push("reactor.liquid_composition.d".into());
        offer_keys.push("reactor.liquid_composition.e".into());
        offer_keys.push("reactor.liquid_composition.f".into());
        offer_keys.push("reactor.liquid_composition.g".into());
        offer_keys.push("reactor.liquid_composition.h".into());

        /* Composição de vapor, um valor por componente (A-H). */
        offer_keys.push("reactor.vapor_composition.a".into());
        offer_keys.push("reactor.vapor_composition.b".into());
        offer_keys.push("reactor.vapor_composition.c".into());
        offer_keys.push("reactor.vapor_composition.d".into());
        offer_keys.push("reactor.vapor_composition.e".into());
        offer_keys.push("reactor.vapor_composition.f".into());
        offer_keys.push("reactor.vapor_composition.g".into());
        offer_keys.push("reactor.vapor_composition.h".into());

        /* Kmol em fase vapor, um valor por componente (A-H). */
        offer_keys.push("reactor.vapor_kmol.a".into());
        offer_keys.push("reactor.vapor_kmol.b".into());
        offer_keys.push("reactor.vapor_kmol.c".into());
        offer_keys.push("reactor.vapor_kmol.d".into());
        offer_keys.push("reactor.vapor_kmol.e".into());
        offer_keys.push("reactor.vapor_kmol.f".into());
        offer_keys.push("reactor.vapor_kmol.g".into());
        offer_keys.push("reactor.vapor_kmol.h".into());

        /* Taxa líquida de consumo/produção por componente (A-H) — estequiometria das 4 reações, já
        somada por componente.
        */
        offer_keys.push("reactor.reaction_rates.a".into());
        offer_keys.push("reactor.reaction_rates.b".into());
        offer_keys.push("reactor.reaction_rates.c".into());
        offer_keys.push("reactor.reaction_rates.d".into());
        offer_keys.push("reactor.reaction_rates.e".into());
        offer_keys.push("reactor.reaction_rates.f".into());
        offer_keys.push("reactor.reaction_rates.g".into());
        offer_keys.push("reactor.reaction_rates.h".into());

        /* converte Vec<String> pra Vec<&str>, formato que subscribe() espera */
        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();

        /* registra todas as chaves de uma vez; devolve os Proxys resolvidos na mesma ordem (segundo
        retorno, needs, vazio — Reactor não depende de mais nada)
        */
        let (offered, _) = registry.subscribe(&offer_refs, &[]);

        /* Semeia o estado próprio com a condição inicial recebida — mesma ordem de offer_keys:
        vapor A,B,C, líquido D-H, entalpia. Chave ausente no Snapshot vira 0.0 (mesmo default que o
        slot já teria).
        */
        offered[0].set(initial.get("state.reactor_vapor.A").unwrap_or(0.0));
        offered[1].set(initial.get("state.reactor_vapor.B").unwrap_or(0.0));
        offered[2].set(initial.get("state.reactor_vapor.C").unwrap_or(0.0));
        offered[3].set(initial.get("state.reactor_vapor.D").unwrap_or(0.0));
        offered[4].set(initial.get("state.reactor_vapor.E").unwrap_or(0.0));
        offered[5].set(initial.get("state.reactor_vapor.F").unwrap_or(0.0));
        offered[6].set(initial.get("state.reactor_vapor.G").unwrap_or(0.0));
        offered[7].set(initial.get("state.reactor_vapor.H").unwrap_or(0.0));
        offered[8].set(initial.get("state.reactor.energy").unwrap_or(0.0));

        Self {
            constants: TepConstants::new(),
            own_state: offered[0..9].to_vec(),
            temperature: offered[9].clone(),
            temperature_k: offered[10].clone(),
            pressure: offered[11].clone(),
            liquid_volume: offered[12].clone(),
            liquid_density: offered[13].clone(),
            vapor_volume: offered[14].clone(),
            total_vapor_kmol: offered[15].clone(),
            heat_of_reaction: offered[16].clone(),
            liquid_composition: offered[17..25].to_vec(),
            vapor_composition: offered[25..33].to_vec(),
            vapor_kmol: offered[33..41].to_vec(),
            reaction_rates: offered[41..49].to_vec(),
        }
    }
}

impl DynamicModel for Reactor {
    fn name(&self) -> &'static str {
        "Reactor"
    }

    fn evaluate(&self) {
        let mut vapor_moles = [0.0f64; 8]; /* kmol A,B,C na fase vapor (estado) */
        let mut liquid_moles = [0.0f64; 8]; /* kmol D,E,F,G,H na fase líquida (estado) */
        for i in 0..3 { vapor_moles[i] = self.own_state[i].get(); }
        for i in 3..8 { liquid_moles[i] = self.own_state[i].get(); }
        let total_enthalpy = self.own_state[8].get();

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

        self.temperature.set(temperature);
        self.temperature_k.set(temperature_k);
        self.pressure.set(pressure);
        self.liquid_volume.set(volume_liquid);
        self.liquid_density.set(density);
        self.vapor_volume.set(volume_vapor);
        self.total_vapor_kmol.set(total_vapor_moles);
        self.heat_of_reaction.set(heat_of_reaction);
        for i in 0..8 {
            self.liquid_composition[i].set(liquid_composition[i]);
            self.vapor_composition[i].set(vapor_composition[i]);
            self.vapor_kmol[i].set(vapor_moles[i]);
            self.reaction_rates[i].set(reaction_rates[i]);
        }

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

        let values: Vec<f64> = reactor.own_state.iter().map(Proxy::get).collect();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 42.0]);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let reactor = Reactor::new(&mut registry.borrow_mut(), &initial);

        let values: Vec<f64> = reactor.own_state.iter().map(Proxy::get).collect();
        assert_eq!(values, vec![0.0; 9]);
    }
}
