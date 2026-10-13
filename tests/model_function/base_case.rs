/* A planta inteira avaliada como função no caso base de Downs & Vogel
(`docs/cases/te_mode1_initial_state.toml`, os `YY` de `TEINIT`), sem controladores, sem sensores e
sem laço de simulação. É o ponto de parada do plano da spec-tennessee-eastman#91: se isto não
fechar, a física difere do FORTRAN ou a função não é pura, e nada se constrói por cima.

Referência das medições e dos custos: Ricker (1995), "Optimal steady-state operation of the
Tennessee Eastman challenge process", Tabelas 2 e 4, coluna "Base case".
*/

use monjolo::model_function::ModelFunction;
use monjolo::snapshot::Snapshot;

const BASE_CASE: &str = "docs/cases/te_mode1_initial_state.toml";

fn base_case() -> ModelFunction {
    let config = Snapshot::from_file(BASE_CASE).expect("o caso base deveria existir e ser um TOML válido");
    ModelFunction::discovered(&config).expect("a planta sem controladores deveria ser uma função do ponto")
}

fn is_valve(state: &str) -> bool {
    state.starts_with("valve.") || state == "agitator.speed"
}

fn all_keys(function: &ModelFunction) -> Vec<&str> {
    function.keys().iter().map(String::as_str).collect()
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

/* Fator determinístico em [1 - spread, 1 + spread], diferente pra cada (ponto, estado). */
fn factor(point: usize, state: usize, spread: f64) -> f64 {
    let hash = (point * 7919 + state * 104729 + 13) % 1000;
    1.0 - spread + 2.0 * spread * (hash as f64 / 999.0)
}

/* Um ponto perto do caso base: cada estado multiplicado pelo seu fator; válvulas limitadas a 100 %. */
fn perturbed_point(function: &ModelFunction, point: usize, spread: f64) -> Vec<(String, f64)> {
    function
        .state_keys()
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let value = function.base(state).unwrap() * factor(point, index, spread);
            (state.clone(), if is_valve(state) { value.min(100.0) } else { value })
        })
        .collect()
}

fn as_writes(point: &[(String, f64)]) -> Vec<(&str, f64)> {
    point.iter().map(|(key, value)| (key.as_str(), *value)).collect()
}

/* Os 50 estados de Downs & Vogel: 4 vasos × 9, as 2 temperaturas da água de resfriamento e as 12
válvulas. Nenhuma chave de controlador.
*/
#[test]
fn has_the_fifty_states_of_the_original_and_no_controller() {
    let function = base_case();

    assert_eq!(function.state_keys().len(), 50);
    assert_eq!(function.state_keys().iter().filter(|state| !is_valve(state)).count(), 38);
    assert!(function.keys().iter().all(|key| !key.starts_with("controller.")));
}

/* O caso base publicado é um ponto de regime: as 38 derivadas físicas ficam próximas de zero.

Valores observados em 2026-10-13 (rode com `--nocapture` pra ver todos): fora do reator, o maior
módulo é 5.3e-6; no reator ficam entre 3.7e-7 e 9.3e-4, com o maior valor relativo ao estado em
1.6e-3 por hora (líquido D, 0.42 kmol). As duas derivadas da água de resfriamento, cujas parcelas
valem ~1700 °C/h cada, fecham em 4.4e-7 e 3.3e-7 °C/h.
*/
#[test]
fn physical_derivatives_are_near_zero_at_the_base_case() {
    let function = base_case();

    for state in function.state_keys().iter().filter(|state| !is_valve(state)) {
        let value = function.base(state).unwrap();
        let derivative = function.base(&format!("{state}.derivative")).unwrap();
        println!("{state:48} x = {value:16.9}   dx/dt = {derivative:13.6e}   (dx/dt)/x = {:10.3e}", derivative / value);

        let limit = if state.starts_with("reactor.state.") && !state.ends_with("cooling_water_temperature") {
            2.0e-3
        } else {
            1.0e-5
        };
        assert!(derivative.abs() < limit, "derivada de {state} longe de zero: {derivative}");
        assert!((derivative / value).abs() < 2.0e-3, "derivada relativa de {state} longe de zero: {}", derivative / value);
    }
}

