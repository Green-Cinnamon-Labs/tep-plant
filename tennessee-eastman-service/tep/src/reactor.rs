// tep/reactor.rs
//
// Reactor como DynamicModel real. É o único dos 7 subsistemas químicos
// (Reactor/Separator/Stripper/Compressor/Flows/Heat/Measurements) que não
// depende de dado de outro componente pra calcular sua termodinâmica — por
// isso é o único com evaluate() de verdade por enquanto (ver _deprecated_2.rs
// e a decisão registrada no chat).
//
// Se inscreve no StateRegistry uma única vez, em new() — cada slot que
// oferece vira um Proxy guardado como campo. evaluate() só escreve nesses
// Proxys, nunca constrói nome de novo (ver plan_refactor.md, seções 5.3, 7).
//
// Duas simplificações temporárias, documentadas aqui porque mudam o
// resultado numérico até serem resolvidas com o Disturbance inscrito também:
//   - reaction_factor_1/2 fixos em 1.0 (valor nominal sem distúrbio ativo,
//     IDV 13) — deveriam ser um `need` deste componente.
//   - seed do Newton-Raphson de temperatura fixo em 120.0 — o valor exato não
//     muda a raiz encontrada, só a convergência; o código original só usava
//     esse valor em t=0, mas `time` não está disponível aqui ainda.
//
// As derivadas reais de estado (yp) dependem de FlowsOut (que precisa dos
// quatro subsistemas termodinâmicos ao mesmo tempo) — por isso ainda não são
// oferecidas como slot; só os valores termodinâmicos são reais.

use simulation_framework::dynamic_model::DynamicModel;
use simulation_framework::state_registry::{EvaluationState, Proxy, StateRegistry};

use crate::constants::TepConstants;
use crate::thermo::{liquid_density, temperature_from_enthalpy};

const REACTOR_VOLUME: f64 = 1300.0; // volume total do vaso do reator [m³]
const GAS_CONSTANT: f64 = 998.9; // R em [mmHg·m³/(kmol·K)]
const REACTION_ENTHALPIES: [f64; 2] = [0.06899381054, 0.05]; // calor das reações 1 e 2 [kJ/kmol]

const REACTION_FACTOR_1_NOMINAL: f64 = 1.0; // TODO: devia ser um `need` do Disturbance (IDV 13)
const REACTION_FACTOR_2_NOMINAL: f64 = 1.0; // TODO: devia ser um `need` do Disturbance (IDV 13)
const TEMPERATURE_SEED: f64 = 120.0; // seed do Newton-Raphson — não afeta a raiz

pub struct Reactor {
    constants: TepConstants,
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
    pub fn new(registry: &mut StateRegistry) -> Self {
        let mut offer_keys: Vec<String> = vec![
            "reactor.temperature".into(),
            "reactor.temperature_k".into(),
            "reactor.pressure".into(),
            "reactor.liquid_volume".into(),
            "reactor.liquid_density".into(),
            "reactor.vapor_volume".into(),
            "reactor.total_vapor_kmol".into(),
            "reactor.heat_of_reaction".into(),
        ];
        for i in 0..8 { offer_keys.push(format!("reactor.liquid_composition.{i}")); }
        for i in 0..8 { offer_keys.push(format!("reactor.vapor_composition.{i}")); }
        for i in 0..8 { offer_keys.push(format!("reactor.vapor_kmol.{i}")); }
        for i in 0..8 { offer_keys.push(format!("reactor.reaction_rates.{i}")); }

        let offer_refs: Vec<&str> = offer_keys.iter().map(String::as_str).collect();
        let (offered, _) = registry.subscribe(&offer_refs, &[]);

        Self {
            constants: TepConstants::new(),
            temperature: offered[0].clone(),
            temperature_k: offered[1].clone(),
            pressure: offered[2].clone(),
            liquid_volume: offered[3].clone(),
            liquid_density: offered[4].clone(),
            vapor_volume: offered[5].clone(),
            total_vapor_kmol: offered[6].clone(),
            heat_of_reaction: offered[7].clone(),
            liquid_composition: offered[8..16].to_vec(),
            vapor_composition: offered[16..24].to_vec(),
            vapor_kmol: offered[24..32].to_vec(),
            reaction_rates: offered[32..40].to_vec(),
        }
    }
}

impl DynamicModel for Reactor {
    fn name(&self) -> &'static str {
        "Reactor"
    }

    fn evaluate(&self, state: &[f64], eval: &EvaluationState) {
        let mut vapor_moles = [0.0f64; 8]; // kmol A,B,C na fase vapor (estado)
        let mut liquid_moles = [0.0f64; 8]; // kmol D,E,F,G,H na fase líquida (estado)
        for i in 0..3 { vapor_moles[i] = state[i]; }
        for i in 3..8 { liquid_moles[i] = state[i]; }
        let total_enthalpy = state[8];

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

        // Cinética de Arrhenius — taxas brutas das 4 reações
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

        // Estequiometria: consumo/produção por componente
        let mut reaction_rates = [0.0f64; 8];
        reaction_rates[0] = -rates[0] - rates[1] - rates[2];
        reaction_rates[2] = -rates[0] - rates[1];
        reaction_rates[3] = -rates[0] - 1.5 * rates[3];
        reaction_rates[4] = -rates[1] - rates[2];
        reaction_rates[5] = rates[2] + rates[3];
        reaction_rates[6] = rates[0];
        reaction_rates[7] = rates[1];
        let heat_of_reaction = rates[0] * REACTION_ENTHALPIES[0] + rates[1] * REACTION_ENTHALPIES[1];

        eval.set(&self.temperature, temperature);
        eval.set(&self.temperature_k, temperature_k);
        eval.set(&self.pressure, pressure);
        eval.set(&self.liquid_volume, volume_liquid);
        eval.set(&self.liquid_density, density);
        eval.set(&self.vapor_volume, volume_vapor);
        eval.set(&self.total_vapor_kmol, total_vapor_moles);
        eval.set(&self.heat_of_reaction, heat_of_reaction);
        for i in 0..8 {
            eval.set(&self.liquid_composition[i], liquid_composition[i]);
            eval.set(&self.vapor_composition[i], vapor_composition[i]);
            eval.set(&self.vapor_kmol[i], vapor_moles[i]);
            eval.set(&self.reaction_rates[i], reaction_rates[i]);
        }

        // TODO: derivadas reais precisam de FlowsOut (component_flows[6]/[7],
        // stream_enthalpies[6]/[7]) — Flows precisa dos 4 subsistemas ao mesmo
        // tempo. Ainda não oferecidas como slot.
    }
}
