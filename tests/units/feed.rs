/* Testes de `units::feed`. Os valores esperados são literais (faixa das válvulas e composições nominais de TEINIT), não os `const` privados do módulo. */

use monjolo::dynamic_model::{Composite, DynamicModel};
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::physics::constants::TepConstants;
use tennessee_eastman_process::units::feed::Feed;

#[test]
fn linear_valve_flows_match_hand_computed_values() {
    /* Não oferece `valve.feed_*.position` na mão: `attach_discovered_components` também
    descobre os atuadores REAIS (`FeedD`/`FeedE`/`FeedA`/`FeedAC`, `src/actuators/`), que já
    ofertam essas mesmas chaves sozinhos — ofertar de novo aqui colidiria (StateRegistry
    sobrescreveria o índice silenciosamente, deixando o valor semeado na mão órfão). Em vez
    disso, semeia pelo MESMO caminho de config que os atuadores reais usam em produção
    (`#[actuator(key = ..., config = "state.valves.d_feed")]` etc.) — testa a fiação de ponta a
    ponta (atuador → Feed), não só Feed isolado.
    */
    let _keep_linked: fn(&mut StateRegistry, &Snapshot) -> std::rc::Rc<Feed> = Feed::new;

    let registry = StateRegistry::shared();
    let config = Snapshot::from_pairs(&[
        ("state.valves.d_feed", 50.0),
        ("state.valves.e_feed", 25.0),
        ("state.valves.a_feed", 10.0),
        ("state.valves.a_c_feed", 5.0),
    ]);
    let mut root = Composite::new();
    monjolo::attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);
    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

    root.evaluate();

    let (_, needed) = registry.borrow_mut().subscribe(
        &[],
        &["flows.stream_flow.0", "flows.stream_flow.1", "flows.stream_flow.2", "flows.stream_flow.3"],
    );
    registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

    assert_eq!(needed[0].get(), 50.0 * 400.0 / 100.0, "D feed: posição*range/100");
    assert_eq!(needed[1].get(), 25.0 * 400.0 / 100.0, "E feed: posição*range/100");
    assert_eq!(needed[2].get(), 10.0 * 100.0 / 100.0, "A feed: posição*range/100");
    assert_eq!(needed[3].get(), 5.0 * 1500.0 / 100.0 + 1e-10, "A&C feed: posição*range/100 + epsilon");
}

#[test]
fn mol_weights_match_hand_computed_values_from_fixed_feed_composition() {
    let _keep_linked: fn(&mut StateRegistry, &Snapshot) -> std::rc::Rc<Feed> = Feed::new;

    let registry = StateRegistry::shared();
    let config = Snapshot::from_pairs(&[]);
    let mut root = Composite::new();
    monjolo::attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);
    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

    root.evaluate();

    let (_, needed) = registry
        .borrow_mut()
        .subscribe(&[], &["flows.d_feed_mol_weight", "flows.e_feed_mol_weight"]);
    registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

    let constants = TepConstants::new();
    let d_composition = [0.0, 0.0001, 0.0, 0.9999, 0.0, 0.0, 0.0, 0.0];
    let e_composition = [0.0, 0.0, 0.0, 0.0, 0.9999, 0.0001, 0.0, 0.0];
    let mol_weight = |z: &[f64; 8]| -> f64 { (0..8).map(|i| z[i] * constants.xmw[i]).sum() };
    assert_eq!(needed[0].get(), mol_weight(&d_composition));
    assert_eq!(needed[1].get(), mol_weight(&e_composition));
}
