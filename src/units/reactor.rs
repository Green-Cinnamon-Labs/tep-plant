/* tep/units/reactor.rs */

use crate::physics::constants::TepConstants;
use monjolo::chemistry::{liquid_density, mixture_enthalpy, temperature_from_enthalpy};

const REACTOR_VOLUME: f64 = 1300.0; /* volume total do vaso do reator [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
const REACTION_ENTHALPIES: [f64; 2] = [0.06899381054, 0.05]; /* calor das reações 1 e 2 [kJ/kmol] */
const REACTION_FACTOR_1_NOMINAL: f64 = 1.0; /* TODO: devia ser um `need` do Disturbance (IDV 13) */
const REACTION_FACTOR_2_NOMINAL: f64 = 1.0; /* TODO: devia ser um `need` do Disturbance (IDV 13) */
const TEMPERATURE_SEED: f64 = 120.0; /* seed do Newton-Raphson — não afeta a raiz */

/* Temperatura de ENTRADA da água de resfriamento do reator (TCWR) — nominal, mesmo `s_zero` de
TepDisturbanceState, canal 4. IDV(4) perturbaria isso; ainda não conectado (issue #62), fica fixo.
*/
const REACTOR_COOLING_WATER_INLET: f64 = 35.0;
const REACTOR_COOLING_WATER_RANGE: f64 = 1000.0; /* VRNG (TEINIT) da válvula de água de resfriamento */
/* cpcw_eff (TEINIT) — calibrado no FORTRAN original pra dar twr≈94.6°C no ponto nominal (fcwr≈41.1,
tcr=120, uar≈0.856). */
const REACTOR_COOLING_WATER_CAPACITY: f64 = 0.00942;

/** Quinta e última unidade migrada pro scheduler de dataflow topológico (issue 10) — fecha a
migração. Absorve de `flows.rs`: Block 22 (AGSP/agitation_factor — fundido direto em `heat`, sem
publicar chave própria, mesmo tratamento de `condenser_ua` em `units::stripper`) e Block 23
(slot 7, reator→separador). De `heat.rs`: Block 32 (troca térmica do reator). De `derivatives.rs`:
a seção "Reator" do balanço de massa/energia (Block 40, YP(1..9)) — a última EDO que não calculava
a própria derivada. `flows.rs`/`heat.rs`/`derivatives.rs` são deletados junto com este commit —
nada mais resta neles.
*/
#[monjolo::dynamic_model(tasks)]
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

    constants: TepConstants,
}

#[monjolo::tasks]
impl Reactor {
    
    /* Bloco 1: balanço de energia próprio → temperatura/pressão/composição/cinética — igual ao
    `compute()` monolítico de antes. Sem `#[need]` nenhum: só lê o próprio `#[state]`.
    */
    #[offer(key = "reactor.temperature")]
    #[offer(key = "reactor.temperature_k")]
    #[offer(key = "reactor.pressure")]
    #[offer(key = "reactor.liquid_volume")]
    #[offer(key = "reactor.liquid_density")]
    #[offer(key = "reactor.vapor_volume")]
    #[offer(key = "reactor.total_vapor_kmol")]
    #[offer(key = "reactor.heat_of_reaction")]
    #[offer(prefix = "reactor.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[offer(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[offer(prefix = "reactor.vapor_kmol", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[offer(prefix = "reactor.reaction_rates", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[allow(clippy::type_complexity)]
    fn thermodynamics(&self) -> (f64, f64, f64, f64, f64, f64, f64, f64, [f64; 8], [f64; 8], [f64; 8], [f64; 8]) {
        let vapor_group = self.vapor();
        let liquid_group = self.liquid();

        let mut vapor_moles = [0.0f64; 8]; /* kmol A,B,C na fase vapor (estado) */
        let mut liquid_moles = [0.0f64; 8]; /* kmol D,E,F,G,H na fase líquida (estado) */
        for i in 0..3 {
            vapor_moles[i] = vapor_group[i];
        }
        for i in 3..8 {
            liquid_moles[i] = liquid_group[i - 3];
        }
        let total_enthalpy = self.enthalpy();

        let total_liquid_moles: f64 = liquid_moles.iter().sum();
        let mut liquid_composition = [0.0f64; 8];
        for i in 0..8 {
            liquid_composition[i] = liquid_moles[i] / total_liquid_moles;
        }

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
        for i in 0..8 {
            vapor_composition[i] = partial_pressures[i] / pressure;
        }
        let total_vapor_moles = pressure * volume_vapor / GAS_CONSTANT / temperature_k;
        for i in 3..8 {
            vapor_moles[i] = total_vapor_moles * vapor_composition[i];
        }

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
        for r in rates.iter_mut() {
            *r *= volume_vapor;
        }

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

        (
            temperature,
            temperature_k,
            pressure,
            volume_liquid,
            density,
            volume_vapor,
            total_vapor_moles,
            heat_of_reaction,
            liquid_composition,
            vapor_composition,
            vapor_moles,
            reaction_rates,
        )
    }

