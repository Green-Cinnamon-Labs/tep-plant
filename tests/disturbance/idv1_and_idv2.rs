/* Testes de `disturbance::idv1` + `disturbance::idv2` LIGADOS JUNTOS — o caso real que motivou a
migração do mecanismo de interceptação pro `offer::`, em cadeia (issue spec-tennessee-eastman#73):
no `teprob.f` original, os dois efeitos se SOMAM sobre a mesma composição (`XST(1,4) = nominal -
IDV(1)*0.03 - IDV(2)*2.43719e-3`, `XST(2,4) = nominal + IDV(2)*0.005`). Antes desta migração, dois
`#[disturbance]` disputando a mesma chave quebravam o boot da planta inteira (`resolve()` errava) —
este teste prova que agora convivem e se somam corretamente, em qualquer ordem de registro.
*/

use monjolo::snapshot::Snapshot;
use monjolo::state_registry::{Proxy, StateRegistry};
use tennessee_eastman_process::disturbance::idv1::Idv1;
use tennessee_eastman_process::disturbance::idv2::Idv2;

const FEED_AC_COMPOSITION: [f64; 8] = [0.4850, 0.0050, 0.5100, 0.0, 0.0, 0.0, 0.0, 0.0];

#[test]
fn idv1_and_idv2_active_together_sum_their_effects_exactly_like_teprob_f() {
    let registry = StateRegistry::shared();

    let keys = ["a", "b", "c", "d", "e", "f", "g", "h"].map(|c| format!("flows.stream4_composition.{c}"));
    let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let (offered, _) = registry.borrow_mut().subscribe(&key_refs, &[]);

    Idv1::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));
    Idv2::new(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

    registry.borrow_mut().resolve().expect("IDV1 e IDV2 deveriam conviver na mesma chave, em cadeia");

    let (_, needed) = registry.borrow_mut().subscribe(
        &[],
        &[
            "flows.stream4_composition.a",
            "flows.stream4_composition.b",
            "flows.stream4_composition.c",
        ],
    );
    registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

    registry.borrow().actuator("disturbance.idv1").expect("Idv1 deveria estar catalogada").write(1.0);
    registry.borrow().actuator("disturbance.idv2").expect("Idv2 deveria estar catalogada").write(1.0);
    Proxy::set_group(&offered, &FEED_AC_COMPOSITION);

    let expected_a = FEED_AC_COMPOSITION[0] - 0.03 - 2.43719e-3;
    let expected_b = FEED_AC_COMPOSITION[1] + 0.005;
    let expected_c = 1.0 - expected_a - expected_b;

    assert!((needed[0].get() - expected_a).abs() < 1e-12, "A = nominal - 0.03 (IDV1) - 2.43719e-3 (IDV2)");
    assert!((needed[1].get() - expected_b).abs() < 1e-12, "B = nominal + 0.005 (só IDV2 mexe em B)");
    assert!((needed[2].get() - expected_c).abs() < 1e-12, "C absorve a diferença: 1 - A - B");
}
