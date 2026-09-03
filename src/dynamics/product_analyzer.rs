/* tep/dynamics/product_analyzer.rs */

/* XMEAS(37..41) — Product Analysis (Stream 11, componentes D-H), teprob.f:177-186. Publica a
composição líquida já calculada pelo Stripper (stream 11, o produto final — a mesma composição que
`valve.stripper_product.position` escoa) convertida de fração molar pra mol%. Sem atraso/amostragem
ainda — mesmo raciocínio de `reactor_feed_analyzer.rs`.
*/
#[monjolo::dynamic_model(after = ["PurgeAnalyzer"])]
pub struct ProductAnalyzer {
    #[need(prefix = "stripper.liquid_composition", components = ["3", "4", "5", "6", "7"])]
    composition: [f64; 5],

    #[offer(prefix = "xmeas.stream11.component", components = ["d", "e", "f", "g", "h"])]
    mole_percent: [f64; 5],
}

impl ProductAnalyzer {
    fn compute(&self) {
        let composition = self.composition();
        let mut mole_percent = [0.0f64; 5];
        for i in 0..5 {
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
                "stripper.liquid_composition.3",
                "stripper.liquid_composition.4",
                "stripper.liquid_composition.5",
                "stripper.liquid_composition.6",
                "stripper.liquid_composition.7",
            ],
            &[],
        );
        for (i, value) in [0.05, 0.10, 0.15, 0.30, 0.40].into_iter().enumerate() {
            offered[i].set(value);
        }

        let config = Snapshot::from_pairs(&[]);
        let analyzer = ProductAnalyzer::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        analyzer.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(
            &[],
            &[
                "xmeas.stream11.component.d",
                "xmeas.stream11.component.e",
                "xmeas.stream11.component.f",
                "xmeas.stream11.component.g",
                "xmeas.stream11.component.h",
            ],
        );
        registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

        let expected = [5.0, 10.0, 15.0, 30.0, 40.0];
        for (i, exp) in expected.into_iter().enumerate() {
            assert_eq!(needed[i].get(), exp);
        }
    }
}
