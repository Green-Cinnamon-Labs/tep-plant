/* tep/units/reactor.rs */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use monjolo::chemistry::{liquid_density, temperature_from_enthalpy, Mixture, Phase, Reaction, ReactionScheme};

const REACTOR_VOLUME: f64 = 1300.0; /* volume total do vaso do reator [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
/* As 4 reações do TEP sobre A..H (índices 0..7): estequiometria por kmol de avanço e calor liberado
[kJ/kmol]. Só as reações 1 e 2 liberam calor no modelo original (teprob.f).
  1: A + C + D → G   2: A + C + E → H   3: A + E → F   4: 1,5 D → F
*/
const TEP_REACTIONS: ReactionScheme<8, 4> = ReactionScheme {
    stoichiometry: [
        [-1.0, 0.0, -1.0, -1.0, 0.0, 0.0, 1.0, 0.0],
        [-1.0, 0.0, -1.0, 0.0, -1.0, 0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, -1.5, 0.0, 1.0, 0.0, 0.0],
    ],
    enthalpies: [0.06899381054, 0.05, 0.0, 0.0],
};
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

/* Cinética de Arrhenius — taxa bruta de cada uma das 4 reações de `TEP_REACTIONS`, dadas a
temperatura e as pressões parciais. A estequiometria (quem consome/produz o quê) e o calor total
ficam por conta do próprio `Reaction`, não de quem chama.
*/
fn kinetics(temperature_k: f64, partial_pressures: &Mixture<8>, volume_vapor: f64) -> Reaction<8, 4> {
    let p = |i: usize| partial_pressures.component(i);

    let mut rates = [0.0f64; 4];
    rates[0] = (31.5859536 - 40000.0 / 1.987 / temperature_k).exp() * REACTION_FACTOR_1_NOMINAL;
    rates[1] = (3.00094014 - 20000.0 / 1.987 / temperature_k).exp() * REACTION_FACTOR_2_NOMINAL;
    rates[2] = (53.4060443 - 60000.0 / 1.987 / temperature_k).exp();
    rates[3] = rates[2] * 0.767488334;
    if p(0) > 0.0 && p(2) > 0.0 {
        let rf1 = p(0).powf(1.1544);
        let rf2 = p(2).powf(0.3735);
        rates[0] *= rf1 * rf2 * p(3);
        rates[1] *= rf1 * rf2 * p(4);
    } else {
        rates[0] = 0.0;
        rates[1] = 0.0;
    }
    rates[2] *= p(0) * p(4);
    rates[3] *= p(0) * p(3);
    for r in rates.iter_mut() {
        *r *= volume_vapor;
    }

    Reaction::new(rates, &TEP_REACTIONS)
}

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