    /* Bloco 2 (ex-Flows, Block 23 slot 7): vazão pro separador, dependente de ΔP (sem válvula). */
    #[need(key = "reactor.pressure")]
    #[need(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "separator.pressure")]
    #[offer(key = "flows.stream_flow.7")]
    fn outlet_flow(&self, own_pressure: f64, own_vapor: [f64; 8], separator_pressure: f64) -> f64 {
        let mol_weight: f64 = (0..8).map(|i| own_vapor[i] * self.constants.xmw[i]).sum();
        4574.21 * (own_pressure - separator_pressure).max(0.0).sqrt() * (1.0 - 0.25 * 0.0) / mol_weight /* disturbance channel 11, neutro */
    }

    /* Bloco 3 (ex-Heat, Block 32 + o AGSP de Block 22, que nunca teve dono além de ser consumido
    aqui mesmo — mesmo tratamento do `condenser_ua` em `units::stripper`): troca térmica entre o
    reator e sua água de resfriamento — UARLEV degrau/rampa/platô conforme o nível de líquido
    (fração da serpentina submersa, ver explicação abaixo) pondera tanto a temperatura de RETORNO
    da água quanto o calor que sai do reator; as duas contas usam o mesmo `uar`, por isso vivem
    juntas num método só, não separadas em dois `#[need]`/`#[offer]` cruzando o `StateRegistry`
    pra compartilhar um número que não é grandeza própria de ninguém de fora.

    DECISÃO DE MODELAGEM: a vazão de água é função EXCLUSIVA da abertura da válvula
    (`valve.reactor_cooling_water.position`, XMV 10) — este modelo não representa rede de tubulação
    nem queda de pressão na linha de resfriamento, mesma simplificação já usada em toda vazão
    process-side (`Reactor::outlet_flow`, `Compressor::outlet_flows`: abertura de válvula ⇒ vazão,
    ponto, sem física de tubulação intermediária). A partir dessa vazão, a temperatura de RETORNO
    da água (`twr`) é a solução de um balanço de calor quase-estático entre a água (capacidade
    térmica `fcwr * REACTOR_COOLING_WATER_CAPACITY`, entrando a `REACTOR_COOLING_WATER_INLET`) e o
    reator (coeficiente de troca `uar`, a `reactor_temperature`) — média ponderada pelas duas
    capacidades. Tradução direta do Block 32 do teprob.f original.

    NOTA (2026-09-14): antes desta correção, este método usava uma constante fixa
    (`REACTOR_COOLING_WATER_RETURN = 35.0`) no lugar de `twr` calculado — 35.0 é, na verdade,
    `REACTOR_COOLING_WATER_INLET` (a água ENTRANDO, não saindo), então o reator resfriava ~3.3x
    mais forte que o correto no nominal (força-motriz de 85 em vez de 25.4) e a válvula
    (`valve.reactor_cooling_water.position`) ficava desconectada da física por completo — escrever
    nela não tinha efeito nenhum. Consequência observada ao vivo: temperatura do reator estabilizava
    perto de 35-38°C em vez de ~120°C; com a reação muito mais lenta nessa faixa, gás não-reagido
    se acumulava até estourar o limite de pressão do ISD (3000 kPa).
    */
    #[need(key = "reactor.liquid_volume")]
    #[need(key = "reactor.temperature")]
    #[need(key = "agitator.speed")]
    #[need(key = "valve.reactor_cooling_water.position")]
    #[offer(key = "heat.reactor_heat")]
    #[offer(key = "heat.reactor_cooling_water_return")]
    fn heat(
        &self,
        reactor_liquid_volume: f64,
        reactor_temperature: f64,
        agitator_speed: f64,
        cooling_water_position: f64,
    ) -> (f64, f64) {
        /* UARLEV: fração da serpentina submersa, 0 abaixo de level=10 (seca, sem troca), rampa
        linear até level=50, platô em 1.0 dali pra cima (totalmente submersa — mais líquido não
        aumenta mais nada).
        */
        let agitation_factor = (agitator_speed + 150.0) / 100.0;
        let level = reactor_liquid_volume / 7.8; /* 7.8 = fator de conversão de volume pra "nível" deste bloco */
        let uar_level = if level > 50.0 {
            1.0
        } else if level < 10.0 {
            0.0
        } else {
            0.025 * level - 0.25
        };
        let uar = uar_level * (-0.5 * agitation_factor * agitation_factor + 2.75 * agitation_factor - 2.5) * 855490e-6;

        /* fcwr: vazão da água, só função da válvula (decisão de modelagem acima) — escala
        `posição * VRNG * 0.001`, não `posição * VRNG / 100` como as outras válvulas: é assim que o
        Block 32 original define fcwr, calibrado junto com REACTOR_COOLING_WATER_CAPACITY.
        */
        let fcwr = cooling_water_position * REACTOR_COOLING_WATER_RANGE * 0.001;
        let cw_capacity = fcwr * REACTOR_COOLING_WATER_CAPACITY;
        let total_capacity = cw_capacity + uar;

        /* Guarda contra 0/0: só degenera quando fcwr E uar são ambos ~0 (válvula fechada com
        nível fora da faixa de agitação eficaz) — nesse caso `uar=0` já zera reactor_heat de
        qualquer forma, então cair em reactor_temperature (força-motriz zero) é seguro, só evita
        NaN se propagando pro resto da simulação (uar * NaN não seria zero).
        */
        let twr = if total_capacity > 1e-12 {
            (cw_capacity * REACTOR_COOLING_WATER_INLET + uar * reactor_temperature) / total_capacity
        } else {
            reactor_temperature
        };

        let reactor_heat = uar * (twr - reactor_temperature) * (1.0 - 0.35 * 0.0); /* disturbance channel 9 (IDV 10), neutro */

        (reactor_heat, twr)
    }

