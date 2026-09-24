/* tep/disturbance/idv1.rs */

/** IDV(1) — A/C feed ratio, step (Table 8, Downs & Vogel 1993; `teprob.f`:
`XST(1,4) -= IDV(1)*0.03`, C absorve a diferença via `XST(3,4) = 1 - XST(1,4) - XST(2,4)`, B
intocado). Primeira migração pro padrão novo de distúrbio (ver `docs/05-disturbios.md`).

`Disturbances::idv1` é o método inteiro: lê a composição NOMINAL publicada por `units::feed::Feed`
(`flows.stream4_composition_nominal.*`, que não sabe nada disto) e reoferece sob a chave PÚBLICA que
`units::stripper` de fato consome (`flows.stream4_composition.*`, que também não sabe nada disto) —
alterada só quando ligado. `#[disturbance(key = "disturbance.idv1")]` (atributo do MÉTODO, junto de
`#[need]`/`#[offer]` — não do `impl` inteiro, já que `Disturbances` pode ganhar outros métodos/IDVs
depois, cada um com sua própria chave) faz o método INTEIRO virar também o comando externo
liga/desliga (`active` chega como o primeiro parâmetro, injetado pela macro — não é um `#[need]`
escrito à mão): a tarefa gerada implementa `Actuator` e se cataloga sob `"disturbance.idv1"`
sozinha, ganhando escrita/exposição OPC-UA de graça, sem nenhum `inventory::submit!` separado pra
manter em sincronia.

NOTA: `tep-plant::disturbance::Disturbance` (`mod.rs`/`state.rs`, o struct antigo de 20 flags) ainda
existe, mas é código morto — nunca chamado desde a migração pro scheduler de dataflow topológico
(issue #10) removeu `build_tep()`. Este arquivo NÃO o reaproveita; IDV(1) é migrado do zero pro
padrão novo. Os outros 19 IDVs migram um a um, em conversas separadas (ver `docs/05-disturbios.md`).
*/

#[monjolo::dynamic_model(tasks)]
pub struct Disturbances {}

#[monjolo::tasks]
impl Disturbances {


    
    #[disturbance(key = "disturbance.idv1")]
    #[need(prefix = "flows.stream4_composition_nominal", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    #[offer(prefix = "flows.stream4_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    fn idv1(&self, active: f64, nominal: [f64; 8]) -> [f64; 8] {
        let mut composition = nominal;
        if active != 0.0 {
            composition[0] -= 0.03;
            composition[2] = 1.0 - composition[0] - composition[1];
        }
        composition
    }
}

#[cfg(test)]
mod tests {
    use crate::units::feed::FEED_AC_COMPOSITION;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    /** Prova o fluxo ponta-a-ponta: `Feed` publica o nominal, `Disturbances::idv1` intercepta e é
    ao mesmo tempo o comando externo, `Stripper` (e qualquer outro `#[need]` na chave pública) só vê
    o resultado — desligado, igual ao nominal; ligado, A cai 0.03/C absorve/B intocado; desligado de
    novo, volta ao nominal.
    */
    #[test]
    fn idv1_step_shifts_a_into_c_leaving_b_untouched_only_while_active() {
        let registry = StateRegistry::shared();
        let config = Snapshot::from_pairs(&[]);
        let mut root = monjolo::dynamic_model::Composite::new();
        monjolo::attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        let (_, needed) = registry.borrow_mut().subscribe(
            &[],
            &[
                "flows.stream4_composition.a",
                "flows.stream4_composition.b",
                "flows.stream4_composition.c",
            ],
        );
        registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

        root.evaluate();
        assert_eq!(needed[0].get(), FEED_AC_COMPOSITION[0], "IDV1 desligado: A nominal");
        assert_eq!(needed[1].get(), FEED_AC_COMPOSITION[1], "IDV1 desligado: B nominal");
        assert_eq!(needed[2].get(), FEED_AC_COMPOSITION[2], "IDV1 desligado: C nominal");

        let idv1 = registry
            .borrow()
            .actuator("disturbance.idv1")
            .expect("Disturbances::idv1 deveria ter se catalogado sozinha como Actuator");
        idv1.write(1.0);
        root.evaluate();
        assert!((needed[0].get() - (FEED_AC_COMPOSITION[0] - 0.03)).abs() < 1e-12, "IDV1 ligado: A cai 0.03");
        assert_eq!(needed[1].get(), FEED_AC_COMPOSITION[1], "IDV1 ligado: B continua intocado");
        assert!((needed[2].get() - (FEED_AC_COMPOSITION[2] + 0.03)).abs() < 1e-12, "IDV1 ligado: C absorve a diferença");

        idv1.write(0.0);
        root.evaluate();
        assert_eq!(needed[0].get(), FEED_AC_COMPOSITION[0], "IDV1 desligado de novo: volta ao nominal");
    }
}
