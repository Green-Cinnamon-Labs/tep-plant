/* Testes de `units::separator`. */

use crate::harness::Harness;
use monjolo::chemistry::{Mixture, Phase};
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::physics::constants::{TepConstants, TEP_SPECIES};
use tennessee_eastman_process::units::separator::Separator;

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

/* Investigação do Experimento 20 (spec-tennessee-eastman/experimentos.md) — mesma técnica dos
testes equivalentes de `reactor.rs`/`compressor.rs`: rodar a tarefa isolada, com entradas
conhecidas semeadas por chave, sem precisar da planta inteira.
*/
#[test]
fn physical_state_matches_nominal_operating_point_from_application_toml() {
    let initial = Snapshot::from_pairs(&[
        ("state.separator_vapor.A", 63.337045809835196),
        ("state.separator_vapor.B", 27.67808577797886),
        ("state.separator_vapor.C", 45.480330241132435),
        ("state.separator_vapor.D", 0.2398728094022691),
        ("state.separator_vapor.E", 14.845882843614561),
        ("state.separator_vapor.F", 1.9220259408956704),
        ("state.separator_vapor.G", 52.521987477541764),
        ("state.separator_vapor.H", 41.289131421711694),
        ("state.separator.energy", 0.571326790156296),
    ]);

    let harness = Harness::new();
    let _separator = harness.unit(|registry| Separator::new(registry, &initial));
    harness.seed("reactor.temperature", 120.0); /* reactor.temperature nominal */
    let task = harness.task("Separator::physical_state");
    harness.resolve();
    task.evaluate();

    let temperature = harness.read("separator.temperature");
    let pressure = harness.read("separator.pressure");

    /* Faixas com folga generosa (mesmo espírito de reactor.rs/compressor.rs) — pegar um
    colapso grosseiro, não validar casas decimais de um nominal que este arquivo não
    documentava antes.
    */
    assert!(
        (60.0..100.0).contains(&temperature),
        "esperava separator.temperature numa faixa plausível (mais fria que o reator, ~120°C), obteve {temperature}°C"
    );

    let xmeas_pressure = (pressure - 760.0) / 760.0 * 101.325;
    assert!(
        (2400.0..2800.0).contains(&xmeas_pressure),
        "esperava XMEAS(13) perto do nominal ~2633 kPa, obteve {xmeas_pressure} kPa"
    );
}

/* Regressão direta do bug do Exp 18: `heat_exchange` sempre publica
`SEPARATOR_COOLING_WATER_RETURN` como temperatura de retorno (é uma constante congelada, não
calculada) — este teste existe só pra travar o valor certo (77.29698353, `[state.cooling].
separator_water_temp` de `application.toml`) contra uma futura regressão de volta pro 40.0
errado (`s_zero` do canal de distúrbio 5, TCWS — entrada, não retorno).
*/
#[test]
fn heat_exchange_returns_the_corrected_frozen_return_temperature() {
    let harness = Harness::new();
    let _separator = harness.unit(|registry| Separator::new(registry, &Snapshot::from_pairs(&[])));
    harness.seed("reactor.temperature", 120.0);
    harness.seed("flows.stream_flow.7", 4000.0);
    let task = harness.task("Separator::heat_exchange");
    harness.resolve();
    task.evaluate();

    assert_eq!(
        harness.read("heat.separator_cooling_water_return"),
        77.29698353,
        "regressão pro bug do Exp 18 — TCWS de entrada no lugar do tws de retorno"
    );
}

/* Semeia todas as entradas de `mass_and_energy_balance` com fluxos BALANCEADOS: o vapor do
reator entrando (`flow7`) e saindo em duas frações que somam exatamente o que entrou
(`flow8+flow9`, mesma composição, `flow10=0`) — nada deveria se acumular.
*/
fn run_balanced_mass_and_energy_balance(separator_heat: f64) -> Harness {
    let harness = Harness::new();
    let _separator = harness.unit(|registry| Separator::new(registry, &Snapshot::from_pairs(&[])));

    let composition = [0.1, 0.05, 0.1, 0.05, 0.2, 0.1, 0.2, 0.2];
    let temperature = 100.0;
    let enthalpy = Mixture::new(composition, Phase::Vapor, &TEP_SPECIES).enthalpy(temperature, 1, &TepConstants::new());

    harness.seed_mixture("reactor.vapor_composition", &composition);
    harness.seed("reactor.temperature", temperature);
    harness.seed("flows.stream_flow.7", 100.0); /* flow7: entrando */
    harness.seed("flows.stream_flow.8", 60.0); /* flow8: saindo (reciclo) */
    harness.seed("flows.stream_flow.9", 40.0); /* flow9: saindo (purge) — 60+40 = 100, balanceado */
    harness.seed("flows.stream_flow.10", 0.0); /* flow10: sem underflow líquido */
    harness.seed_mixture("separator.vapor_composition", &composition); /* mesma composição */
    harness.seed_mixture("separator.liquid_composition", &[0.0; 8]); /* sem líquido (flow10=0 de qualquer forma) */
    harness.seed("separator.temperature", temperature); /* mesma do reator, pra enthalpy_reactor_outlet == enthalpy_separator_vapor_uncorrected */
    harness.seed("flows.compressor_discharge_enthalpy", enthalpy); /* igual à entalpia da mesma composição/temperatura, cancela com flow8 */
    harness.seed("heat.separator_heat", separator_heat);

    let task = harness.task("Separator::mass_and_energy_balance");
    harness.resolve();
    task.evaluate();
    harness
}

/* `mass_and_energy_balance`: com o vapor do reator entrando e saindo em duas frações que somam
exatamente o que entrou, nada deveria se acumular — mesma técnica de "fluxo balanceado" de
`reactor.rs`/`compressor.rs`.
*/
#[test]
fn mass_and_energy_balance_cancels_when_outflow_matches_inflow_exactly() {
    let harness = run_balanced_mass_and_energy_balance(0.0);

    for key in [
        "separator.state.vapor_a.derivative",
        "separator.state.vapor_b.derivative",
        "separator.state.vapor_c.derivative",
        "separator.state.liquid_d.derivative",
        "separator.state.liquid_e.derivative",
        "separator.state.liquid_f.derivative",
        "separator.state.liquid_g.derivative",
        "separator.state.liquid_h.derivative",
    ] {
        assert_eq!(harness.read(key), 0.0, "vazão de saída igual à de entrada não deveria acumular nada em {key}");
    }
    assert_eq!(
        harness.read("separator.state.enthalpy.derivative"),
        0.0,
        "entalpias e vazões balanceadas não deveriam gerar entalpia líquida"
    );
}

#[test]
fn mass_and_energy_balance_passes_through_separator_heat_when_flows_are_balanced() {
    let separator_heat = -6.5;
    let harness = run_balanced_mass_and_energy_balance(separator_heat);

    assert_eq!(
        harness.read("separator.state.enthalpy.derivative"),
        separator_heat,
        "com fluxos e entalpias balanceados, só separator_heat deveria sobrar"
    );
}