/* Com o comando igual à posição, nenhuma válvula se move. */
#[test]
fn valve_derivatives_are_zero_at_the_base_case() {
    let function = base_case();

    for state in function.state_keys().iter().filter(|state| is_valve(state)) {
        assert_eq!(function.base(&format!("{state}.derivative")), Some(0.0), "{state}");
    }
}

/* (chave, valor da Tabela 2, casas decimais impressas na tabela) — as 41 XMEAS, na ordem do artigo. */
const TABLE_2_BASE_CASE: [(&str, f64, i32); 41] = [
    ("xmeas.stream1.flow_rate", 0.251, 3),
    ("xmeas.stream2.flow_rate", 3664.0, 0),
    ("xmeas.stream3.flow_rate", 4509.0, 0),
    ("xmeas.stream4.flow_rate", 9.35, 2),
    ("xmeas.stream8.flow_rate", 26.90, 2),
    ("xmeas.stream6.flow_rate", 42.34, 2),
    ("xmeas.reactor.pressure", 2705.0, 0),
    ("xmeas.reactor.level", 75.0, 0),
    ("xmeas.reactor.temperature", 120.4, 1),
    ("xmeas.stream9.flow_rate", 0.337, 3),
    ("xmeas.separator.temperature", 80.1, 1),
    ("xmeas.separator.level", 50.0, 0),
    ("xmeas.separator.pressure", 2634.0, 0),
    ("xmeas.stream10.flow_rate", 25.16, 2),
    ("xmeas.stripper.level", 50.0, 0),
    ("xmeas.stripper.pressure", 3102.0, 0),
    ("xmeas.stream11.flow_rate", 22.95, 2),
    ("xmeas.stripper.temperature", 65.73, 2),
    ("xmeas.stripper.steam_flow_rate", 230.0, 0),
    ("xmeas.compressor.work", 341.0, 0),
    ("xmeas.reactor.cooling_water_outlet_temperature", 94.6, 1),
    ("xmeas.separator.cooling_water_outlet_temperature", 77.3, 1),
    ("xmeas.stream6.component.a", 32.19, 2),
    ("xmeas.stream6.component.b", 8.89, 2),
    ("xmeas.stream6.component.c", 26.38, 2),
    ("xmeas.stream6.component.d", 6.88, 2),
    ("xmeas.stream6.component.e", 18.78, 2),
    ("xmeas.stream6.component.f", 1.66, 2),
    ("xmeas.stream9.component.a", 32.96, 2),
    ("xmeas.stream9.component.b", 13.82, 2),
    ("xmeas.stream9.component.c", 23.98, 2),
    ("xmeas.stream9.component.d", 1.26, 2),
    ("xmeas.stream9.component.e", 18.58, 2),
    ("xmeas.stream9.component.f", 2.26, 2),
    ("xmeas.stream9.component.g", 4.84, 2),
    ("xmeas.stream9.component.h", 2.30, 2),
    ("xmeas.stream11.component.d", 0.02, 2),
    ("xmeas.stream11.component.e", 0.84, 2),
    ("xmeas.stream11.component.f", 0.10, 2),
    ("xmeas.stream11.component.g", 53.72, 2),
    ("xmeas.stream11.component.h", 43.83, 2),
];

/* Cada medição, arredondada pras casas que a tabela imprime, dá o número da tabela. */
#[test]
fn measurements_match_table_2_of_ricker_1995() {
    let function = base_case();

    for (key, expected, decimals) in TABLE_2_BASE_CASE {
        let value = function.base(key).unwrap_or_else(|| panic!("{key} deveria existir"));
        let half_last_digit = 0.5 * 10f64.powi(-decimals) + 1e-9;
        assert!((value - expected).abs() <= half_last_digit, "{key}: obteve {value}, a tabela diz {expected}");
    }
}

