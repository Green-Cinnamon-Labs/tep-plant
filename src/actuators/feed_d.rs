/* tep/actuators/feed_d.rs */

/* XMV-1: D Feed Flow. VTAU(1) = 8s.

Prova de conceito de `#[actuator(...)]` (`monjolo-macros`, branch feat/proc-macro-components):
`FeedD` é a MESMA lei física da versão à mão de `feed_e` (mesma chave, mesmo τ, mesma fórmula), só
que declarada como struct+atributos em vez de `Actuator::new()` + closure. `command`/`position`
viram getters (`self.command()`/`self.position()`); `dynamics()` é código comum do usuário, nunca
tocado pela macro — só a struct é reescrita.
*/
#[monjolo::actuator(key = "valve.feed_d.position")]
pub struct FeedD {
    #[command]
    command: f64,
    #[state]
    position: f64,
}

impl FeedD {
    fn dynamics(&self) -> f64 {
        let tau = 8.0 / 3600.0;
        (self.command() - self.position()) / tau
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use monjolo::actuator::Actuator as ActuatorTrait;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::state_registry::StateRegistry;

    /* Mesma prova de derivative_is_real_not_a_stub (monjolo/monjolo/actuator/model.rs), agora
    contra a versão gerada por `#[actuator(...)]` — prova que a struct+atributos se comporta
    identicamente à versão escrita à mão: mesmo τ = 8s, mesma fórmula (command - position) / tau.
    `__derivative` é campo privado (gerado pela macro sem `pub`), acessível aqui pelo mesmo motivo
    de `actuator.derivative` no teste original: `tests` é submódulo do módulo onde a struct nasceu.
    */
    #[test]
    fn feed_d_derivative_matches_the_hand_written_law() {
        let registry = StateRegistry::shared();
        let feed_d = FeedD::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().unwrap();

        feed_d.write(72.0);
        /* posição nasce em 0.0 (default do slot) — derivada esperada: (72-0)/(8/3600) = 32400 */
        feed_d.evaluate();
        assert_eq!(feed_d.__derivative.get(), (72.0 - 0.0) / (8.0 / 3600.0));
    }

    #[test]
    fn feed_d_registers_itself_under_its_own_key() {
        let registry = StateRegistry::shared();
        let feed_d = FeedD::new(&mut registry.borrow_mut());
        let feed_d: Rc<dyn ActuatorTrait> = feed_d;

        let found = registry
            .borrow()
            .actuator("valve.feed_d.position")
            .expect("deveria estar no catálogo");
        assert!(Rc::ptr_eq(&feed_d, &found));
    }

    #[test]
    fn feed_d_is_a_dynamic_model_too() {
        let registry = StateRegistry::shared();
        let feed_d = FeedD::new(&mut registry.borrow_mut());
        registry.borrow_mut().resolve().unwrap();

        let feed_d: Rc<dyn DynamicModel> = feed_d;
        feed_d.evaluate(); // não deve entrar em pânico
    }
}
