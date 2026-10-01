/* Testes de `disturbance::idv3`. */

use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::disturbance::idv3::Idv3;

/* Valor nominal de `FEED_TEMPERATURE` em `units/feed.rs` (`pub(crate)`, não visível daqui — mesma
convenção de `tests/units/feed.rs`: literal, não o const privado do módulo). */
const FEED_TEMPERATURE: f64 = 45.0;

/** Prova a interceptação isolada do IDV(3), escalar (sem `components`/`Mixture`): desligado, o
valor cru; ligado, +5°C — exatamente `TST(1) += IDV(3)*5.0` do `teprob.f`. Interceptação acontece
no `offer::` (issue spec-tennessee-eastman#73) — o teste reescreve a chave (`offered[0].set(...)`)
depois de cada mudança no comando liga/desliga, simulando o que `Feed::d_feed_temperature` faria a
cada tick de verdade.
*/
#[test]
fn idv3_step_raises_d_feed_temperature_by_five_degrees_only_while_active() {
    let registry = StateRegistry::shared();

    let (offered, _) = registry.borrow_mut().subscribe(&["flows.d_feed_temperature"], &[]);

    Idv3::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

    let (_, needed) = registry.borrow_mut().subscribe(&[], &["flows.d_feed_temperature"]);
    registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");

    offered[0].set(FEED_TEMPERATURE);
    assert_eq!(needed[0].get(), FEED_TEMPERATURE, "IDV3 desligado: temperatura nominal");

    let idv3 = registry
        .borrow()
        .actuator("disturbance.idv3")
        .expect("Idv3 deveria ter se catalogado sozinha como Actuator");
    idv3.write(1.0);
    offered[0].set(FEED_TEMPERATURE);
    assert_eq!(needed[0].get(), FEED_TEMPERATURE + 5.0, "IDV3 ligado: +5°C");

    idv3.write(0.0);
    offered[0].set(FEED_TEMPERATURE);
    assert_eq!(needed[0].get(), FEED_TEMPERATURE, "IDV3 desligado de novo: volta ao nominal");
}
