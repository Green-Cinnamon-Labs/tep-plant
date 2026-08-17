/* tep/controllers/stripper_level_control.rs */

/* XMEAS(15) Stripper Liquid Level (%) -> XMV(8) Stripper Product Valve — a malha de nível do
stripper (docs/07-controle.md).

Mesmo controlador P clássico de reactor_pressure_control.rs: `mv = clamp(bias + Kp*(medida -
setpoint), 0, 100)`. Kp=1.0, setpoint=50%, bias=46.5 são os parâmetros validados em
`experimentos.md` (Exp 10/11/13) — sem essa malha, o stripper acumula ou esvazia líquido sem
controle, pelo mesmo motivo de separator_level_control.rs.
*/
#[monjolo::controller(name = "stripper_level_control")]
pub struct StripperLevelControl {
    #[sensor(key = "xmeas.stripper_level")]
    level: f64,
    #[actuator(key = "valve.stripper_product.position")]
    product: f64,
}

impl StripperLevelControl {
    fn control(&self) {
        const KP: f64 = 1.0;
        const SETPOINT: f64 = 50.0;
        const BIAS: f64 = 46.5;

        let measurement = self.level().read();
        let output = (BIAS + KP * (measurement - SETPOINT)).clamp(0.0, 100.0);
        self.product().write(output);
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
        let (offered, _) = registry.subscribe(&["xmeas.stripper_level"], &[]);
        offered[0].set(level);
        ConcreteSensor::new(registry, "xmeas.stripper_level", Box::new(Ideal));
        ConcreteActuator::new(registry, "valve.stripper_product.position", |command, _state| command)
    }

    #[test]
    fn control_matches_the_hand_computed_p_law() {
        let registry = StateRegistry::shared();
        let product = seed_registry(&mut registry.borrow_mut(), 45.0);
        let controller = StripperLevelControl::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");
        registry.borrow_mut().commit();

        controller.evaluate();
        product.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["valve.stripper_product.position.derivative"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");

        let expected = (46.5_f64 + 1.0 * (45.0 - 50.0)).clamp(0.0, 100.0);
        assert_eq!(needed[0].get(), expected, "bias + Kp*(medida - setpoint), sem saturar");
    }

    #[test]
    fn control_clamps_output_to_valve_range() {
        let registry = StateRegistry::shared();
        let product = seed_registry(&mut registry.borrow_mut(), 1000.0); // nível absurdamente alto, força saturação em 100
        let controller = StripperLevelControl::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");
        registry.borrow_mut().commit();

        controller.evaluate();
        product.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["valve.stripper_product.position.derivative"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 100.0, "clamp(..., 0.0, 100.0) satura no teto da válvula");
    }
}
