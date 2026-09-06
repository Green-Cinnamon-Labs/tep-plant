/* tep-plant/examples/golden_trace.rs */

/** Harness de verificação determinística (issue 10, Parte B da refatoração — scheduler de dataflow
topológico): captura N ticks de RK4 sem thread/sleep/rede, dirigindo o mesmo loop que
`Simulation::spawn_plant_thread` roda por dentro (ver `monjolo::simulation`), só que parando depois
de N passos em vez de rodar pra sempre, e despejando `registry.snapshot()` (todo StateSlot conhecido,
não só o que é `#[state]`) por tick num CSV.

Não é `#[test]`/`cargo test` — não faz nenhuma asserção, só produz dado bruto. O uso é comparar duas
rodadas: uma em `main` (antes da refatoração) e outra nesta branch (depois), linha a linha, com
tolerância — se os dois `.csv` convergirem para os mesmos valores (a menos de erro de ponto
flutuante por reassociação), a refatoração não mudou o comportamento físico da planta, só quem
calcula cada pedaço.

Uso: `cargo run --release --example golden_trace -- <n_ticks> <output.csv>`
(padrão: 3600 ticks, "golden_trace.csv" — `dt_hours` e o caminho de config são os MESMOS de
`src/main.rs`, não parâmetros daqui, já que o objetivo é reproduzir exatamente o que o binário real
faz por dentro).
*/
extern crate tennessee_eastman_process;

use monjolo::component::attach_discovered_components;
use monjolo::dynamic_model::{Composite, DynamicModel};
use monjolo::numerical_method::integrator::Integrator;
use monjolo::numerical_method::rk4::RK4;
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::{Proxy, StateRegistry};
use std::io::Write as _;

const CONFIG_PATH: &str = "config/application.toml";
const DT_HOURS: f64 = 1.0 / 3600.0;

fn main() {
    let mut args = std::env::args().skip(1);
    let ticks: usize = args.next().and_then(|value| value.parse().ok()).unwrap_or(3600);
    let output_path = args.next().unwrap_or_else(|| "golden_trace.csv".to_string());

    let config = Snapshot::from_file(CONFIG_PATH)
        .unwrap_or_else(|err| panic!("golden_trace: falha ao carregar config de '{CONFIG_PATH}': {err}"));

    let registry = StateRegistry::shared();
    let mut root = Composite::new();
    attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);
    let state_keys = root.state_keys();

    /* Mesmo truque de `spawn_plant_thread`: pede par (estado, ".derivative") ANTES do resolve()
    geral, pra sair com Proxy pareado na mesma ordem de state_keys.
    */
    let mut integration_needs: Vec<String> = Vec::with_capacity(state_keys.len() * 2);
    for key in &state_keys {
        integration_needs.push(key.clone());
        integration_needs.push(format!("{key}.derivative"));
    }
    let integration_need_refs: Vec<&str> = integration_needs.iter().map(String::as_str).collect();
    let (_, integration_proxies) = registry.borrow_mut().subscribe(&[], &integration_need_refs);

    registry
        .borrow_mut()
        .resolve()
        .expect("golden_trace: falha ao resolver StateRegistry — algum `need` não tem provedor");

    let mut state_proxies: Vec<Proxy> = Vec::with_capacity(state_keys.len());
    let mut derivative_proxies: Vec<Proxy> = Vec::with_capacity(state_keys.len());
    for pair in integration_proxies.chunks(2) {
        state_proxies.push(pair[0].clone());
        derivative_proxies.push(pair[1].clone());
    }

    let rk4 = RK4;
    let mut file = std::fs::File::create(&output_path)
        .unwrap_or_else(|err| panic!("golden_trace: falha ao criar '{output_path}': {err}"));
    let mut header_written = false;

    for tick in 0..ticks {
        if state_proxies.is_empty() {
            root.evaluate();
        } else {
            let current: Vec<f64> = state_proxies.iter().map(Proxy::get).collect();
            let next = rk4.step(&current, DT_HOURS, &mut |perturbed: &[f64]| {
                for (proxy, &value) in state_proxies.iter().zip(perturbed) {
                    proxy.set(value);
                }
                root.evaluate();
                derivative_proxies.iter().map(Proxy::get).collect()
            });
            for (proxy, &value) in state_proxies.iter().zip(&next) {
                proxy.set(value);
            }
            root.evaluate();
        }

        registry.borrow_mut().commit();

        let snapshot = registry.borrow().snapshot();
        if !header_written {
            write!(file, "tick").expect("golden_trace: falha ao escrever cabeçalho");
            for slot in &snapshot {
                write!(file, ",{}", slot.key).expect("golden_trace: falha ao escrever cabeçalho");
            }
            writeln!(file).expect("golden_trace: falha ao escrever cabeçalho");
            header_written = true;
        }
        write!(file, "{tick}").expect("golden_trace: falha ao escrever linha");
        for slot in &snapshot {
            write!(file, ",{:.15e}", slot.value).expect("golden_trace: falha ao escrever linha");
        }
        writeln!(file).expect("golden_trace: falha ao escrever linha");
    }

    eprintln!("golden_trace: {ticks} tick(s) gravados em '{output_path}' ({} chave(s) por tick)", state_keys.len());
}
