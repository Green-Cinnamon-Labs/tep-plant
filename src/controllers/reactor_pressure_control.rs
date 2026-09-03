/* tep/controllers/reactor_pressure_control.rs */

/* XMEAS(7) Reactor Pressure -> XMV(6) Purge Valve — o laço de pressão do reator (docs/07-controle.md).

Controlador P clássico do TEP (Downs & Vogel 1993): `mv = clamp(bias + Kp*(medida - setpoint), 0,
100)`. Kp=0.1, setpoint=2705 kPa, bias=40.06 são os mesmos parâmetros usados nos experimentos que
validaram esta malha (`experimentos.md`, Exp 10/11/13) — sem essa malha, a pressão do reator deriva
sem limite (Exp 8: desbalanço de massa gasosa global, ~0.15%/h) porque a purge é a única saída de gás
da planta e nada mais a regula.
*/
#[monjolo::controller(name = "reactor_pressure_control")]
pub struct ReactorPressureControl {
    #[sensor(key = "xmeas.reactor.pressure")]
    pressure: f64,
    #[actuator(key = "valve.purge.position")]
    purge: f64,
}

impl ReactorPressureControl {
    fn control(&self) {
        const KP: f64 = 0.1;
        const SETPOINT: f64 = 2705.0;
        const BIAS: f64 = 40.06;

        let measurement = self.pressure().read();
        let output = (BIAS + KP * (measurement - SETPOINT)).clamp(0.0, 100.0);
        self.purge().write(output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::actuator::model::Actuator as ConcreteActuator;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::sensor::model::{Ideal, Sensor as ConcreteSensor};
    use monjolo::state_registry::StateRegistry;
    use std::rc::Rc;

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

    #[test]
    fn control_matches_the_hand_computed_p_law() {
        let registry = StateRegistry::shared();
        let purge = seed_registry(&mut registry.borrow_mut(), 2750.0);
        let controller = ReactorPressureControl::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");
        registry.borrow_mut().commit(); // Sensor lê CurrentState — precisa de commit() antes do 1º read()

        controller.evaluate(); // control() escreve o command calculado em purge
        purge.evaluate(); // Actuator::evaluate(): derivada = dynamics(command, state) = command (identidade)

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["valve.purge.position.derivative"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");

        let expected = (40.06_f64 + 0.1 * (2750.0 - 2705.0)).clamp(0.0, 100.0);
        assert_eq!(needed[0].get(), expected, "bias + Kp*(medida - setpoint), sem saturar");
    }

    #[test]
    fn control_clamps_output_to_valve_range() {
        let registry = StateRegistry::shared();
        let purge = seed_registry(&mut registry.borrow_mut(), 10_000.0); // pressão absurda, força saturação em 100
        let controller = ReactorPressureControl::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");
        registry.borrow_mut().commit();

        controller.evaluate();
        purge.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["valve.purge.position.derivative"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 100.0, "clamp(..., 0.0, 100.0) satura no teto da válvula");
    }
}