/* Equação 3 do artigo, parcela por parcela (Tabela 4, coluna "Base case"): 114.71 de purga, 30.28
de produto, 18.30 de compressor e 7.32 de vapor, total 170.61 $/h. O produto usa a posição da
válvula de produto (`x46`), como no artigo.
*/
#[test]
fn operating_cost_matches_table_4_of_ricker_1995() {
    let function = base_case();
    let y = |key: &str| function.base(key).unwrap();

    let purge = 0.44791
        * y("xmeas.stream9.flow_rate")
        * (2.209 * y("xmeas.stream9.component.a")
            + 6.177 * y("xmeas.stream9.component.c")
            + 22.06 * y("xmeas.stream9.component.d")
            + 14.56 * y("xmeas.stream9.component.e")
            + 17.89 * y("xmeas.stream9.component.f")
            + 30.44 * y("xmeas.stream9.component.g")
            + 22.94 * y("xmeas.stream9.component.h"));
    let product = 4.541
        * y("valve.stripper_product.position")
        * (0.2206 * y("xmeas.stream11.component.d")
            + 0.1456 * y("xmeas.stream11.component.e")
            + 0.1789 * y("xmeas.stream11.component.f"));
    let compressor = 0.0536 * y("xmeas.compressor.work");
    let steam = 0.0318 * y("xmeas.stripper.steam_flow_rate");
    let total = purge + product + compressor + steam;
    println!("purga {purge:.3}  produto {product:.3}  compressor {compressor:.3}  vapor {steam:.3}  total {total:.3} $/h");

    assert!((purge - 114.71).abs() < 0.05, "purga: {purge}");
    assert!((product - 30.28).abs() < 0.05, "produto: {product}");
    assert!((compressor - 18.30).abs() < 0.05, "compressor: {compressor}");
    assert!((steam - 7.32).abs() < 0.05, "vapor: {steam}");
    assert!((total - 170.61).abs() < 0.05, "total: {total}");
}

/* Teste de memória, forma 1: A, B, A, com B distante de A (cada estado deslocado em até 15 %, o que
muda a pressão do reator em centenas de kPa). Compara TODAS as chaves, bit a bit.
*/
#[test]
fn point_a_gives_the_same_result_before_and_after_a_distant_point_b() {
    let function = base_case();
    let keys = all_keys(&function);
    let point_b = perturbed_point(&function, 1, 0.15);

    let first = function.evaluate(&[], &keys).unwrap();
    let distant = function.evaluate(&as_writes(&point_b), &keys).unwrap();
    let again = function.evaluate(&[], &keys).unwrap();

    let pressure = keys.iter().position(|key| *key == "xmeas.reactor.pressure").unwrap();
    assert!(
        (distant[pressure] - first[pressure]).abs() > 100.0,
        "B deveria estar longe de A: pressão do reator {} contra {}",
        distant[pressure],
        first[pressure]
    );
    assert_eq!(bits(&first), bits(&again));
}

/* Teste de memória, forma 2: uma sequência de pontos variados numa instância reaproveitada dá, pra
cada ponto, exatamente o que uma instância recém-criada dá. Cobre o que o A, B, A não cobre
sozinho: qualquer coisa que dependa de quais pontos vieram antes, em qualquer ordem.
*/
#[test]
fn a_reused_instance_matches_a_fresh_instance_at_every_point() {
    let reused = base_case();
    let keys = all_keys(&reused);

    for point in 0..12 {
        let spread = if point % 2 == 0 { 0.03 } else { 0.10 };
        let writes = perturbed_point(&reused, point, spread);

        let from_reused = reused.evaluate(&as_writes(&writes), &keys).unwrap();
        let from_fresh = base_case().evaluate(&as_writes(&writes), &keys).unwrap();

        assert_eq!(bits(&from_reused), bits(&from_fresh), "ponto {point} depende do que veio antes");
    }
}
