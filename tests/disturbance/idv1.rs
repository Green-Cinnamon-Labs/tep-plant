/* Testes de `disturbance::idv1`. */

use monjolo::snapshot::Snapshot;
use monjolo::state_registry::{Proxy, StateRegistry};
use tennessee_eastman_process::disturbance::idv1::Idv1;

/* Valor nominal de `FEED_AC_COMPOSITION` em `units/feed.rs` (`pub(crate)`, não visível daqui —
mesma convenção de `tests/units/feed.rs`: literal, não o const privado do módulo). */
const FEED_AC_COMPOSITION: [f64; 8] = [0.4850, 0.0050, 0.5100, 0.0, 0.0, 0.0, 0.0, 0.0];

/** Prova a interceptação isolada, sem construir o `Feed` real nem a planta inteira: constrói só
`Idv1` diretamente (`pub struct`/`pub fn new`) e oferta `flows.stream4_composition.*` à mão. A
interceptação acontece no `offer::` agora (issue spec-tennessee-eastman#73) — por isso o teste
REESCREVE a composição (`Proxy::set_group`, simulando o que `Feed::ac_feed_composition` faria a
cada tick de verdade) depois de cada mudança no comando liga/desliga, em vez de só girar o
`Actuator` e reler: o valor gravado só reflete o distúrbio na escrita SEGUINTE a ele.
*/
#[test]
fn idv1_step_shifts_a_into_c_leaving_b_untouched_only_while_active() {
    let registry = StateRegistry::shared();

    let keys = ["a", "b", "c", "d", "e", "f", "g", "h"].map(|c| format!("flows.stream4_composition.{c}"));
    let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let (offered, _) = registry.borrow_mut().subscribe(&key_refs, &[]);

    Idv1::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

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
    assert_eq!(needed[0].get(), FEED_AC_COMPOSITION[0], "IDV1 desligado: A nominal");
    assert_eq!(needed[1].get(), FEED_AC_COMPOSITION[1], "IDV1 desligado: B nominal");
    assert_eq!(needed[2].get(), FEED_AC_COMPOSITION[2], "IDV1 desligado: C nominal");

    let idv1 = registry
        .borrow()
        .actuator("disturbance.idv1")
        .expect("Idv1 deveria ter se catalogado sozinha como Actuator");
    idv1.write(1.0);
    Proxy::set_group(&offered, &FEED_AC_COMPOSITION);
    assert!((needed[0].get() - (FEED_AC_COMPOSITION[0] - 0.03)).abs() < 1e-12, "IDV1 ligado: A cai 0.03");
    assert_eq!(needed[1].get(), FEED_AC_COMPOSITION[1], "IDV1 ligado: B continua intocado");
    assert!((needed[2].get() - (FEED_AC_COMPOSITION[2] + 0.03)).abs() < 1e-12, "IDV1 ligado: C absorve a diferença");

    idv1.write(0.0);
    Proxy::set_group(&offered, &FEED_AC_COMPOSITION);
    assert_eq!(needed[0].get(), FEED_AC_COMPOSITION[0], "IDV1 desligado de novo: volta ao nominal");
}
