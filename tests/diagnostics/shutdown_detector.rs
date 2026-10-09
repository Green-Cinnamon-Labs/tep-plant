/* Testes de `diagnostics::shutdown_detector`. */

use monjolo::dynamic_model::DynamicModel;
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::diagnostics::shutdown_detector::ShutdownDetector;

fn seed_all(registry: &mut StateRegistry) -> Vec<monjolo::state_registry::Proxy> {
    let (offered, _) = registry.subscribe(
        &[
            "xmeas.reactor.pressure",
            "reactor.liquid_volume",
            "reactor.temperature",
            "separator.liquid_volume",
            "stripper.liquid_volume",
        ],
        &[],
    );
    offered
}

#[test]
fn shutdown_detected_flags_reactor_pressure_above_3000_kpa() {
    let registry = StateRegistry::shared();
    let offered = seed_all(&mut registry.borrow_mut());
    offered[0].set(3000.1); /* xmeas.reactor.pressure já em kPa */

    let config = Snapshot::from_pairs(&[]);
    let detector = ShutdownDetector::new(&mut registry.borrow_mut(), &config);
    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

    detector.evaluate();

    let (_, needed) = registry.borrow_mut().subscribe(&[], &["status.shutdown_detected"]);
    registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
    assert_eq!(needed[0].get(), 1.0);
}

#[test]
fn shutdown_not_detected_within_normal_operating_ranges() {
    let registry = StateRegistry::shared();
    let offered = seed_all(&mut registry.borrow_mut());
    offered[0].set(2110.6); /* xmeas.reactor.pressure — dentro do normal (docs/07-controle.md) */
    offered[1].set(12.0 * 35.3145); /* reactor.liquid_volume — 12 m³, dentro de [2,24] */
    offered[2].set(120.4); /* reactor.temperature — dentro de <175 */
    offered[3].set(6.0 * 35.3145); /* separator.liquid_volume — 6 m³, dentro de [1,12] */
    offered[4].set(4.0 * 35.3145); /* stripper.liquid_volume — 4 m³, dentro de [1,8] */

    let config = Snapshot::from_pairs(&[]);
    let detector = ShutdownDetector::new(&mut registry.borrow_mut(), &config);
    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

    detector.evaluate();

    let (_, needed) = registry.borrow_mut().subscribe(&[], &["status.shutdown_detected"]);
    registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
    assert_eq!(needed[0].get(), 0.0);
}
