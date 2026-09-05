/* tep/dynamics/feed.rs */

/** As 4 alimentações externas do TEP (D/E/A/A+C) — sem inventário próprio (nenhuma EDO, nenhum
`#[state]`: são puramente vazão-por-posição-de-válvula, sem acumulação física nenhuma), mas com
identidade própria o bastante pra merecer uma unidade: cada uma tem sua composição fixa (TEINIT),
sua faixa de válvula (VRNG) e, pras duas medidas por massa (D/E), seu próprio peso molecular.

Migrado de `dynamics::flows` (issue 10, spec-tennessee-eastman) — antes, essas 4 vazões e os 2
pesos moleculares eram calculados dentro de `Flows::compute()`, sem nenhum dono próprio. As chaves
publicadas (`flows.stream_flow.0..3`, `flows.d_feed_mol_weight`/`.e_feed_mol_weight`) continuam
EXATAMENTE as mesmas — só quem as publica mudou; `Flows`/`Derivatives`/`Measured` continuam lendo
por chave, indiferentes à origem.
*/

use crate::physics::constants::TepConstants;

/* Vazão máxima de cada válvula com curva linear (posição% * range / 100) — VRNG em TEINIT. */
const FEED_D_RANGE: f64 = 400.0;
const FEED_E_RANGE: f64 = 400.0;
const FEED_A_RANGE: f64 = 100.0;
const FEED_AC_RANGE: f64 = 1500.0;

/* Composições nominais dos feeds puros (TEINIT) — índice de componente A=0,B=1,C=2,D=3,E=4,F=5,
G=6,H=7, mesma convenção de physics/constants.rs. `pub(crate)` — `dynamics::flows`/
`dynamics::derivatives` reusam FEED_AC_COMPOSITION/FEED_D_COMPOSITION/etc. pro balanço de massa/
energia do compressor e pro flash do stripper, em vez de duplicar os números.
*/
pub(crate) const FEED_D_COMPOSITION: [f64; 8] = [0.0, 0.0001, 0.0, 0.9999, 0.0, 0.0, 0.0, 0.0];
pub(crate) const FEED_E_COMPOSITION: [f64; 8] = [0.0, 0.0, 0.0, 0.0, 0.9999, 0.0001, 0.0, 0.0];
pub(crate) const FEED_A_COMPOSITION: [f64; 8] = [0.9999, 0.0001, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
pub(crate) const FEED_AC_COMPOSITION: [f64; 8] = [0.4850, 0.0050, 0.5100, 0.0, 0.0, 0.0, 0.0, 0.0];
/* °C — nominal, os 4 feeds nascem à mesma temperatura em TEINIT. */
pub(crate) const FEED_TEMPERATURE: f64 = 45.0;

fn mol_weight(z: &[f64; 8], constants: &TepConstants) -> f64 {
    (0..8).map(|i| z[i] * constants.xmw[i]).sum()
}

#[monjolo::dynamic_model(tasks)]
pub struct Feed {
    constants: TepConstants,
}

#[monjolo::tasks]
impl Feed {
    #[need(key = "valve.feed_d.position")]
    #[offer(key = "flows.stream_flow.0")]
    fn d_feed_flow(&self, position: f64) -> f64 {
        position * FEED_D_RANGE / 100.0
    }

    #[need(key = "valve.feed_e.position")]
    #[offer(key = "flows.stream_flow.1")]
    fn e_feed_flow(&self, position: f64) -> f64 {
        position * FEED_E_RANGE / 100.0
    }

    /* IDV6 (nominal = 0) atuaria aqui — fora de escopo, mesma lacuna de hoje em Flows. */
    #[need(key = "valve.feed_a.position")]
    #[offer(key = "flows.stream_flow.2")]
    fn a_feed_flow(&self, position: f64) -> f64 {
        position * FEED_A_RANGE / 100.0
    }

    /* IDV7 (nominal = 0) atuaria aqui — mesma lacuna. +1e-10: evita divisão por zero em quem usa
    este valor como denominador (Block 26 do flash split, `flow[3] / flow[10]`) quando a válvula
    está fechada — preservado do original.
    */
    #[need(key = "valve.feed_ac.position")]
    #[offer(key = "flows.stream_flow.3")]
    fn ac_feed_flow(&self, position: f64) -> f64 {
        position * FEED_AC_RANGE / 100.0 + 1e-10
    }

    #[offer(key = "flows.d_feed_mol_weight")]
    fn d_feed_mol_weight(&self) -> f64 {
        mol_weight(&FEED_D_COMPOSITION, &self.constants)
    }

    #[offer(key = "flows.e_feed_mol_weight")]
    fn e_feed_mol_weight(&self) -> f64 {
        mol_weight(&FEED_E_COMPOSITION, &self.constants)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    #[test]
    fn linear_valve_flows_match_hand_computed_values() {
        /* Não oferece `valve.feed_*.position` na mão: `attach_discovered_components` também
        descobre os atuadores REAIS (`FeedD`/`FeedE`/`FeedA`/`FeedAC`, `src/actuators/`), que já
        ofertam essas mesmas chaves sozinhos — ofertar de novo aqui colidiria (StateRegistry
        sobrescreveria o índice silenciosamente, deixando o valor semeado na mão órfão). Em vez
        disso, semeia pelo MESMO caminho de config que os atuadores reais usam em produção
        (`#[actuator(key = ..., config = "state.valves.d_feed")]` etc.) — testa a fiação de ponta a
        ponta (atuador → Feed), não só Feed isolado.
        */
        let registry = StateRegistry::shared();
        let config = Snapshot::from_pairs(&[
            ("state.valves.d_feed", 50.0),
            ("state.valves.e_feed", 25.0),
            ("state.valves.a_feed", 10.0),
            ("state.valves.a_c_feed", 5.0),
        ]);
        let mut root = monjolo::dynamic_model::Composite::new();
        monjolo::attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        root.evaluate();

        let (_, needed) = registry.borrow_mut().subscribe(
            &[],
            &["flows.stream_flow.0", "flows.stream_flow.1", "flows.stream_flow.2", "flows.stream_flow.3"],
        );
        registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

        assert_eq!(needed[0].get(), 50.0 * FEED_D_RANGE / 100.0, "D feed: posição*range/100");
        assert_eq!(needed[1].get(), 25.0 * FEED_E_RANGE / 100.0, "E feed: posição*range/100");
        assert_eq!(needed[2].get(), 10.0 * FEED_A_RANGE / 100.0, "A feed: posição*range/100");
        assert_eq!(needed[3].get(), 5.0 * FEED_AC_RANGE / 100.0 + 1e-10, "A&C feed: posição*range/100 + epsilon");
    }

    #[test]
    fn mol_weights_match_hand_computed_values_from_fixed_feed_composition() {
        let registry = StateRegistry::shared();
        let config = Snapshot::from_pairs(&[]);
        let mut root = monjolo::dynamic_model::Composite::new();
        monjolo::attach_discovered_components(&mut root, &mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        root.evaluate();

        let (_, needed) = registry
            .borrow_mut()
            .subscribe(&[], &["flows.d_feed_mol_weight", "flows.e_feed_mol_weight"]);
        registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

        let constants = TepConstants::new();
        assert_eq!(needed[0].get(), mol_weight(&FEED_D_COMPOSITION, &constants));
        assert_eq!(needed[1].get(), mol_weight(&FEED_E_COMPOSITION, &constants));
    }
}
