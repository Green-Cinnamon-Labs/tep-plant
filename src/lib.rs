/** tep/lib.rs

Só o que é específico do TEP mora aqui. Framework de simulação genérico (DynamicModel,
StateRegistry, Integrator, atuador/sensor de 1ª ordem, distúrbio cúbico, e agora também o carregador
genérico de condição inicial — `monjolo::snapshot::Snapshot`) mora em monjolo (crate irmão).

Organização de pastas: units/ (os 5 blocos químicos — Feed/Reactor/Separator/Stripper/
Compressor, cada um dono da própria física via `#[monjolo::tasks]`, issue 10 — nenhuma álgebra
transversal sobra fora deles), diagnostics/ (agregações que cruzam MAIS DE UMA unidade e por isso
não têm dono natural — hoje só `ShutdownDetector`), actuators/ (os 12 atuadores físicos, um arquivo
por atuador, todos `#[actuator(...)]`), sensors/ (idem, `#[sensor(...)]`), controllers/ (idem,
`#[controller(...)]`), disturbance/ (Disturbance + seu estado interno), physics/ (números do TEP
compartilhados — `TepConstants` — as correlações em si moram em `monjolo::chemistry`). Todos os
componentes de `units/`/`diagnostics/` são `#[dynamic_model]`, auto-descobertos via inventory.

NOTA (2026-08-13, branch feat/proc-macro-components): não existe mais `model.rs`/`build_tep()`.
Todo componente da planta (os 7 blocos de units/, os 12 atuadores, os 6 sensores, o controller)
se auto-registra via `inventory::submit!` escondido, gerado pelas macros de `monjolo-macros`;
`Simulation::run()` (bootstrap de `monjolo::simulation`) descobre e monta a árvore de avaliação
inteira sozinho — nada aqui precisa mais listar manualmente o que compõe a planta. O único trabalho
que sobra pra quem monta a aplicação (`src/main.rs`) é dizer onde está o arquivo de condição
inicial (`Simulation::set_config_path`).

NÃO existe mais um `initial_state.rs` próprio do TEP — o struct rígido
(`InitialState`/`StateSections`, um campo por chave do TOML) foi substituído pelo `Snapshot`
genérico do framework: cada `#[dynamic_model]` declara, campo a campo (`#[config(...)]`), só as
chaves que interessam pra ele — do mesmo jeito que já faz com `StateRegistry` (`#[offer(...)]`/
`#[need(...)]`).
*/
pub mod actuators;
pub mod controllers;
pub mod diagnostics;
pub mod disturbance;
pub mod physics;
pub mod sensors;
pub mod units;

/* subsystems.rs original (física de todos os 7 blocos, antes da migração para `#[dynamic_model]`
em units/) preservada em _deprecated_2.rs — não é módulo do crate, só referência.
*/

#[cfg(test)]
mod tests {
    use monjolo::dynamic_model::{Composite, DynamicModel};
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    /** Prova que a cadeia inteira funciona sem nenhuma lista manual: as 5 unidades de units/, o
    diagnóstico de shutdown, os 12 atuadores, os 6 sensores e o controller se descobrem sozinhos via inventory —
    `attach_discovered_components` contra um `Composite` vazio (não sobra nenhum modelo manual pra
    ser o "primeiro filho", diferente de quando `build_tep()` existia) + `resolve()` (sem erro,
    nenhum `need` órfão) + `evaluate()` sem panic (nenhum Proxy lido antes de ser resolvido,
    nenhuma dependência entre blocos fora de ordem — `after` cuida disso). Mesma sequência que
    `Simulation::run()` roda de verdade, sem precisar subir a Thread da planta.
    */
    #[test]
    fn wires_and_evaluates_without_panicking() {
        let registry = StateRegistry::shared();
        let config = Snapshot::from_pairs(&[]);
        let mut root = Composite::new();

        monjolo::attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        root.evaluate();
    }

    /** Investigação do Experimento 21 (spec-tennessee-eastman/experimentos.md). Os Exp 19/20 já
    provaram que `Reactor`/`Separator`/`Compressor` calculam sua PRÓPRIA física corretamente,
    isoladamente, com entradas conhecidas — mas o processo real ainda diverge (Exp 18). Esse teste
    fecha a lacuna entre os dois: monta a planta INTEIRA de verdade (mesmo `attach_discovered_
    components` do teste acima, mas com o estado real de `application.toml`, não vazio) e roda o
    mesmo laço evaluate()+RK4+commit() que `Simulation::spawn_plant_thread` roda de verdade —
    sem thread, sem OPC-UA, sem `sleep` — pra reproduzir em milissegundos a mesma janela (~0.9h
    simuladas) onde o colapso apareceu ao vivo, e achar o primeiro tick onde a trajetória real
    deixa de bater com o que os Exp 19/20 provam ser fisicamente correto.
    */
    #[test]
    fn traces_the_real_multi_unit_trajectory_to_find_where_it_first_diverges() {
        use monjolo::numerical_method::integrator::Integrator;
        use monjolo::numerical_method::rk4::RK4;
        use monjolo::state_registry::Proxy;

        let registry = StateRegistry::shared();
        let config = Snapshot::from_file("application.toml")
            .expect("application.toml deveria existir na raiz do crate e ser um TOML válido");
        let mut root = Composite::new();

        monjolo::attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);

