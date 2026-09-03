/* tep/dynamics/purge_analyzer.rs */

/* XMEAS(29..36) — Purge Gas Analysis (Stream 9, componentes A-H), teprob.f:162-174. Publica a
composição já calculada pelo Separator (stream 9, purge, compartilha a mesma composição de vapor
que alimenta o recycle na stream 8 — Block 27 de teprob.f, `FCM(I,9)`/`FCM(I,8)` usam o mesmo
`XST(.,9)=XST(.,8)`) convertida de fração molar pra mol%. Sem atraso/amostragem ainda — mesmo
raciocínio de `reactor_feed_analyzer.rs`.
*/
#[monjolo::dynamic_model(after = ["ReactorFeedAnalyzer"])]
pub struct PurgeAnalyzer {
    #[need(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    composition: [f64; 8],

    #[offer(prefix = "xmeas.stream9.component", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    mole_percent: [f64; 8],
}

impl PurgeAnalyzer {
    fn compute(&self) {
        let composition = self.composition();
        let mut mole_percent = [0.0f64; 8];
        for i in 0..8 {
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
                "separator.vapor_composition.a",
                "separator.vapor_composition.b",
                "separator.vapor_composition.c",
                "separator.vapor_composition.d",
                "separator.vapor_composition.e",
                "separator.vapor_composition.f",
                "separator.vapor_composition.g",
                "separator.vapor_composition.h",
            ],
            &[],
        );
        for (i, value) in [0.10, 0.15, 0.05, 0.10, 0.10, 0.10, 0.20, 0.20].into_iter().enumerate() {
            offered[i].set(value);
        }

        let config = Snapshot::from_pairs(&[]);
        let analyzer = PurgeAnalyzer::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        analyzer.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(
            &[],
            &[
                "xmeas.stream9.component.a",
                "xmeas.stream9.component.b",
                "xmeas.stream9.component.c",
                "xmeas.stream9.component.d",
                "xmeas.stream9.component.e",
                "xmeas.stream9.component.f",
                "xmeas.stream9.component.g",
                "xmeas.stream9.component.h",
            ],
        );
        registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

        let expected = [10.0, 15.0, 5.0, 10.0, 10.0, 10.0, 20.0, 20.0];
        for (i, exp) in expected.into_iter().enumerate() {
            assert_eq!(needed[i].get(), exp);
        }
    }
}
