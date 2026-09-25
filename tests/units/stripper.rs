/* Testes de `units::stripper`. */

use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::units::stripper::Stripper;

#[test]
fn new_seeds_own_state_with_initial_condition() {
    let registry = StateRegistry::shared();
    let initial = Snapshot::from_pairs(&[
        ("state.stripper_liquid.A", 1.0),
        ("state.stripper_liquid.B", 2.0),
        ("state.stripper_liquid.C", 3.0),
        ("state.stripper_liquid.D", 4.0),
        ("state.stripper_liquid.E", 5.0),
        ("state.stripper_liquid.F", 6.0),
        ("state.stripper_liquid.G", 7.0),
        ("state.stripper_liquid.H", 8.0),
        ("state.stripper.energy", 42.0),
    ]);

    let stripper = Stripper::new(&mut registry.borrow_mut(), &initial);

    assert_eq!(stripper.liquid(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    assert_eq!(stripper.enthalpy(), 42.0);
}

#[test]
fn new_defaults_missing_keys_to_zero() {
    let registry = StateRegistry::shared();
    let initial = Snapshot::from_pairs(&[]);

    let stripper = Stripper::new(&mut registry.borrow_mut(), &initial);

    assert_eq!(stripper.liquid(), [0.0; 8]);
    assert_eq!(stripper.enthalpy(), 0.0);
}