    /* Bloco 4 (ex-Derivatives, Block 40 YP(1..9)): balanço de massa/energia do próprio estado —
    a última EDO que faltava. Entalpias recomputadas frescas, mesmo padrão das outras 3 unidades.
    */
    #[need(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    #[need(key = "compressor.temperature")]
    #[need(key = "flows.stream_flow.6")]
    #[need(key = "flows.stream_flow.7")]
    #[need(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "reactor.temperature")]
    #[need(prefix = "reactor.reaction_rates", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[need(key = "reactor.heat_of_reaction")]
    #[need(key = "heat.reactor_heat")]
    #[offer(prefix = "reactor.state", components = ["vapor_a.derivative", "vapor_b.derivative", "vapor_c.derivative"])]
    #[offer(prefix = "reactor.state", components = ["liquid_d.derivative", "liquid_e.derivative", "liquid_f.derivative", "liquid_g.derivative", "liquid_h.derivative"])]
    #[offer(key = "reactor.state.enthalpy.derivative")]
    #[allow(clippy::too_many_arguments)]
    fn yp_derivative(
        &self,
        compressor_vapor: [f64; 8],
        compressor_temperature: f64,
        compressor_recycle_flow: f64,
        outlet_flow: f64,
        reactor_vapor: [f64; 8],
        reactor_temperature: f64,
        reaction_rates: [f64; 8],
        heat_of_reaction: f64,
        reactor_heat: f64,
    ) -> ([f64; 3], [f64; 5], f64) {
        let enthalpy_compressor_recycle = mixture_enthalpy(&compressor_vapor, compressor_temperature, 1, &self.constants);
        let enthalpy_reactor_outlet = mixture_enthalpy(&reactor_vapor, reactor_temperature, 1, &self.constants);

        let mut vapor_derivative = [0.0f64; 3];
        let mut liquid_derivative = [0.0f64; 5];
        for i in 0..8 {
            let value = compressor_vapor[i] * compressor_recycle_flow - reactor_vapor[i] * outlet_flow + reaction_rates[i];
            if i < 3 {
                vapor_derivative[i] = value;
            } else {
                liquid_derivative[i - 3] = value;
            }
        }
        let enthalpy_derivative =
            enthalpy_compressor_recycle * compressor_recycle_flow - enthalpy_reactor_outlet * outlet_flow + heat_of_reaction + reactor_heat;

        (vapor_derivative, liquid_derivative, enthalpy_derivative)
    }

    /* Bloco 5 (ex-measured.rs, Block 35): XMEAS 7-9 (pressão/nível/temperatura do reator) + XMEAS
    21 (temperatura de saída da água de resfriamento) — conversões preservadas exatamente do
    original: (P-760)/760*101.325 (mmHg gauge → kPa gauge), volume/666.7*100 (calibração do
    instrumento de nível).
    */
    #[need(key = "reactor.pressure")]
    #[need(key = "reactor.liquid_volume")]
    #[need(key = "reactor.temperature")]
    #[need(key = "heat.reactor_cooling_water_return")]
    #[offer(key = "xmeas.reactor.pressure")]
    #[offer(key = "xmeas.reactor.level")]
    #[offer(key = "xmeas.reactor.temperature")]
    #[offer(key = "xmeas.reactor.cooling_water_outlet_temperature")]
    fn xmeas_conversions(&self, pressure: f64, liquid_volume: f64, temperature: f64, cooling_water_return: f64) -> (f64, f64, f64, f64) {
        let xmeas_pressure = (pressure - 760.0) / 760.0 * 101.325;
        let xmeas_level = (liquid_volume - 84.6) / 666.7 * 100.0;

        (xmeas_pressure, xmeas_level, temperature, cooling_water_return)
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
