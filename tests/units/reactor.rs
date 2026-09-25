/* Testes de `units::reactor` — ver docs/12-reator.md pra o que cada tarefa faz. */

use crate::harness::Harness;
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::units::reactor::Reactor;

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
