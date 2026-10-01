/* Testes de `disturbance::idv1`. */

use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::disturbance::Disturbances;

/* Valor nominal de `FEED_AC_COMPOSITION` em `units/feed.rs` (`pub(crate)`, não visível daqui —
mesma convenção de `tests/units/feed.rs`: literal, não o const privado do módulo). */
const FEED_AC_COMPOSITION: [f64; 8] = [0.4850, 0.0050, 0.5100, 0.0, 0.0, 0.0, 0.0, 0.0];

/** Prova a interceptação isolada, sem construir o `Feed` real nem a planta inteira: semeia
`flows.stream4_composition.*` direto (é só uma chave já ofertada, não importa quem a ofertou de
verdade) e constrói só a tarefa `Disturbances::idv1` pelo nome — igual `Harness::task` faz em
`tests/units/`, só que sem depender daquele helper (crate de teste separado). Prova: desligado,
`need::` enxerga o valor cru; ligado, A cai 0.03/C absorve/B intocado — sem nenhum `evaluate()`,
porque a troca acontece dentro do `Proxy`, resolvida uma vez em `resolve()`, nunca por tick.
*/
#[test]
fn idv1_step_shifts_a_into_c_leaving_b_untouched_only_while_active() {
    /* Força o linker a manter o objeto compilado de `idv1.rs` neste binário de teste separado —
    sem isso, nada aqui referencia `Disturbances::idv1` e o `inventory::submit!` que a macro gera
    ao lado dele nunca entra no link (mesmo truque de `tests/units/feed.rs` com `Feed::new`). */
    let _keep_linked: fn(f64, &[f64]) -> Vec<f64> = Disturbances::idv1;

    let registry = StateRegistry::shared();

    let keys = ["a", "b", "c", "d", "e", "f", "g", "h"].map(|c| format!("flows.stream4_composition.{c}"));
    let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let (offered, _) = registry.borrow_mut().subscribe(&key_refs, &[]);
    for (proxy, value) in offered.iter().zip(FEED_AC_COMPOSITION.iter()) {
        proxy.set(*value);
    }

    let descriptor = monjolo::inventory::iter::<monjolo::ComponentDescriptor>()
        .find(|d| d.name == "Disturbances::idv1")
        .expect("Disturbances::idv1 deveria estar registrada no inventory");
    (descriptor.construct)(&mut registry.borrow_mut(), &Snapshot::from_pairs(&[]));

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

    assert_eq!(needed[0].get(), FEED_AC_COMPOSITION[0], "IDV1 desligado: A nominal");
    assert_eq!(needed[1].get(), FEED_AC_COMPOSITION[1], "IDV1 desligado: B nominal");
    assert_eq!(needed[2].get(), FEED_AC_COMPOSITION[2], "IDV1 desligado: C nominal");

    let idv1 = registry
        .borrow()
        .actuator("disturbance.idv1")
        .expect("Disturbances::idv1 deveria ter se catalogado sozinha como Actuator");
    idv1.write(1.0);
    assert!((needed[0].get() - (FEED_AC_COMPOSITION[0] - 0.03)).abs() < 1e-12, "IDV1 ligado: A cai 0.03");
    assert_eq!(needed[1].get(), FEED_AC_COMPOSITION[1], "IDV1 ligado: B continua intocado");
    assert!((needed[2].get() - (FEED_AC_COMPOSITION[2] + 0.03)).abs() < 1e-12, "IDV1 ligado: C absorve a diferença");

    idv1.write(0.0);
    assert_eq!(needed[0].get(), FEED_AC_COMPOSITION[0], "IDV1 desligado de novo: volta ao nominal");
}
