/* Testes de `controllers::reactor_pressure_control`. */

use monjolo::actuator::model::Actuator as ConcreteActuator;
use monjolo::dynamic_model::DynamicModel;
use monjolo::sensor::model::{Ideal, Sensor as ConcreteSensor};
use monjolo::state_registry::StateRegistry;
use std::rc::Rc;
use tennessee_eastman_process::controllers::ReactorPressureControl;

const INTEGRAL: &str = "controller.reactor_pressure.integral";

/* Reproduz xmeas.reactor.pressure (offer bruto, o que Measured faria — já em kPa, não o
reactor.pressure bruto em mmHg que Reactor publica) + o sensor/atuador reais que o controller
precisa — mesmo padrão de component.rs::tests, mas contra o controller real da planta. O
atuador nasce com dinâmica IDENTIDADE (`|command, _state| command`): a derivada que ele publica
passa a ser exatamente o `command` que `control()` escreveu, o jeito mais direto de verificar o
valor calculado sem expor um getter de `command` só para teste.
*/
fn seed_registry(registry: &mut StateRegistry, pressure_kpa: f64) -> Rc<ConcreteActuator> {
    let (offered, _) = registry.subscribe(&["xmeas.reactor.pressure"], &[]);
    offered[0].set(pressure_kpa);
    ConcreteSensor::new(registry, "xmeas.reactor.pressure", Box::new(Ideal));
    ConcreteActuator::new(registry, "valve.purge.position", |command, _state| command)
}

/* Monta o controller com a integral em `integral` e roda um `evaluate()`. Devolve (saída
escrita na purga, dI/dt que o controller publicou para o Integrator). A integral é escrita
pelo mesmo caminho que o Integrator usa: um `need` na chave de estado.
*/
fn run(pressure_kpa: f64, integral: f64) -> (f64, f64) {
    let registry = StateRegistry::shared();
    let purge = seed_registry(&mut registry.borrow_mut(), pressure_kpa);
    let controller = ReactorPressureControl::new(&mut registry.borrow_mut());
    let derivative_key = format!("{INTEGRAL}.derivative");
    let (_, state) = registry.borrow_mut().subscribe(&[], &[INTEGRAL, &derivative_key]);
    registry.borrow_mut().resolve().expect("todo input deveria ter provedor");
    /* Sensor lê CurrentState — precisa de commit() antes do 1º read() */
    registry.borrow_mut().commit();
    state[0].set(integral);

    /* control() escreve o command em purge e devolve dI/dt; o Actuator publica como derivada
    dynamics(command, state) = command (identidade)
    */
    controller.evaluate();
    purge.evaluate();

    let (_, needed) = registry.borrow_mut().subscribe(&[], &["valve.purge.position.derivative"]);
    registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
    (needed[0].get(), state[1].get())
}

#[test]
fn with_zero_integral_the_output_is_the_old_p_law() {
    let (output, derivative) = run(2750.0, 0.0);
    let expected = (40.06_f64 + 0.1 * (2750.0 - 2705.0)).clamp(0.0, 100.0);
    assert_eq!(output, expected, "I = 0: bias + Kc*(medida - setpoint), sem solavanco");
    assert_eq!(derivative, 2750.0 - 2705.0, "fora da saturação, dI/dt = e");
}

#[test]
fn control_matches_the_hand_computed_pi_law() {
    let (output, derivative) = run(2750.0, 30.0);
    let expected = 40.06 + 0.1 * ((2750.0 - 2705.0) + 30.0 / (20.0 / 60.0));
    assert!((output - expected).abs() < 1e-9, "bias + Kc*(e + I/τi): esperava {expected}, deu {output}");
    assert_eq!(derivative, 2750.0 - 2705.0, "fora da saturação, dI/dt = e");
}

#[test]
fn control_clamps_output_to_valve_range() {
    let (output, _) = run(10_000.0, 0.0); /* pressão absurda, força saturação em 100 */
    assert_eq!(output, 100.0, "clamp(..., 0.0, 100.0) satura no teto da válvula");
    let (output, _) = run(0.0, 0.0); /* pressão zero, força saturação em 0 */
    assert_eq!(output, 0.0, "clamp(..., 0.0, 100.0) satura no piso da válvula");
}

#[test]
fn integral_stops_while_saturated_and_pushing_further_out() {
    let (output, derivative) = run(10_000.0, 0.0);
    assert_eq!(output, 100.0);
    assert_eq!(derivative, 0.0, "saturada no teto com e > 0: a integral não pode crescer");
    let (output, derivative) = run(0.0, 0.0);
    assert_eq!(output, 0.0);
    assert_eq!(derivative, 0.0, "saturada no piso com e < 0: a integral não pode decrescer");
}

#[test]
fn integral_keeps_running_while_saturated_but_coming_back() {
    /* I grande o bastante para saturar no teto mesmo com a pressão abaixo do setpoint: o erro
    já puxa a saída de volta para dentro da faixa, então a integral tem de seguir descarregando.
    */
    let (output, derivative) = run(2690.0, 300.0);
    assert_eq!(output, 100.0);
    assert_eq!(derivative, 2690.0 - 2705.0, "saturada mas voltando: dI/dt = e");
}
