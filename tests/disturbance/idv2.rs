/* Testes de `disturbance::idv2`. */

use monjolo::snapshot::Snapshot;
use monjolo::state_registry::{Proxy, StateRegistry};
use tennessee_eastman_process::disturbance::idv2::Idv2;

/* Valor nominal de `FEED_AC_COMPOSITION` em `units/feed.rs` (`pub(crate)`, não visível daqui —
mesma convenção de `tests/units/feed.rs`: literal, não o const privado do módulo). */
const FEED_AC_COMPOSITION: [f64; 8] = [0.4850, 0.0050, 0.5100, 0.0, 0.0, 0.0, 0.0, 0.0];

/** Prova a interceptação isolada do IDV(2) — mesmo molde do teste de `idv1.rs`. A interceptação
acontece no `offer::` (issue spec-tennessee-eastman#73) — por isso o teste REESCREVE a composição
(`Proxy::set_group`) a cada mudança do comando liga/desliga, simulando o que `Feed::
ac_feed_composition` faria a cada tick de verdade. Desligado: valor cru. Ligado: A cai 2.43719e-3,
B sobe 0.005, C absorve a diferença (`1 - A - B`) — exatamente a fórmula de `teprob.f` com IDV(1)
assumido desligado (ver nota em `src/disturbance/idv2.rs`).
*/
#[test]
fn idv2_step_raises_b_and_lowers_a_leaving_c_to_absorb_the_difference_only_while_active() {
    let registry = StateRegistry::shared();

    let keys = ["a", "b", "c", "d", "e", "f", "g", "h"].map(|c| format!("flows.stream4_composition.{c}"));
    let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let (offered, _) = registry.borrow_mut().subscribe(&key_refs, &[]);

    Idv2::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

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

    Proxy::set_group(&offered, &FEED_AC_COMPOSITION);
    assert_eq!(needed[0].get(), FEED_AC_COMPOSITION[0], "IDV2 desligado: A nominal");
    assert_eq!(needed[1].get(), FEED_AC_COMPOSITION[1], "IDV2 desligado: B nominal");
    assert_eq!(needed[2].get(), FEED_AC_COMPOSITION[2], "IDV2 desligado: C nominal");

    let idv2 = registry
        .borrow()
        .actuator("disturbance.idv2")
        .expect("Idv2 deveria ter se catalogado sozinha como Actuator");
    idv2.write(1.0);
    Proxy::set_group(&offered, &FEED_AC_COMPOSITION);
    assert!(
        (needed[0].get() - (FEED_AC_COMPOSITION[0] - 2.43719e-3)).abs() < 1e-12,
        "IDV2 ligado: A cai 2.43719e-3",
    );
    assert!(
        (needed[1].get() - (FEED_AC_COMPOSITION[1] + 0.005)).abs() < 1e-12,
        "IDV2 ligado: B sobe 0.005",
    );
    assert!(
        (needed[2].get() - (1.0 - needed[0].get() - needed[1].get())).abs() < 1e-12,
        "IDV2 ligado: C absorve a diferença (1 - A - B)",
    );

    idv2.write(0.0);
    Proxy::set_group(&offered, &FEED_AC_COMPOSITION);
    assert_eq!(needed[0].get(), FEED_AC_COMPOSITION[0], "IDV2 desligado de novo: volta ao nominal");
}
