/* Testes de `controllers::stripper_level_control`. */

use monjolo::actuator::model::Actuator as ConcreteActuator;
use monjolo::dynamic_model::DynamicModel;
use monjolo::sensor::model::{Ideal, Sensor as ConcreteSensor};
use monjolo::state_registry::StateRegistry;
use std::rc::Rc;
use tennessee_eastman_process::controllers::StripperLevelControl;

const INTEGRAL: &str = "controller.stripper_level.integral";

fn seed_registry(registry: &mut StateRegistry, level: f64) -> Rc<ConcreteActuator> {
    let (offered, _) = registry.subscribe(&["xmeas.stripper.level"], &[]);
    offered[0].set(level);
    ConcreteSensor::new(registry, "xmeas.stripper.level", Box::new(Ideal));
    ConcreteActuator::new(registry, "valve.stripper_product.position", |command, _state| command)
}

/* Mesmo arranjo do `run` de reactor_pressure_control.rs — devolve (saída escrita na válvula,
dI/dt publicado para o Integrator).
*/
fn run(level: f64, integral: f64) -> (f64, f64) {
    let registry = StateRegistry::shared();
    let product = seed_registry(&mut registry.borrow_mut(), level);
    let controller = StripperLevelControl::new(&mut registry.borrow_mut());
    let derivative_key = format!("{INTEGRAL}.derivative");
    let (_, state) = registry.borrow_mut().subscribe(&[], &[INTEGRAL, &derivative_key]);
    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");
    registry.borrow_mut().commit();
    state[0].set(integral);

    controller.evaluate();
    product.evaluate();

    let (_, needed) = registry.borrow_mut().subscribe(&[], &["valve.stripper_product.position.derivative"]);
    registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
    (needed[0].get(), state[1].get())
}

#[test]
fn with_zero_integral_the_output_is_the_old_p_law() {
    let (output, derivative) = run(45.0, 0.0);
    let expected = (46.5_f64 + 1.0 * (45.0 - 50.0)).clamp(0.0, 100.0);
    assert_eq!(output, expected, "I = 0: bias + Kc*(medida - setpoint), sem solavanco");
    assert_eq!(derivative, 45.0 - 50.0, "fora da saturação, dI/dt = e");
}

#[test]
fn control_matches_the_hand_computed_pi_law() {
    let (output, derivative) = run(45.0, -2.0);
    let expected = 46.5 + 1.0 * ((45.0 - 50.0) + -2.0 / (200.0 / 60.0));
    assert!((output - expected).abs() < 1e-9, "bias + Kc*(e + I/τi): esperava {expected}, deu {output}");
    assert_eq!(derivative, 45.0 - 50.0, "fora da saturação, dI/dt = e");
}

#[test]
fn control_clamps_output_to_valve_range() {
    let (output, _) = run(1000.0, 0.0); /* nível absurdamente alto, força saturação em 100 */
    assert_eq!(output, 100.0, "clamp(..., 0.0, 100.0) satura no teto da válvula");
    let (output, _) = run(-1000.0, 0.0); /* nível absurdamente baixo, força saturação em 0 */
    assert_eq!(output, 0.0, "clamp(..., 0.0, 100.0) satura no piso da válvula");
}

#[test]
fn integral_stops_while_saturated_and_pushing_further_out() {
    let (output, derivative) = run(1000.0, 0.0);
    assert_eq!(output, 100.0);
    assert_eq!(derivative, 0.0, "saturada no teto com e > 0: a integral não pode crescer");
    let (output, derivative) = run(-1000.0, 0.0);
    assert_eq!(output, 0.0);
    assert_eq!(derivative, 0.0, "saturada no piso com e < 0: a integral não pode decrescer");
}

#[test]
fn integral_keeps_running_while_saturated_but_coming_back() {
    /* I negativa o bastante para saturar no piso com o nível acima do setpoint: o erro já puxa
    a saída de volta, então a integral tem de seguir descarregando.
    */
    let (output, derivative) = run(55.0, -300.0);
    assert_eq!(output, 0.0);
    assert_eq!(derivative, 55.0 - 50.0, "saturada mas voltando: dI/dt = e");
}