        let state_keys = root.state_keys();
        let mut integration_needs: Vec<String> = Vec::with_capacity(state_keys.len() * 2);
        for key in &state_keys {
            integration_needs.push(key.clone());
            integration_needs.push(format!("{key}.derivative"));
        }
        let integration_need_refs: Vec<&str> = integration_needs.iter().map(String::as_str).collect();
        let (_, integration_proxies) = registry.borrow_mut().subscribe(&[], &integration_need_refs);

        /* Sonda extra, só pra observação — não faz parte da integração. As quatro grandezas que os
        Exp 19/20 já validaram isoladamente pra cada unidade.
        */
        let watch_keys = ["reactor.temperature", "reactor.pressure", "separator.temperature", "compressor.pressure"];
        let (_, watch_proxies) = registry.borrow_mut().subscribe(&[], &watch_keys);

        registry
            .borrow_mut()
            .resolve()
            .expect("todo `need` deveria ter provedor — planta real, não deveria faltar nada");

        let mut state_proxies: Vec<Proxy> = Vec::with_capacity(state_keys.len());
        let mut derivative_proxies: Vec<Proxy> = Vec::with_capacity(state_keys.len());
        for pair in integration_proxies.chunks(2) {
            state_proxies.push(pair[0].clone());
            derivative_proxies.push(pair[1].clone());
        }

        // Priming (mesmo fix de monjolo/simulation.rs::spawn_plant_thread) — sem isso, os 3
        // controladores leriam pressão/nível como 0.0 no tick 0 e comandariam as válvulas pra 0%.
        root.evaluate();
        registry.borrow_mut().commit();
        root.evaluate();

        let integrator = RK4;
        let dt_hours = 1.0 / 3600.0; /* mesmo default de Simulation — 1s simulado por tick */