/* Os sinais (`need::nome` lido, `offer::nome = valor;` escrito) moram dentro de cada método — o nome
é a chave, `__` vira `.` (`reactor__temperature` = "reactor.temperature"). Uma mistura
(`::<Vapor>`) é UM valor, publicado como 8 chaves `.a`..`.h`.
*/
#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Reactor {
    /* Bloco 1: balanço de energia próprio → temperatura/pressão/composição/cinética — igual ao
    `compute()` monolítico de antes. Não lê nenhum `need::`: só o próprio `#[state]`.
    */
    #[task]
    fn physical_state(&self) {
        /* `vapor()`/`liquid()` são os dois grupos do estado (A/B/C e D-H, separados só pela chave de
        config); `Mixture::at` posiciona cada um no seu lugar de um total de 8, zero no resto.
        */
        let vapor = Mixture::at(0, &self.vapor(), Phase::Vapor, &TEP_SPECIES);
        let liquid = Mixture::at(3, &self.liquid(), Phase::Liquid, &TEP_SPECIES);
        let liquid_composition = liquid.mole_fractions();

        let specific_enthalpy = self.enthalpy() / liquid.total();
        let temperature = temperature_from_enthalpy(&liquid_composition.as_array(), TEMPERATURE_SEED, specific_enthalpy, 0, &self.constants);
        let temperature_k = temperature + 273.15;
        let density = liquid_density(&liquid_composition.as_array(), temperature, &self.constants);
        let volume_liquid = liquid.total() / density;
        let volume_vapor = REACTOR_VOLUME - volume_liquid;

        /* A/B/C: gás ideal a partir dos moles de vapor; D-H: Antoine × fração líquida. Cada `Mixture`
        só é não-zero na própria faixa de índices, então `+` já junta as duas.
        */
        let partial_pressures = vapor.ideal_gas_pressure(temperature_k, volume_vapor, GAS_CONSTANT) + liquid_composition.vapor_pressure(temperature, &self.constants);
        let pressure = partial_pressures.total();
        let vapor_composition = partial_pressures.mole_fractions();

        let reaction = kinetics(temperature_k, &partial_pressures, volume_vapor);

        offer::reactor__temperature = temperature;
        offer::reactor__pressure = pressure;
        offer::reactor__liquid_volume = volume_liquid;
        offer::reactor__heat_of_reaction = reaction.heat();
        offer::reactor__vapor_composition::<Vapor> = vapor_composition;
        offer::reactor__reaction_rates::<Mixed> = Mixture::new(reaction.species_rates(), Phase::Mixed, &TEP_SPECIES);
    }

    /* Bloco 2 (ex-Flows, Block 23 slot 7): vazão pro separador, dependente de ΔP (sem válvula). */
    #[task]
    fn flow_to_separator(&self) {
        let mol_weight = need::reactor__vapor_composition::<Vapor>.dot(&self.constants.xmw);
        offer::flows__stream_flow__7 = 4574.21 * (need::reactor__pressure - need::separator__pressure).max(0.0).sqrt() * (1.0 - 0.25 * 0.0) / mol_weight /* disturbance channel 11, neutro */;
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
    #[task]
    fn heat_exchange(&self) {
        /* UARLEV: fração da serpentina submersa, 0 abaixo de level=10 (seca, sem troca), rampa
        linear até level=50, platô em 1.0 dali pra cima (totalmente submersa — mais líquido não
        aumenta mais nada).
        */
        let agitation_factor = (need::agitator__speed + 150.0) / 100.0;
        let level = need::reactor__liquid_volume / 7.8; /* 7.8 = fator de conversão de volume pra "nível" deste bloco */
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

        offer::heat__reactor_heat = uar * (twr - need::reactor__temperature) * (1.0 - 0.35 * 0.0) /* disturbance channel 9 (IDV 17), neutro */;
        offer::heat__reactor_cooling_water_return = twr;
    }

    /* Bloco 4 (ex-Derivatives, Block 40 YP(1..9)): balanço de massa/energia do próprio estado —
    a última EDO que faltava. Entalpias recomputadas frescas, mesmo padrão das outras 3 unidades.
    */
    #[task]
    fn mass_and_energy_balance(&self) {
        let compressor_vapor = need::compressor__vapor_composition::<Vapor>;
        let reactor_vapor = need::reactor__vapor_composition::<Vapor>;
        let compressor_recycle_flow = need::flows__stream_flow__6;
        let outlet_flow = need::flows__stream_flow__7;

        let derivative = compressor_vapor.scaled_by(compressor_recycle_flow) - reactor_vapor.scaled_by(outlet_flow) + need::reactor__reaction_rates::<Mixed>;

        offer::reactor__state__vapor_a__derivative = derivative.component(0);
        offer::reactor__state__vapor_b__derivative = derivative.component(1);
        offer::reactor__state__vapor_c__derivative = derivative.component(2);
        offer::reactor__state__liquid_d__derivative = derivative.component(3);
        offer::reactor__state__liquid_e__derivative = derivative.component(4);
        offer::reactor__state__liquid_f__derivative = derivative.component(5);
        offer::reactor__state__liquid_g__derivative = derivative.component(6);
        offer::reactor__state__liquid_h__derivative = derivative.component(7);
        offer::reactor__state__enthalpy__derivative = compressor_vapor.enthalpy(need::compressor__temperature, 1, &self.constants) * compressor_recycle_flow
            - reactor_vapor.enthalpy(need::reactor__temperature, 1, &self.constants) * outlet_flow
            + need::reactor__heat_of_reaction
            + need::heat__reactor_heat;
    }

    /* Bloco 5 (ex-measured.rs, Block 35): XMEAS 7-9 (pressão/nível/temperatura do reator) + XMEAS
    21 (temperatura de saída da água de resfriamento) — conversões preservadas exatamente do
    original: (P-760)/760*101.325 (mmHg gauge → kPa gauge), volume/666.7*100 (calibração do
    instrumento de nível).
    */
    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__reactor__pressure = (need::reactor__pressure - 760.0) / 760.0 * 101.325;
        offer::xmeas__reactor__level = (need::reactor__liquid_volume - 84.6) / 666.7 * 100.0;
        offer::xmeas__reactor__temperature = need::reactor__temperature;
        offer::xmeas__reactor__cooling_water_outlet_temperature = need::heat__reactor_cooling_water_return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::harness::Harness;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    const NOMINAL_STATE: [(&str, f64); 9] = [
        ("state.reactor_vapor.A", 10.679592788064898),
        ("state.reactor_vapor.B", 4.666690921054032),
        ("state.reactor_vapor.C", 7.668821577187304),
        ("state.reactor_vapor.D", 0.3968744850195289),
        ("state.reactor_vapor.E", 22.74175383132296),
        ("state.reactor_vapor.F", 2.9441162740614635),
        ("state.reactor_vapor.G", 148.4668245572372),
        ("state.reactor_vapor.H", 153.02693445529974),
        ("state.reactor.energy", 2.6983891373872186),
    ];

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
        let harness = Harness::new();
        let _reactor = harness.unit(|registry| Reactor::new(registry, &Snapshot::from_pairs(&NOMINAL_STATE)));
        let task = harness.task("Reactor::physical_state");
        harness.resolve();
        task.evaluate();

        let temperature = harness.read("reactor.temperature");
        let pressure = harness.read("reactor.pressure");
        let heat_of_reaction = harness.read("reactor.heat_of_reaction");
        let reaction_rates = harness.read_mixture("reactor.reaction_rates", 8);

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

    /* Semeia todas as entradas de `mass_and_energy_balance` com fluxos BALANCEADOS (mesma
    composição/temperatura/vazão entrando e saindo) e roda a tarefa. */
    fn run_balanced_mass_and_energy_balance(reaction_rates: [f64; 8], heat_of_reaction: f64, reactor_heat: f64) -> Harness {
        let harness = Harness::new();
        let _reactor = harness.unit(|registry| Reactor::new(registry, &Snapshot::from_pairs(&[])));

        let composition = [0.1, 0.05, 0.1, 0.05, 0.2, 0.1, 0.2, 0.2];
        let temperature = 120.0;
        let flow = 500.0;

        harness.seed_mixture("compressor.vapor_composition", &composition);
        harness.seed("compressor.temperature", temperature);
        harness.seed("flows.stream_flow.6", flow);
        harness.seed("flows.stream_flow.7", flow);
        harness.seed_mixture("reactor.vapor_composition", &composition);
        harness.seed("reactor.temperature", temperature);
        harness.seed_mixture("reactor.reaction_rates", &reaction_rates);
        harness.seed("reactor.heat_of_reaction", heat_of_reaction);
        harness.seed("heat.reactor_heat", reactor_heat);

        let task = harness.task("Reactor::mass_and_energy_balance");
        harness.resolve();
        task.evaluate();
        harness
    }

    /* `mass_and_energy_balance` consome os sinais de `physical_state`/`heat_exchange`/fluxos de
    outras unidades — testado aqui com entradas balanceadas (mesma composição/temperatura/vazão
    entrando e saindo) pra confirmar que os termos de fluxo se cancelam exatamente, sobrando só
    reação/calor. Não depende de nenhum valor "nominal" hipotético — é uma verificação estrutural
    da equação de balanço, útil mesmo sem saber o ponto de operação de cor.
    */
    #[test]
    fn mass_and_energy_balance_cancels_flow_terms_when_inflow_equals_outflow() {
        let harness = run_balanced_mass_and_energy_balance([0.0; 8], 0.0, 0.0);

        for key in [
            "reactor.state.vapor_a.derivative",
            "reactor.state.vapor_b.derivative",
            "reactor.state.vapor_c.derivative",
            "reactor.state.liquid_d.derivative",
            "reactor.state.liquid_e.derivative",
            "reactor.state.liquid_f.derivative",
            "reactor.state.liquid_g.derivative",
            "reactor.state.liquid_h.derivative",
        ] {
            assert_eq!(harness.read(key), 0.0, "mesma composição/vazão entrando e saindo não deveria acumular nada em {key}");
        }
        assert_eq!(
            harness.read("reactor.state.enthalpy.derivative"),
            0.0,
            "mesma composição/temperatura/vazão não deveria gerar entalpia líquida"
        );
    }

    #[test]
    fn mass_and_energy_balance_passes_through_reaction_and_heat_when_flows_are_balanced() {
        let reaction_rates = [-3.0, 0.0, -2.0, -1.5, -1.0, 1.5, 2.0, 1.0];
        let heat_of_reaction = 10.0;
        let reactor_heat = -4.0;
        let harness = run_balanced_mass_and_energy_balance(reaction_rates, heat_of_reaction, reactor_heat);

        /* Com os fluxos cancelados, a derivada de cada componente É a taxa de reação — sem
        surpresa de índice trocado entre as derivadas de vapor (0..3) e de líquido (3..8).
        */
        let derivative_keys = [
            "reactor.state.vapor_a.derivative",
            "reactor.state.vapor_b.derivative",
            "reactor.state.vapor_c.derivative",
            "reactor.state.liquid_d.derivative",
            "reactor.state.liquid_e.derivative",
            "reactor.state.liquid_f.derivative",
            "reactor.state.liquid_g.derivative",
            "reactor.state.liquid_h.derivative",
        ];
        for (i, key) in derivative_keys.iter().enumerate() {
            assert_eq!(harness.read(key), reaction_rates[i], "derivada de {key} deveria ser a taxa de reação");
        }
        assert_eq!(harness.read("reactor.state.enthalpy.derivative"), heat_of_reaction + reactor_heat);
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

            let harness = Harness::new();
            let _reactor = harness.unit(|registry| Reactor::new(registry, &initial));
            harness.seed("agitator.speed", fields[34] /* XMV(12) agitator */);
            let physical_state = harness.task("Reactor::physical_state");
            let heat_exchange = harness.task("Reactor::heat_exchange");
            harness.resolve();

            physical_state.evaluate();
            heat_exchange.evaluate();

            let temperature = harness.read("reactor.temperature");
            let xmeas_pressure = (harness.read("reactor.pressure") - 760.0) / 760.0 * 101.325;
            let twr = harness.read("heat.reactor_cooling_water_return");

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
