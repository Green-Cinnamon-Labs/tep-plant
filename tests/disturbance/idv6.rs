/* Testes de `disturbance::idv6`. */

use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::disturbance::idv6::Idv6;

/** Prova a interceptação isolada do IDV(6): desligado, a vazão passa reta; ligado, zera —
exatamente `FTM(3) = VPOS(3)*(1-IDV(6))*VRNG(3)/100` do `teprob.f` com `IDV(6)=1`. Interceptação
no `offer::` (issue spec-tennessee-eastman#73) — o teste reescreve a chave depois de cada mudança
no comando liga/desliga, simulando o que `Feed::a_feed_flow` faria a cada tick de verdade.
*/
#[test]
fn idv6_step_zeroes_the_a_feed_flow_only_while_active() {
    let registry = StateRegistry::shared();

    let (offered, _) = registry.borrow_mut().subscribe(&["flows.stream_flow.1"], &[]);

    Idv6::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

    let (_, needed) = registry.borrow_mut().subscribe(&[], &["flows.stream_flow.1"]);
    registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");

    offered[0].set(24.64);
    assert_eq!(needed[0].get(), 24.64, "IDV6 desligado: vazão nominal");

    let idv6 = registry
        .borrow()
        .actuator("disturbance.idv6")
        .expect("Idv6 deveria ter se catalogado sozinha como Actuator");
    idv6.write(1.0);
    offered[0].set(24.64);
    assert_eq!(needed[0].get(), 0.0, "IDV6 ligado: vazão zerada (válvula fechada)");

    idv6.write(0.0);
    offered[0].set(24.64);
    assert_eq!(needed[0].get(), 24.64, "IDV6 desligado de novo: volta à vazão nominal");
}
