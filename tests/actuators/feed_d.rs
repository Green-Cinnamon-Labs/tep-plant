/* Testes de `actuators::feed_d`. */

use std::rc::Rc;

use monjolo::actuator::Actuator as ActuatorTrait;
use monjolo::dynamic_model::DynamicModel;
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;
use tennessee_eastman_process::actuators::FeedD;

/* A derivada que o atuador publica, lida pela chave `valve.feed_d.position.derivative` — do mesmo
jeito que o Integrator a lê. Dentro do crate estes testes liam o campo `__derivative` direto, mas
ele é privado (gerado pela macro sem `pub`) e não é visível daqui.
*/
fn derivative(registry: &mut StateRegistry) -> f64 {
    let (_, needed) = registry.subscribe(&[], &["valve.feed_d.position.derivative"]);
    registry.resolve().expect("chave já ofertada deveria resolver de novo sem erro");
    needed[0].get()
}

/* Mesma prova de derivative_is_real_not_a_stub (monjolo/monjolo/actuator/model.rs), agora
contra a versão gerada por `#[actuator(...)]` — prova que a struct+atributos se comporta
identicamente à versão escrita à mão: mesmo τ = 8s, mesma fórmula (command - position) / tau.
*/
#[test]
fn feed_d_derivative_matches_the_hand_written_law() {
    let registry = StateRegistry::shared();
    /* sem "state.valves.d_feed" — nasce em 0.0, como antes */
    let config = Snapshot::from_pairs(&[]);
    let feed_d = FeedD::new(&mut registry.borrow_mut(), &config);
    registry.borrow_mut().resolve().unwrap();

    feed_d.write(72.0);
    /* posição nasce em 0.0 (config vazio) — derivada esperada: (72-0)/(8/3600) = 32400 */
    feed_d.evaluate();
    assert_eq!(derivative(&mut registry.borrow_mut()), (72.0 - 0.0) / (8.0 / 3600.0));
}

/** A capacidade nova: `config = "state.valves.d_feed"` semeia `command` E `position` com o
MESMO valor nominal — `dynamics()` (command - position)/tau nasce em zero, nada deriva sozinho
até algo escrever um `command` diferente (ver doc de `#[actuator(...)]`, monjolo-macros/lib.rs).
Antes desta capacidade, todo atuador nascia em 0.0 incondicionalmente, mesmo com
`application.toml` já tendo o valor certo em `[state.valves]` — a causa raiz de "a planta não
estabiliza porque as válvulas manuais nascem fechadas".
*/
#[test]
fn feed_d_seeds_command_and_position_from_config() {
    let registry = StateRegistry::shared();
    let config = Snapshot::from_pairs(&[("state.valves.d_feed", 63.05263038999999)]);
    let feed_d = FeedD::new(&mut registry.borrow_mut(), &config);
    registry.borrow_mut().resolve().unwrap();

    assert_eq!(feed_d.command(), 63.05263038999999);
    assert_eq!(feed_d.position(), 63.05263038999999);

    feed_d.evaluate();
    assert_eq!(
        derivative(&mut registry.borrow_mut()),
        0.0,
        "command == position no nascimento — nada deriva sozinho"
    );
}

#[test]
fn feed_d_registers_itself_under_its_own_key() {
    let registry = StateRegistry::shared();
    let config = Snapshot::from_pairs(&[]);
    let feed_d = FeedD::new(&mut registry.borrow_mut(), &config);
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
    let config = Snapshot::from_pairs(&[]);
    let feed_d = FeedD::new(&mut registry.borrow_mut(), &config);
    registry.borrow_mut().resolve().unwrap();

    let feed_d: Rc<dyn DynamicModel> = feed_d;
    feed_d.evaluate(); /* não deve entrar em pânico */
}
