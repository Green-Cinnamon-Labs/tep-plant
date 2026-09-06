/* tep/diagnostics/shutdown_detector.rs */

/** Detecção de shutdown (Block 36 de teprob.f) — 1.0 se qualquer condição de segurança for
violada, 0.0 caso contrário. Migrado do antigo `dynamics/measured.rs` (issue 10): é o único pedaço
de `Measured` sem dono natural — agrega Reactor/Separator/Stripper ao mesmo tempo, então não faz
sentido virar tarefa de nenhuma unidade específica. Fica fora de `units/` (não é um acumulador
físico) num módulo próprio, `diagnostics`.

Prefixo `status.`, não `xmeas.` — não é uma das 41 XMEAS canônicas do TEP, é um diagnóstico à parte
(equivalente ao antigo `isd_active` do gRPC). Volumes em m³ convertidos de kmol via /35.3145 (mesma
conversão usada em XMEAS 8/12/15), mas contra limites absolutos de segurança, não contra o zero de
calibração dos instrumentos — por isso lê os volumes BRUTOS (`reactor.liquid_volume` etc.), não as
XMEAS já convertidas. A pressão do reator É a XMEAS já convertida (`xmeas.reactor.pressure`,
kPa) — o limite de 3000 é expresso nessa unidade no original, não em mmHg bruto.
*/
#[monjolo::dynamic_model]
pub struct ShutdownDetector {
    #[need(key = "xmeas.reactor.pressure")]
    reactor_pressure_kpa: f64,
    #[need(key = "reactor.liquid_volume")]
    reactor_liquid_volume: f64,
    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,
    #[need(key = "separator.liquid_volume")]
    separator_liquid_volume: f64,
    #[need(key = "stripper.liquid_volume")]
    stripper_liquid_volume: f64,

    #[offer(key = "status.shutdown_detected")]
    shutdown_detected: f64,
}

impl ShutdownDetector {
    fn compute(&self) {
        let shutdown_detected = self.reactor_pressure_kpa() > 3000.0
            || self.reactor_liquid_volume() / 35.3145 > 24.0
            || self.reactor_liquid_volume() / 35.3145 < 2.0
            || self.reactor_temperature() > 175.0
            || self.separator_liquid_volume() / 35.3145 > 12.0
            || self.separator_liquid_volume() / 35.3145 < 1.0
            || self.stripper_liquid_volume() / 35.3145 > 8.0
            || self.stripper_liquid_volume() / 35.3145 < 1.0;

        self.set_shutdown_detected(if shutdown_detected { 1.0 } else { 0.0 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

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
        offered[0].set(3000.1); // xmeas.reactor.pressure já em kPa

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
        offered[0].set(2110.6); // xmeas.reactor.pressure — dentro do normal (docs/07-controle.md)
        offered[1].set(12.0 * 35.3145); // reactor.liquid_volume — 12 m³, dentro de [2,24]
        offered[2].set(120.4); // reactor.temperature — dentro de <175
        offered[3].set(6.0 * 35.3145); // separator.liquid_volume — 6 m³, dentro de [1,12]
        offered[4].set(4.0 * 35.3145); // stripper.liquid_volume — 4 m³, dentro de [1,8]

        let config = Snapshot::from_pairs(&[]);
        let detector = ShutdownDetector::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        detector.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(&[], &["status.shutdown_detected"]);
        registry.borrow_mut().resolve().expect("chave já ofertada deveria resolver de novo sem erro");
        assert_eq!(needed[0].get(), 0.0);
    }
}
