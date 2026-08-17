/* tep/controllers/separator_level_control.rs */

/* XMEAS(12) Separator Liquid Level (%) -> XMV(7) Separator Underflow Valve — a malha de nível do
separador (docs/07-controle.md).

Mesmo controlador P clássico de reactor_pressure_control.rs: `mv = clamp(bias + Kp*(medida -
setpoint), 0, 100)`. Kp=1.0, setpoint=50%, bias=38.1 são os parâmetros validados em
`experimentos.md` (Exp 10/11/13) — sem essa malha, o separador acumula ou esvazia líquido sem
controle: nada mais no sistema regula o inventário do vaso.
*/
#[monjolo::controller(name = "separator_level_control")]
pub struct SeparatorLevelControl {
    #[sensor(key = "xmeas.separator_level")]
    level: f64,
    #[actuator(key = "valve.separator_underflow.position")]
    underflow: f64,
}

impl SeparatorLevelControl {
    fn control(&self) {
        const KP: f64 = 1.0;
        const SETPOINT: f64 = 50.0;
        const BIAS: f64 = 38.1;

        let measurement = self.level().read();
        let output = (BIAS + KP * (measurement - SETPOINT)).clamp(0.0, 100.0);
        self.underflow().write(output);
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

    fn seed_registry(registry: &mut StateRegistry, level: f64) -> Rc<ConcreteActuator> {
        let (offered, _) = registry.subscribe(&["xmeas.separator_level"], &[]);
        offered[0].set(level);
        ConcreteSensor::new(registry, "xmeas.separator_level", Box::new(Ideal));
        ConcreteActuator::new(registry, "valve.separator_underflow.position", |command, _state| command)
    }

    #[test]
    fn control_matches_the_hand_computed_p_law() {
        let registry = StateRegistry::shared();
        let underflow = seed_registry(&mut registry.borrow_mut(), 55.0);
        let controller = SeparatorLevelControl::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");
        registry.borrow_mut().commit();

        controller.evaluate();
        underflow.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["valve.separator_underflow.position.derivative"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");

        let expected = (38.1_f64 + 1.0 * (55.0 - 50.0)).clamp(0.0, 100.0);
        assert_eq!(needed[0].get(), expected, "bias + Kp*(medida - setpoint), sem saturar");
    }

    #[test]
    fn control_clamps_output_to_valve_range() {
        let registry = StateRegistry::shared();
        let underflow = seed_registry(&mut registry.borrow_mut(), -1000.0); // nível absurdamente baixo, força saturação em 0
        let controller = SeparatorLevelControl::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");
        registry.borrow_mut().commit();

        controller.evaluate();
        underflow.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["valve.separator_underflow.position.derivative"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 0.0, "clamp(..., 0.0, 100.0) satura no piso da válvula");
    }
}
