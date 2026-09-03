/* tep/dynamics/reactor_feed_analyzer.rs */

/* XMEAS(23..28) — Reactor Feed Analysis (Stream 6, componentes A-F), teprob.f:149-159. Publica a
composição já calculada pelo Compressor (stream 6 = cópia bit-a-bit da composição de vapor do
Compressor — Block 31 de teprob.f, "bypass") convertida de fração molar pra mol% (teprob.f: "Units
= Mole %", `XCMP(I) = XST(componente,stream) * 100.0`). O estado interno da planta (a composição em
si) fica intocado — a conversão só acontece aqui, na camada de exposição.

Sem atraso/amostragem ainda: publica o valor instantâneo a cada tick, igual a qualquer XMEAS 1-22.
teprob.f amostra isso com período = tempo morto = 0.1h (Block 39) — quando essa fidelidade temporal
for necessária, o mecanismo deve virar um refinamento DESTE componente (comportamento específico do
TEP), nunca um `SensorBehavior` genérico do monjolo (esse fica reservado a características de
aquisição: `Ideal`, `Noisy`, `Hysteresis`).
*/
#[monjolo::dynamic_model(after = ["Measured"])]
pub struct ReactorFeedAnalyzer {
    #[need(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5"])]
    composition: [f64; 6],

    #[offer(prefix = "xmeas.stream6.component", components = ["a", "b", "c", "d", "e", "f"])]
    mole_percent: [f64; 6],
}

impl ReactorFeedAnalyzer {
    fn compute(&self) {
        let composition = self.composition();
        let mut mole_percent = [0.0f64; 6];
        for i in 0..6 {
            mole_percent[i] = composition[i] * 100.0;
        }
        self.set_mole_percent(mole_percent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    #[test]
    fn converts_mole_fraction_to_mole_percent() {
        let registry = StateRegistry::shared();
        let (offered, _) = registry.borrow_mut().subscribe(
            &[
                "compressor.vapor_composition.0",
                "compressor.vapor_composition.1",
                "compressor.vapor_composition.2",
                "compressor.vapor_composition.3",
                "compressor.vapor_composition.4",
                "compressor.vapor_composition.5",
            ],
            &[],
        );
        offered[0].set(0.10);
        offered[1].set(0.20);
        offered[2].set(0.05);
        offered[3].set(0.30);
        offered[4].set(0.15);
        offered[5].set(0.20);

        let config = Snapshot::from_pairs(&[]);
        let analyzer = ReactorFeedAnalyzer::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        analyzer.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(
            &[],
            &[
                "xmeas.stream6.component.a",
                "xmeas.stream6.component.b",
                "xmeas.stream6.component.c",
                "xmeas.stream6.component.d",
                "xmeas.stream6.component.e",
                "xmeas.stream6.component.f",
            ],
        );
        registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

        assert_eq!(needed[0].get(), 10.0);
        assert_eq!(needed[1].get(), 20.0);
        assert_eq!(needed[2].get(), 5.0);
        assert_eq!(needed[3].get(), 30.0);
        assert_eq!(needed[4].get(), 15.0);
        assert_eq!(needed[5].get(), 20.0);
    }
}