        /* 3500 ticks ≈ 0.972h simuladas — cobre com folga a janela de ~0.87h onde o colapso
        numérico apareceu ao vivo (Exp 18).
        */
        for tick in 0..3500u32 {
            let current: Vec<f64> = state_proxies.iter().map(Proxy::get).collect();
            let next = integrator.step(&current, dt_hours, &mut |perturbed: &[f64]| {
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
            registry.borrow_mut().commit();

            let t_h = (tick + 1) as f64 * dt_hours;
            let reactor_t = watch_proxies[0].get();
            let reactor_p = watch_proxies[1].get();
            let separator_t = watch_proxies[2].get();
            let compressor_p = watch_proxies[3].get();

            let xmeas_reactor_p = (reactor_p - 760.0) / 760.0 * 101.325;

            if tick < 20 || tick % 50 == 0 {
                println!(
                    "tick={tick:4} t_h={t_h:.4} reactor.T={reactor_t:9.3} xmeas.reactor.P={xmeas_reactor_p:9.3} separator.T={separator_t:9.3} compressor.P={compressor_p:9.1}"
                );
            }

            let blew_up = !reactor_t.is_finite()
                || !reactor_p.is_finite()
                || !separator_t.is_finite()
                || !compressor_p.is_finite()
                || reactor_t.abs() > 1.0e6
                || xmeas_reactor_p.abs() > 1.0e6;
            assert!(
                !blew_up,
                "divergência numérica no tick {tick} (t_h={t_h:.4}h): reactor.T={reactor_t}, \
                xmeas.reactor.P={xmeas_reactor_p}, separator.T={separator_t}, compressor.P={compressor_p}"
            );
        }
    }

    /** Continuação do Experimento 21 (spec-tennessee-eastman/experimentos.md) — mesmo harness do
    teste acima, mas em vez de checar contra faixas "parece razoável", compara tick a tick contra
    `docs/simulations/simulation_log_13.csv`: a gravação REAL do Exp 13 (baseline validado, 20h
    simuladas, mesmo `te_exp3_snapshot.toml`/`application.toml`, física antiga confirmada correta).
    Troca "a física parece estranha" por "no tick N, reactor.temperature já diverge X graus do
    valor exato que a planta validada tinha nesse mesmo t_h".
    */
    #[test]
    fn diverges_from_the_validated_baseline_csv_early_not_gradually() {
        use monjolo::numerical_method::integrator::Integrator;
        use monjolo::numerical_method::rk4::RK4;
        use monjolo::state_registry::Proxy;

        /* CSV: `t_h,XMEAS(1),XMEAS(2),...` — XMEAS(7) é a 8ª coluna (índice 7), XMEAS(9) a 10ª
        (índice 9), 0-indexado a partir de `t_h` (índice 0).
        */
        let csv = std::fs::read_to_string("docs/simulations/simulation_log_13.csv")
            .expect("docs/simulations/simulation_log_13.csv deveria existir (cópia do Exp 13)");
        let reference: Vec<(f64, f64, f64)> = csv
            .lines()
            .skip(1) /* cabeçalho */
            .map(|line| {
                let fields: Vec<&str> = line.split(',').collect();
                let t_h: f64 = fields[0].parse().expect("t_h deveria ser um número");
                let xmeas7: f64 = fields[7].parse().expect("XMEAS(7) deveria ser um número");
                let xmeas9: f64 = fields[9].parse().expect("XMEAS(9) deveria ser um número");
                (t_h, xmeas7, xmeas9)
            })
            .collect();

        let registry = StateRegistry::shared();
        let config = Snapshot::from_file("application.toml")
            .expect("application.toml deveria existir na raiz do crate e ser um TOML válido");
        let mut root = Composite::new();
        monjolo::attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);

        let state_keys = root.state_keys();
        let mut integration_needs: Vec<String> = Vec::with_capacity(state_keys.len() * 2);
        for key in &state_keys {
            integration_needs.push(key.clone());
            integration_needs.push(format!("{key}.derivative"));
        }
        let integration_need_refs: Vec<&str> = integration_needs.iter().map(String::as_str).collect();
        let (_, integration_proxies) = registry.borrow_mut().subscribe(&[], &integration_need_refs);

        let watch_keys = ["reactor.temperature", "reactor.pressure"];
        let (_, watch_proxies) = registry.borrow_mut().subscribe(&[], &watch_keys);

        registry.borrow_mut().resolve().expect("todo `need` deveria ter provedor");

        let mut state_proxies: Vec<Proxy> = Vec::with_capacity(state_keys.len());
        let mut derivative_proxies: Vec<Proxy> = Vec::with_capacity(state_keys.len());
        for pair in integration_proxies.chunks(2) {
            state_proxies.push(pair[0].clone());
            derivative_proxies.push(pair[1].clone());
        }

        // Priming (mesmo fix de monjolo/simulation.rs::spawn_plant_thread).
        root.evaluate();
        registry.borrow_mut().commit();
        root.evaluate();

        let integrator = RK4;
        let dt_hours = 1.0 / 3600.0;

        /* Limiar de divergência: 2°C ou 20 kPa — bem acima do ruído/drift lento documentado
        (Exp 10: +1 kPa/h, +0.2%/h de nível), então só dispara numa divergência real, não em
        flutuação normal.
        */
        const TEMPERATURE_TOLERANCE: f64 = 2.0;
        const PRESSURE_TOLERANCE: f64 = 20.0;

        let mut csv_idx = 0usize;
        let mut first_divergence: Option<(u32, f64, f64, f64, f64, f64)> = None;

        for tick in 0..1000u32 {
            let current: Vec<f64> = state_proxies.iter().map(Proxy::get).collect();
            let next = integrator.step(&current, dt_hours, &mut |perturbed: &[f64]| {
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
            registry.borrow_mut().commit();

            let t_h = (tick + 1) as f64 * dt_hours;
            let reactor_t = watch_proxies[0].get();
            let xmeas_reactor_p = (watch_proxies[1].get() - 760.0) / 760.0 * 101.325;

            /* Anda no CSV até achar a linha com t_h mais próximo do tick atual — os dois grids não
            coincidem exatamente (CSV a cada 0.001h, aqui a cada 1/3600h), mas ambos são
            monotônicos, então um ponteiro avançando é suficiente.
            */
            while csv_idx + 1 < reference.len() && reference[csv_idx + 1].0 <= t_h {
                csv_idx += 1;
            }
            let (ref_t_h, ref_p, ref_t) = reference[csv_idx];

            let temperature_diff = (reactor_t - ref_t).abs();
            let pressure_diff = (xmeas_reactor_p - ref_p).abs();

            if first_divergence.is_none() && (temperature_diff > TEMPERATURE_TOLERANCE || pressure_diff > PRESSURE_TOLERANCE) {
                first_divergence = Some((tick, t_h, reactor_t, ref_t, xmeas_reactor_p, ref_p));
            }

            if tick < 20 || tick % 20 == 0 {
                println!(
                    "tick={tick:4} t_h={t_h:.4} (csv t_h={ref_t_h:.4}) reactor.T={reactor_t:8.3} (ref {ref_t:8.3}, Δ{temperature_diff:6.3}) xmeas.P={xmeas_reactor_p:9.3} (ref {ref_p:9.3}, Δ{pressure_diff:7.3})"
                );
            }

            if first_divergence.is_some() {
                break;
            }
        }

        if let Some((tick, t_h, reactor_t, ref_t, reactor_p, ref_p)) = first_divergence {
            println!(
                "\nPRIMEIRA DIVERGÊNCIA: tick={tick} (t_h={t_h:.4}h) — reactor.temperature={reactor_t:.3}°C \
                (referência: {ref_t:.3}°C, Δ={:.3}°C), xmeas.reactor.pressure={reactor_p:.3} kPa \
                (referência: {ref_p:.3} kPa, Δ={:.3} kPa)",
                (reactor_t - ref_t).abs(),
                (reactor_p - ref_p).abs(),
            );
        }

        assert!(
            first_divergence.is_none(),
            "divergiu da trajetória validada — ver PRIMEIRA DIVERGÊNCIA acima (rode com --nocapture)"
        );
    }
}
