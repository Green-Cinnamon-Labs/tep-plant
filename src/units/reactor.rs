/* tep/units/reactor.rs */

use crate::physics::constants::TepConstants;
use monjolo::chemistry::{liquid_density, mixture_enthalpy, temperature_from_enthalpy};

const REACTOR_VOLUME: f64 = 1300.0; /* volume total do vaso do reator [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
const REACTION_ENTHALPIES: [f64; 2] = [0.06899381054, 0.05]; /* calor das reações 1 e 2 [kJ/kmol] */
const REACTION_FACTOR_1_NOMINAL: f64 = 1.0; /* TODO: devia ser um `need` do Disturbance (IDV 13) */
const REACTION_FACTOR_2_NOMINAL: f64 = 1.0; /* TODO: devia ser um `need` do Disturbance (IDV 13) */
const TEMPERATURE_SEED: f64 = 120.0; /* seed do Newton-Raphson — não afeta a raiz */

/* Temperatura de SAÍDA da água de resfriamento do reator (TWR) — congelada, fiel ao FORTRAN
original (TEFUNC: YP(37) nunca é atribuído, TWR fica constante no valor de inicialização) e ao
próprio `[state.cooling].reactor_water_temp` de `application.toml`. Mesmo padrão já usado em
`Separator::heat_exchange` (`SEPARATOR_COOLING_WATER_RETURN`), nunca mexido nesta novela.

NOTA (2026-09-16, Exp 24): existiu aqui uma fórmula quase-estática (`twr` como média ponderada de
`uar`/`tcr` e a água de entrada) que foi REMOVIDA — não é regressão, é correção de um erro estrutural
descoberto ao investigar por que a `v1.0.0` (usada como "ground truth" nos Exp 18-23) tinha essa
fórmula ativa. Arqueologia via `git log --follow` em
`tennessee-eastman-service/core/src/dynamics/tep/model.rs`:

  - 2026-03-09 (`b4c2077`): existia uma EDO própria pra `twr`/`tws` (não a fórmula quase-estática,
    uma dinâmica de 1ª ordem ainda mais forte) — revertida por "causar colapso da temperatura de
    resfriamento pra ~35°C, destabilizando o balanço de energia do reator". Fiel ao FORTRAN
    (`YP(37)` nunca atribuído) = TWR congelado, sem exceção.
  - 2026-05-27 (`123a6a3`): a fórmula quase-estática foi introduzida DE NOVO, especificamente pra
    dar ao IDV(4) algum efeito observável (com `twr` congelado, IDV(4) literalmente não faz nada —
    o que estava emperrando o Exp 14 daquela época).
  - 2026-05-28: a tag `v1.0.0` foi cortada — um dia depois, capturando a fórmula quase-estática
    ainda ativa e não validada.
  - 2026-06-01 (`ad08ea0`, "EXP14 Done"): a mesma fórmula foi comentada de volta pra `twr` congelado
    — o autor documentou o motivo: "quando tcr cai abaixo de ~67°C o balanço produz twr < tcr →
    QUR < 0 (trocador 'aquece' o reator), comportamento ausente no FORTRAN original".

Ou seja: a MESMA classe de mecanismo foi tentada e revertida duas vezes por instabilidade — e a
`v1.0.0` que este repositório vinha usando como referência validada é, por coincidência de datas, o
único ponto da história onde ela ficou ativa. Confirmado empiricamente contra
`docs/simulations/simulation_log_13.csv`: ao longo de 20h sem distúrbio, `reactor.temperature`
(XMEAS(9)) varia ~0.49°C, mas `XMEAS(21)` (twr) varia só ~0.076°C — bem menos do que a fórmula
quase-estática preveria (peso ~0.69 de `tcr` implicaria ~0.34°C de variação em `twr`) e plenamente
consistente com `twr` congelado + ruído de medição (σ=0.01). O próprio baseline validado nunca usou
essa fórmula.
*/
const REACTOR_COOLING_WATER_RETURN: f64 = 94.59927549;

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
    fn physical_state(&self) -> (f64, f64, f64, f64, f64, f64, f64, f64, [f64; 8], [f64; 8], [f64; 8], [f64; 8]) {
        
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
    fn flow_to_separator(&self, own_pressure: f64, own_vapor: [f64; 8], separator_pressure: f64) -> f64 {
        let mol_weight: f64 = (0..8).map(|i| own_vapor[i] * self.constants.xmw[i]).sum();
        4574.21 * (own_pressure - separator_pressure).max(0.0).sqrt() * (1.0 - 0.25 * 0.0) / mol_weight /* disturbance channel 11, neutro */
    }

    /* Bloco 3 (ex-Heat, Block 32 + o AGSP de Block 22, que nunca teve dono além de ser consumido
    aqui mesmo — mesmo tratamento do `condenser_ua` em `units::stripper`): troca térmica entre o
    reator e sua água de resfriamento — UARLEV degrau/rampa/platô conforme o nível de líquido
    (fração da serpentina submersa, ver explicação abaixo) pondera o calor que sai do reator.

    NOTA (2026-09-16, Exp 24): a fórmula quase-estática que calculava `twr` (temperatura de RETORNO
    da água) a partir da abertura da válvula (`valve.reactor_cooling_water.position`, XMV 10) foi
    REMOVIDA — ver `REACTOR_COOLING_WATER_RETURN` acima pra arqueologia completa. Resumo: essa
    fórmula já foi tentada e revertida duas vezes na história deste projeto (2026-03, 2026-06) por
    causar exatamente este tipo de colapso térmico; a `v1.0.0` usada como referência nos Exp 18-23
    só a tinha ativa por coincidência de datas (tag cortada um dia depois dela ter sido reintroduzida
    e três dias antes de ser revertida de novo); e o próprio baseline validado
    (`docs/simulations/simulation_log_13.csv`) mostra `twr` essencialmente congelado, não
    respondendo a `tcr` com a sensibilidade que a fórmula preveria. `twr` volta a ser a constante
    congelada `REACTOR_COOLING_WATER_RETURN`, fiel ao FORTRAN original — `valve.reactor_cooling_
    water.position` deixa de ser um `#[need]` daqui (não afeta esta física, mesma conclusão a que o
    projeto já tinha chegado em 2026-03/2026-06 e que só não tinha sido reaplicada aqui).
    */
    #[need(key = "reactor.liquid_volume")]
    #[need(key = "reactor.temperature")]
    #[need(key = "agitator.speed")]
    #[offer(key = "heat.reactor_heat")]
    #[offer(key = "heat.reactor_cooling_water_return")]
    fn heat_exchange(&self, reactor_liquid_volume: f64, reactor_temperature: f64, agitator_speed: f64) -> (f64, f64) {
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

        /* Fórmula quase-estática removida (Exp 24) — preservada aqui só como referência, nunca
        compilada:
        //
        // let fcwr = cooling_water_position * REACTOR_COOLING_WATER_RANGE * 0.001;
        // let cw_capacity = fcwr * REACTOR_COOLING_WATER_CAPACITY; // REACTOR_COOLING_WATER_CAPACITY = 0.00942
        // let total_capacity = cw_capacity + uar;
        // let twr = if total_capacity > 1e-12 {
        //     (cw_capacity * REACTOR_COOLING_WATER_INLET + uar * reactor_temperature) / total_capacity // REACTOR_COOLING_WATER_INLET = 38.5
        // } else {
        //     reactor_temperature
        // };
        */
        let twr = REACTOR_COOLING_WATER_RETURN;

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
    fn mass_and_energy_balance(
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
    fn xmeas_readings(&self, pressure: f64, liquid_volume: f64, temperature: f64, cooling_water_return: f64) -> (f64, f64, f64, f64) {
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

    /* Investigação do Experimento 18 (spec-tennessee-eastman/experimentos.md): a planta colapsa
    (reactor.temperature cai de ~120°C pra ~40-60°C, reactor.pressure estoura o ISD) mesmo depois
    das duas correções de água de resfriamento. Este teste isola `physical_state()` do resto da
    simulação — semeia exatamente os valores de `application.toml` (o mesmo estado inicial validado
    nos Exp 3/10/11/13) e verifica se o cálculo de flash+cinética, sozinho, já reproduz o ponto
    nominal documentado. Se isto falhar, o bug está aqui, não em várias horas de integração RK4
    acumulando erro.
    */
    #[test]
    fn physical_state_matches_nominal_operating_point_from_application_toml() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[
            ("state.reactor_vapor.A", 10.679592788064898),
            ("state.reactor_vapor.B", 4.666690921054032),
            ("state.reactor_vapor.C", 7.668821577187304),
            ("state.reactor_vapor.D", 0.3968744850195289),
            ("state.reactor_vapor.E", 22.74175383132296),
            ("state.reactor_vapor.F", 2.9441162740614635),
            ("state.reactor_vapor.G", 148.4668245572372),
            ("state.reactor_vapor.H", 153.02693445529974),
            ("state.reactor.energy", 2.6983891373872186),
        ]);

        let reactor = Reactor::new(&mut registry.borrow_mut(), &initial);
        let (temperature, _temperature_k, pressure, _volume_liquid, _density, _volume_vapor, _total_vapor_moles, heat_of_reaction, _liquid_composition, _vapor_composition, _vapor_moles, reaction_rates) =
            reactor.__physical_state_impl();

        /* Nominal documentado (Exp 3/10/11/13, te_exp3_snapshot.toml): ~120°C, XMEAS(7) ~2695-2705
        kPa. Faixas com folga generosa — o objetivo é pegar um colapso grosseiro (dezenas de graus/
        milhares de kPa fora), não validar casas decimais.
        */
        assert!(
            (110.0..130.0).contains(&temperature),
            "esperava temperatura perto do nominal ~120°C, obteve {temperature}°C — flash/cinética \
            já diverge na primeira chamada, sem nenhuma integração RK4 envolvida"
        );

        let xmeas_pressure = (pressure - 760.0) / 760.0 * 101.325;
        assert!(
            (2500.0..2900.0).contains(&xmeas_pressure),
            "esperava XMEAS(7) perto do nominal ~2700 kPa, obteve {xmeas_pressure} kPa (pressão \
            bruta = {pressure} mmHg)"
        );

        /* Exotérmica: heat_of_reaction > 0 confirma que a reação está de fato avançando (rr[0]/
        rr[1] > 0) — se as taxas tivessem colapsado pra perto de zero, isso apareceria aqui como um
        heat_of_reaction quase nulo, incapaz de compensar qualquer resfriamento.
        */
        assert!(
            heat_of_reaction > 0.0,
            "esperava heat_of_reaction > 0 (reação exotérmica avançando), obteve {heat_of_reaction}"
        );

        /* reaction_rates[0] é o consumo líquido de A (sempre ≤ 0 pela estequiometria) — perto de
        zero indicaria reação estagnada.
        */
        assert!(
            reaction_rates[0] < -1.0,
            "esperava consumo líquido de A claramente negativo, obteve {}",
            reaction_rates[0]
        );
    }

    /* `mass_and_energy_balance` consome os outputs de `physical_state`/`heat_exchange`/fluxos de
    outras unidades — testado aqui com entradas balanceadas (mesma composição/temperatura/vazão
    entrando e saindo) pra confirmar que os termos de fluxo se cancelam exatamente, sobrando só
    reação/calor. Não depende de nenhum valor "nominal" hipotético — é uma verificação estrutural
    da equação de balanço, útil mesmo sem saber o ponto de operação de cor.
    */
    #[test]
    fn mass_and_energy_balance_cancels_flow_terms_when_inflow_equals_outflow() {
        let registry = StateRegistry::shared();
        let reactor = Reactor::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

        let composition = [0.1, 0.05, 0.1, 0.05, 0.2, 0.1, 0.2, 0.2];
        let temperature = 120.0;
        let flow = 500.0;

        let (vapor_derivative, liquid_derivative, enthalpy_derivative) = reactor.__mass_and_energy_balance_impl(
            composition,
            temperature,
            flow,
            flow,
            composition,
            temperature,
            [0.0; 8],
            0.0,
            0.0,
        );

        assert_eq!(vapor_derivative, [0.0; 3], "mesma composição/vazão entrando e saindo não deveria acumular nada");
        assert_eq!(liquid_derivative, [0.0; 5], "mesma composição/vazão entrando e saindo não deveria acumular nada");
        assert_eq!(enthalpy_derivative, 0.0, "mesma composição/temperatura/vazão não deveria gerar entalpia líquida");
    }

    #[test]
    fn mass_and_energy_balance_passes_through_reaction_and_heat_when_flows_are_balanced() {
        let registry = StateRegistry::shared();
        let reactor = Reactor::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

        let composition = [0.1, 0.05, 0.1, 0.05, 0.2, 0.1, 0.2, 0.2];
        let temperature = 120.0;
        let flow = 500.0;
        let reaction_rates = [-3.0, 0.0, -2.0, -1.5, -1.0, 1.5, 2.0, 1.0];
        let heat_of_reaction = 10.0;
        let reactor_heat = -4.0;

        let (vapor_derivative, liquid_derivative, enthalpy_derivative) = reactor.__mass_and_energy_balance_impl(
            composition,
            temperature,
            flow,
            flow,
            composition,
            temperature,
            reaction_rates,
            heat_of_reaction,
            reactor_heat,
        );

        /* Com os fluxos cancelados, a derivada de cada componente É a taxa de reação — sem
        surpresa de índice trocado entre vapor_derivative (0..3) e liquid_derivative (3..8).
        */
        assert_eq!(vapor_derivative, [reaction_rates[0], reaction_rates[1], reaction_rates[2]]);
        assert_eq!(liquid_derivative, [reaction_rates[3], reaction_rates[4], reaction_rates[5], reaction_rates[6], reaction_rates[7]]);
        assert_eq!(enthalpy_derivative, heat_of_reaction + reactor_heat);
    }

    /** Experimento 23/24 (spec-tennessee-eastman/experimentos.md). O Exp 19 provou que
    `physical_state()` reproduz o nominal — mas um ÚNICO ponto não pega um bug que só aparece fora
    dele. Este teste é bem mais rigoroso: para cada um dos primeiros 200 ticks de
    `docs/simulations/simulation_log_13.csv` (a trajetória REAL, validada, do Exp 13), alimenta
    `physical_state()`/`heat_exchange()` com o estado EXATO (`YY[0..8]`, `XMV(12)`) que a planta
    validada tinha naquele instante — e compara a saída contra o `XMEAS(7)`/`XMEAS(9)`/`XMEAS(21)`
    que a MESMA planta validada mediu no MESMO instante. Se a fórmula estiver certa, dar o estado
    certo tem que produzir a medida certa em QUALQUER ponto da trajetória, não só no nominal — isso
    isola `Reactor` por completo do resto da planta, então qualquer divergência encontrada aqui é,
    por eliminação, um bug de fórmula dentro do próprio `Reactor`, não de acoplamento.

    Desde o Exp 24, `twr` é uma constante congelada (`REACTOR_COOLING_WATER_RETURN`) — a
    comparação contra `XMEAS(21)` continua valiosa (confirma que a constante escolhida bate com o
    que o baseline validado realmente mediu), só que agora o limiar pode ser bem mais apertado, já
    que não sobra nenhuma dependência de `tcr`/nível/agitação pra propagar erro nenhum: a única
    fonte de diferença é o ruído de medição do próprio CSV.
    */
    #[test]
    fn physical_state_and_heat_exchange_match_the_validated_csv_at_many_points_along_the_real_trajectory() {
        let csv = std::fs::read_to_string("docs/simulations/simulation_log_13.csv")
            .expect("docs/simulations/simulation_log_13.csv deveria existir (cópia do Exp 13)");

        let mut max_temperature_diff = 0.0f64;
        let mut max_pressure_diff = 0.0f64;
        let mut max_twr_diff = 0.0f64;

        for (row_index, line) in csv.lines().skip(1).take(200).enumerate() {
            let fields: Vec<f64> = line.split(',').map(|f| f.parse().expect("campo deveria ser numerico")).collect();
            let xmeas7 = fields[7];
            let xmeas9 = fields[9];
            let xmeas21 = fields[21];
            let yy = &fields[36..45]; /* YY[0..8]: reactor A,B,C,D,E,F,G,H,energy */

            let registry = StateRegistry::shared();
            let initial = Snapshot::from_pairs(&[
                ("state.reactor_vapor.A", yy[0]),
                ("state.reactor_vapor.B", yy[1]),
                ("state.reactor_vapor.C", yy[2]),
                ("state.reactor_vapor.D", yy[3]),
                ("state.reactor_vapor.E", yy[4]),
                ("state.reactor_vapor.F", yy[5]),
                ("state.reactor_vapor.G", yy[6]),
                ("state.reactor_vapor.H", yy[7]),
                ("state.reactor.energy", yy[8]),
            ]);
            let reactor = Reactor::new(&mut registry.borrow_mut(), &initial);

            let (temperature, _temperature_k, pressure, volume_liquid, _density, _volume_vapor, _total_vapor_moles, _heat_of_reaction, _liquid_composition, _vapor_composition, _vapor_moles, _reaction_rates) =
                reactor.__physical_state_impl();
            let xmeas_pressure = (pressure - 760.0) / 760.0 * 101.325;
            let (_reactor_heat, twr) = reactor.__heat_exchange_impl(volume_liquid, temperature, fields[34] /* XMV(12) agitator */);

            let temperature_diff = (temperature - xmeas9).abs();
            let pressure_diff = (xmeas_pressure - xmeas7).abs();
            let twr_diff = (twr - xmeas21).abs();

            max_temperature_diff = max_temperature_diff.max(temperature_diff);
            max_pressure_diff = max_pressure_diff.max(pressure_diff);
            max_twr_diff = max_twr_diff.max(twr_diff);

            if row_index < 20 || row_index % 20 == 0 {
                println!(
                    "tick={row_index:4} T calc={temperature:9.4} ref={xmeas9:9.4} \u{394}T={temperature_diff:8.5} | \
                    P calc={xmeas_pressure:9.4} ref={xmeas7:9.4} \u{394}P={pressure_diff:8.5} | \
                    twr calc={twr:9.4} ref={xmeas21:9.4} \u{394}twr={twr_diff:8.5}"
                );
            }
        }

        println!(
            "\nmax |\u{394}| em 200 ticks: temperatura={max_temperature_diff:.6}\u{b0}C pressao={max_pressure_diff:.6}kPa twr={max_twr_diff:.6}\u{b0}C"
        );

        /* Limiares: XMEAS(9) σ=0.01, XMEAS(7) σ=0.3, XMEAS(21) σ=0.01 (Block 37/XNS de `v1.0.0`).
        `twr` agora é uma constante pura, então seu limiar pode ser bem mais apertado que antes do
        Exp 24 (era 0.5, folgado pra absorver o viés de fórmula que já não existe mais).
        */
        assert!(
            max_temperature_diff < 0.05 && max_pressure_diff < 1.0 && max_twr_diff < 0.1,
            "physical_state()/heat_exchange() nao reproduzem a trajetoria validada dado o MESMO \
            estado de entrada - a formula diverge da fisica correta mesmo fora do ponto nominal \
            (max dT={max_temperature_diff}, max dP={max_pressure_diff}, max dtwr={max_twr_diff})"
        );
    }
}
