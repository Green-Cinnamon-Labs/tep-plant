/* Testes de `disturbance::idv7`. */

use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::disturbance::idv7::Idv7;

/** Prova a interceptação isolada do IDV(7): desligado, a vazão passa reta; ligado, cai 20% —
exatamente `FTM(4) = VPOS(4)*(1-0.2·IDV(7))*VRNG(4)/100 + 1e-10` do `teprob.f` com `IDV(7)=1`.
Interceptação no `offer::` (issue spec-tennessee-eastman#73) — o teste reescreve a chave depois
de cada mudança no comando liga/desliga, simulando o que `Feed::ac_feed_flow` faria a cada tick.
*/
#[test]
fn idv7_step_cuts_the_ac_feed_flow_by_twenty_percent_only_while_active() {
    let registry = StateRegistry::shared();

    let (offered, _) = registry.borrow_mut().subscribe(&["flows.stream_flow.4"], &[]);

    Idv7::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

    let (_, needed) = registry.borrow_mut().subscribe(&[], &["flows.stream_flow.4"]);
    registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");

    let nominal = 61.30 * 1500.0 / 100.0 + 1e-10;
    offered[0].set(nominal);
    assert_eq!(needed[0].get(), nominal, "IDV7 desligado: vazão nominal");

    let idv7 = registry
        .borrow()
        .actuator("disturbance.idv7")
        .expect("Idv7 deveria ter se catalogado sozinha como Actuator");
    idv7.write(1.0);
    offered[0].set(nominal);
    let expected = (nominal - 1e-10) * 0.8 + 1e-10;
    assert!((needed[0].get() - expected).abs() < 1e-15, "IDV7 ligado: vazão cai 20%");

    idv7.write(0.0);
    offered[0].set(nominal);
    assert_eq!(needed[0].get(), nominal, "IDV7 desligado de novo: volta à vazão nominal");
}
