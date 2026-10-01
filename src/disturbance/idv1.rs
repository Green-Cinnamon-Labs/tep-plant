/* tep/disturbance/idv1.rs */

/** IDV(1) — A/C feed ratio, step (Table 8, Downs & Vogel 1993; `teprob.f`:
`XST(1,4) -= IDV(1)*0.03`, C absorve a diferença via `XST(3,4) = 1 - XST(1,4) - XST(2,4)`, B
intocado). Migrado pro mecanismo de INTERCEPTAÇÃO (ver `docs/05-disturbios.md`,
`monjolo-macros/tasks.rs::build_disturbance_interceptor`) — não existe mais uma chave "_nominal"
paralela: `Disturbances::idv1` intercepta DIRETO a chave pública que `units::feed::Feed` publica e
`units::stripper` consome (`flows.stream4_composition.*`), sem que nenhum dos dois saiba que isto
existe.

`#[disturbance(key = "disturbance.idv1", intercepts = "flows.stream4_composition", components =
[...])]` cataloga o próprio método como o comando externo liga/desliga (`Actuator` sob essa chave,
exposição OPC-UA automática, igual a qualquer outro atuador). `idv1` é uma função PURA, sem `&self`
— ela vira um `fn(f64, &[f64]) -> Vec<f64>` sem nenhuma captura, chamado de dentro de `Proxy::get()`
pra quem pede `need::flows__stream4_composition::<Mixture>` (ex.: `Stripper::flash_split`), sem
acesso ao `StateRegistry` nem a qualquer instância.
*/

#[monjolo::dynamic_model(tasks)]
pub struct Disturbances {}

#[monjolo::tasks]
impl Disturbances {
    #[disturbance(
        key = "disturbance.idv1",
        intercepts = "flows.stream4_composition",
        components = ["a", "b", "c", "d", "e", "f", "g", "h"]
    )]
    fn idv1(active: f64, raw: &[f64]) -> Vec<f64> {
        let mut composition = raw.to_vec();
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

    /** Prova o fluxo ponta-a-ponta: `Feed` publica a composição direto na chave pública,
    `Disturbances::idv1` intercepta e é ao mesmo tempo o comando externo, `Stripper` (e qualquer
    outro `need::` na chave pública) só vê o resultado — desligado, igual ao nominal; ligado, A cai
    0.03/C absorve/B intocado; desligado de novo, volta ao nominal.
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
