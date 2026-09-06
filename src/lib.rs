/** tep/lib.rs

Só o que é específico do TEP mora aqui. Framework de simulação genérico (DynamicModel,
StateRegistry, Integrator, atuador/sensor de 1ª ordem, distúrbio cúbico, e agora também o carregador
genérico de condição inicial — `monjolo::snapshot::Snapshot`) mora em monjolo (crate irmão).

Organização de pastas: dynamics/ (os 5 blocos químicos — Feed/Reactor/Separator/Stripper/
Compressor, cada um dono da própria física via `#[monjolo::tasks]`, issue 10 — nenhuma álgebra
transversal sobra fora deles), diagnostics/ (agregações que cruzam MAIS DE UMA unidade e por isso
não têm dono natural — hoje só `ShutdownDetector`), actuators/ (os 12 atuadores físicos, um arquivo
por atuador, todos `#[actuator(...)]`), sensors/ (idem, `#[sensor(...)]`), controllers/ (idem,
`#[controller(...)]`), disturbance/ (Disturbance + seu estado interno), physics/ (números do TEP
compartilhados — `TepConstants` — as correlações em si moram em `monjolo::chemistry`). Todos os
componentes de `dynamics/`/`diagnostics/` são `#[dynamic_model]`, auto-descobertos via inventory.

NOTA (2026-08-13, branch feat/proc-macro-components): não existe mais `model.rs`/`build_tep()`.
Todo componente da planta (os 7 blocos de dynamics/, os 12 atuadores, os 6 sensores, o controller)
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
pub mod dynamics;
pub mod physics;
pub mod sensors;

/* subsystems.rs original (física de todos os 7 blocos, antes da migração para `#[dynamic_model]`
em dynamics/) preservada em _deprecated_2.rs — não é módulo do crate, só referência.
*/

#[cfg(test)]
mod tests {
    use monjolo::dynamic_model::{Composite, DynamicModel};
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    /** Prova que a cadeia inteira funciona sem nenhuma lista manual: as 5 unidades de dynamics/, o
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
}
