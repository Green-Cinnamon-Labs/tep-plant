/* Testes de `units::compressor`. */

use crate::harness::Harness;
use monjolo::chemistry::{Mixture};
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::physics::constants::{TepConstants, TEP_SPECIES};
use tennessee_eastman_process::units::compressor::Compressor;
use tennessee_eastman_process::units::separator::Separator;

/* Investigação do Experimento 20 (spec-tennessee-eastman/experimentos.md), continuação do Exp
19: o Exp 19 descartou `Reactor::physical_state`/`mass_and_energy_balance` como origem do
colapso ainda observado depois dos dois fixes de água de resfriamento (Exp 18). Esta seção
testa as tarefas análogas de `Compressor` — cada uma isolada, com entradas semeadas por chave
— não precisa da planta inteira.
*/

#[test]
fn physical_state_matches_nominal_operating_point_from_application_toml() {
    /* `separator.temperature` é uma entrada externa aqui — em vez de chutar um número,
    computamos ele de verdade a partir de `Separator::physical_state` com o próprio estado
    nominal do separador (mesma técnica de "confiar só em valores já validados", não em números
    de memória). Exp 19 já confirmou `Reactor::physical_state` correto; usamos 120°C (nominal
    documentado) como a temperatura do reator que o separador precisa. As duas unidades vivem
    no MESMO registry, e a tarefa do compressor lê o slot que a do separador escreve.
    */
    let harness = Harness::new();
    let _separator = harness.unit(|registry| {
        Separator::new(
            registry,
            &Snapshot::from_pairs(&[
                ("state.separator_vapor.A", 63.337045809835196),
                ("state.separator_vapor.B", 27.67808577797886),
                ("state.separator_vapor.C", 45.480330241132435),
                ("state.separator_vapor.D", 0.2398728094022691),
                ("state.separator_vapor.E", 14.845882843614561),
                ("state.separator_vapor.F", 1.9220259408956704),
                ("state.separator_vapor.G", 52.521987477541764),
                ("state.separator_vapor.H", 41.289131421711694),
                ("state.separator.energy", 0.571326790156296),
            ]),
        )
    });

    let initial = Snapshot::from_pairs(&[
        ("state.compressor_vapor.A", 107.18718027273283),
        ("state.compressor_vapor.B", 30.801210941908113),
        ("state.compressor_vapor.C", 87.22311903671716),
        ("state.compressor_vapor.D", 22.921606015837646),
        ("state.compressor_vapor.E", 61.9020790931974),
        ("state.compressor_vapor.F", 5.777503238613575),
        ("state.compressor_vapor.G", 12.022672768816811),
        ("state.compressor_vapor.H", 5.609107990171663),
        ("state.compressor.energy", 0.9193937935424599),
    ]);
    let _compressor = harness.unit(|registry| Compressor::new(registry, &initial));

    harness.seed("reactor.temperature", 120.0);
    let separator_state = harness.task("Separator::physical_state");
    let compressor_state = harness.task("Compressor::physical_state");
    harness.resolve();

    separator_state.evaluate();
    compressor_state.evaluate();

    let separator_temperature = harness.read("separator.temperature");
    let temperature = harness.read("compressor.temperature");
    let pressure = harness.read("compressor.pressure");

    /* Faixas com folga generosa (mesmo espírito do teste equivalente em reactor.rs) — o
    objetivo é pegar um colapso grosseiro, não validar casas decimais de um nominal que este
    arquivo não documentava antes.
    */
    assert!(
        (40.0..90.0).contains(&temperature),
        "esperava compressor.temperature numa faixa plausível (a partir de separator_temperature={separator_temperature}°C), obteve {temperature}°C"
    );

    /* XMEAS(16) roda genuinamente mais alto que a pressão do reator/separador (~2700/~2633) —
    não é bug, é a pressão de descarga do próprio compressor. Faixa (2900,3300) escolhida
    depois de rodar este teste pela primeira vez com um chute errado (~2700) e ver 3091.3 kPa
    sair — valor plausível (perto do nominal ~3100 kPa comumente citado na literatura do TEP
    pra esta variável), não uma falha.
    */
    let xmeas_stripper_pressure = (pressure - 760.0) / 760.0 * 101.325;
    assert!(
        (2900.0..3300.0).contains(&xmeas_stripper_pressure),
        "esperava XMEAS(16) (pressão do compressor, apesar do nome) numa faixa plausível (~3100 kPa), obteve {xmeas_stripper_pressure} kPa"
    );
}

/* `mass_and_energy_balance`: com feeds frescos zerados (streams 1/2/3=0), sem vapor de flash
(flash_vapor_flow=0) e o reciclo balanceado (mesma composição/entalpia entrando e saindo, na
mesma vazão), nada deveria se acumular — mesma técnica de "fluxo balanceado" já usada nos
testes de `Reactor::mass_and_energy_balance`.
*/
#[test]
fn mass_and_energy_balance_cancels_when_only_recycle_flow_is_present_and_balanced() {
    let harness = Harness::new();
    let _compressor = harness.unit(|registry| Compressor::new(registry, &Snapshot::from_pairs(&[])));

    let composition = [0.1, 0.05, 0.1, 0.05, 0.2, 0.1, 0.2, 0.2];
    let temperature = 45.0;
    let flow = 300.0;
    let discharge_enthalpy = Mixture::new(composition, &TEP_SPECIES).enthalpy(temperature, 1, &TepConstants::new());

    harness.seed("flows.stream_flow.1", 0.0); /* sem feed fresco (stream 1 = A) */
    harness.seed("flows.stream_flow.2", 0.0); /* stream 2 = D */
    harness.seed("flows.stream_flow.3", 0.0); /* stream 3 = E */
    harness.seed("flows.stream_flow.5", 0.0); /* sem vazão de flash (stream 5, o vetor flash_vapor_flow já é zero) */
    harness.seed("flows.stream_flow.6", flow); /* reciclo saindo (stream 6, reactor feed) */
    harness.seed("flows.stream_flow.8", flow); /* reciclo entrando (stream 8), mesma vazão */
    harness.seed_mixture("flows.flash_vapor_component_flow", &[0.0; 8]); /* sem vapor de flash */
    harness.seed_mixture("separator.vapor_composition", &composition); /* mesma composição do compressor */
    harness.seed("stripper.temperature", temperature); /* não importa aqui (flash_vapor_flow=0) */
    harness.seed("flows.compressor_discharge_enthalpy", discharge_enthalpy);
    harness.seed_mixture("compressor.vapor_composition", &composition);
    harness.seed("compressor.temperature", temperature);

    let task = harness.task("Compressor::mass_and_energy_balance");
    harness.resolve();
    task.evaluate();

    for i in 0..8 {
        let key = format!("compressor.state.{i}.derivative");
        assert_eq!(harness.read(&key), 0.0, "reciclo com mesma composição/vazão entrando e saindo não deveria acumular nada em {key}");
    }
    assert_eq!(
        harness.read("compressor.state.8.derivative"),
        0.0,
        "reciclo balanceado (mesma entalpia/vazão) não deveria gerar entalpia líquida"
    );
}

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
