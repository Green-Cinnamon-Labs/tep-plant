/* tep/dynamics/derivatives.rs */

/** Fecha o balanço de massa/energia (Block 40 de teprob.f) pros quatro acumuladores — as EDOs de
verdade (yp), que `Reactor`/`Separator`/`Stripper`/`Compressor` deliberadamente NÃO calculam (ver
comentário `[DECISÃO DE MODELAGEM]` em cada um deles): cada `#[state]` ali só declara QUE existe
estado integrável, não QUEM calcula a derivada — e quem calcula é sempre quem tem o balanço de
entrada/saída inteiro à mão ao mesmo tempo.

Por que este componente existe SEPARADO de `Flows`, e não dentro dele: a energia (Block 40, YP(9)/
YP(18)/YP(27)) precisa de QUR/QUS/QUC — só `Heat` tem esses valores, e `Heat` roda DEPOIS de `Flows`
na cadeia (`after = ["Flows"]`, porque `Heat` por sua vez precisa de `flows.agitation_factor`/
`flows.stream_flow.7`/`flows.condenser_ua`). Colocar o balanço final dentro de `Flows` criaria um
ciclo real (Flows precisaria de Heat, Heat precisa de Flows) — algo que a cadeia única desta fase
não permite (ver `monjolo::component`, "Cadeia única, de propósito"). `Derivatives` resolve isso
sendo o único elo que roda depois dos dois, `after = ["Heat"]`, o último da fase (A).

Mapeamento de streams: os 13 slots (`flows.stream_flow.0`..`.12`) são exatamente `FTM(1)`..`FTM(13)`
do original, na mesma ordem (slot N = FORTRAN stream N+1) — não há reordenação nenhuma, só o
deslocamento de índice 1→0. `FCM(componente, stream)` do original — a vazão de UM componente numa
stream — é sempre `composição[stream][componente] * stream_flow[stream]`, exceto pros slots 4
(vapor do flash) e 11 (líquido do flash), cujas composições nunca são publicadas como fração (só a
vazão por componente, `flows.flash_vapor_component_flow`/`flash_liquid_component_flow` — já é
exatamente o FCM que a equação precisa, sem precisar normalizar e multiplicar de novo).
*/

use crate::dynamics::flows::{
    FEED_A_COMPOSITION, FEED_AC_COMPOSITION, FEED_D_COMPOSITION, FEED_E_COMPOSITION,
    FEED_TEMPERATURE,
};
use crate::physics::constants::TepConstants;
use crate::physics::thermo::mixture_enthalpy;

#[monjolo::dynamic_model(after = ["Heat"])]
pub struct Derivatives {
    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,
    #[need(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    reactor_vapor_composition: [f64; 8],
    #[need(prefix = "reactor.reaction_rates", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    reactor_reaction_rates: [f64; 8],
    #[need(key = "reactor.heat_of_reaction")]
    reactor_heat_of_reaction: f64,

    #[need(key = "separator.temperature")]
    separator_temperature: f64,
    #[need(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    separator_vapor_composition: [f64; 8],
    #[need(prefix = "separator.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    separator_liquid_composition: [f64; 8],

    #[need(key = "stripper.temperature")]
    stripper_temperature: f64,
    #[need(prefix = "stripper.liquid_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    stripper_liquid_composition: [f64; 8],

    #[need(key = "compressor.temperature")]
    compressor_temperature: f64,
    #[need(prefix = "compressor.vapor_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    compressor_vapor_composition: [f64; 8],

    #[need(prefix = "flows.stream_flow", components = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"])]
    stream_flows: [f64; 13],
    #[need(prefix = "flows.flash_vapor_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    flash_vapor_component_flow: [f64; 8],
    #[need(prefix = "flows.flash_liquid_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    flash_liquid_component_flow: [f64; 8],
    #[need(key = "flows.compressor_discharge_enthalpy")]
    compressor_discharge_enthalpy: f64,

    #[need(key = "heat.reactor_heat")]
    reactor_heat: f64,
    #[need(key = "heat.separator_heat")]
    separator_heat: f64,
    #[need(key = "heat.condenser_heat")]
    condenser_heat: f64,

    #[offer(prefix = "reactor.state", components = ["vapor_a.derivative", "vapor_b.derivative", "vapor_c.derivative"])]
    reactor_vapor_derivative: [f64; 3],
    #[offer(prefix = "reactor.state", components = ["liquid_d.derivative", "liquid_e.derivative", "liquid_f.derivative", "liquid_g.derivative", "liquid_h.derivative"])]
    reactor_liquid_derivative: [f64; 5],
    #[offer(key = "reactor.state.enthalpy.derivative")]
    reactor_enthalpy_derivative: f64,

    #[offer(prefix = "separator.state", components = ["vapor_a.derivative", "vapor_b.derivative", "vapor_c.derivative"])]
    separator_vapor_derivative: [f64; 3],
    #[offer(prefix = "separator.state", components = ["liquid_d.derivative", "liquid_e.derivative", "liquid_f.derivative", "liquid_g.derivative", "liquid_h.derivative"])]
    separator_liquid_derivative: [f64; 5],
    #[offer(key = "separator.state.enthalpy.derivative")]
    separator_enthalpy_derivative: f64,

    #[offer(prefix = "stripper.state", components = ["0.derivative", "1.derivative", "2.derivative", "3.derivative", "4.derivative", "5.derivative", "6.derivative", "7.derivative"])]
    stripper_liquid_derivative: [f64; 8],
    #[offer(key = "stripper.state.8.derivative")]
    stripper_enthalpy_derivative: f64,

    #[offer(prefix = "compressor.state", components = ["0.derivative", "1.derivative", "2.derivative", "3.derivative", "4.derivative", "5.derivative", "6.derivative", "7.derivative"])]
    compressor_vapor_derivative: [f64; 8],
    #[offer(key = "compressor.state.8.derivative")]
    compressor_enthalpy_derivative: f64,
}

impl Derivatives {
    fn compute(&self) {
        let constants = TepConstants::new();

        let reactor_vapor = self.reactor_vapor_composition();
        let reactor_temperature = self.reactor_temperature();
        let separator_vapor = self.separator_vapor_composition();
        let separator_liquid = self.separator_liquid_composition();
        let separator_temperature = self.separator_temperature();
        let stripper_liquid = self.stripper_liquid_composition();
        let stripper_temperature = self.stripper_temperature();
        let compressor_vapor = self.compressor_vapor_composition();
        let compressor_temperature = self.compressor_temperature();

        let flow = self.stream_flows();
        let flash_vapor_flow = self.flash_vapor_component_flow(); /* FCM(·,5) — nosso slot 4 */
        let flash_liquid_flow = self.flash_liquid_component_flow(); /* FCM(·,12) — nosso slot 11 */

        /* Entalpias das streams — reconstruídas de composição+temperatura já publicadas (Blocks
        20-21), exceto as 3 que só existem dentro de Flows (flash e a correção do compressor).
        */
        let enthalpy_feed_d = mixture_enthalpy(&FEED_D_COMPOSITION, FEED_TEMPERATURE, 1, &constants);
        let enthalpy_feed_e = mixture_enthalpy(&FEED_E_COMPOSITION, FEED_TEMPERATURE, 1, &constants);
        let enthalpy_feed_a = mixture_enthalpy(&FEED_A_COMPOSITION, FEED_TEMPERATURE, 1, &constants);
        let enthalpy_feed_ac = mixture_enthalpy(&FEED_AC_COMPOSITION, FEED_TEMPERATURE, 1, &constants);
        /* slot 5 (recycle do compressor) e slot 6 (bypass, cópia de 5) — mesma entalpia. */
        let enthalpy_compressor_recycle = mixture_enthalpy(&compressor_vapor, compressor_temperature, 1, &constants);
        let enthalpy_reactor_outlet = mixture_enthalpy(&reactor_vapor, reactor_temperature, 1, &constants);
        /* slot 8 SEM a correção de Block 24 (HST(9) antes de += CPDH/FTM(9)) — é o que HST(10)
        preserva no original, por ter sido copiado ANTES da correção rodar.
        */
        let enthalpy_separator_vapor_uncorrected = mixture_enthalpy(&separator_vapor, separator_temperature, 1, &constants);
        let enthalpy_separator_liquid = mixture_enthalpy(&separator_liquid, separator_temperature, 0, &constants);
        let enthalpy_stripper_liquid = mixture_enthalpy(&stripper_liquid, stripper_temperature, 0, &constants);
        /* slot 8 COM a correção — a única forma "corrigida" que existe, publicada por Flows. */
        let compressor_discharge_enthalpy = self.compressor_discharge_enthalpy();

        /* Composição do vapor do flash (slot 4) só existe normalizando FCM — ninguém publica a
        fração em si, só a vazão por componente (que é o que as equações de massa já usam direto).
        Não existe equivalente pro líquido do flash (slot 11): `HST(11)`/`XST(·,11)` são computados
        no original (Block 29-30) mas NUNCA usados em Block 40 — quem entra na energia do stripper
        é `HST(10)` (entalpia do underflow do separador, `enthalpy_separator_liquid`, já calculada
        acima), não uma entalpia própria do líquido do flash.
        */
        let flash_vapor_total: f64 = flash_vapor_flow.iter().sum();
        let mut flash_vapor_composition = [0.0f64; 8];
        if flash_vapor_total > 0.0 {
            for i in 0..8 {
                flash_vapor_composition[i] = flash_vapor_flow[i] / flash_vapor_total;
            }
        }
        let enthalpy_flash_vapor = mixture_enthalpy(&flash_vapor_composition, stripper_temperature, 1, &constants);

        /* ===== Reator — YP(1..8), YP(9): FCM(·,7) - FCM(·,8) + CRXR(·) ===== */
        let reaction_rates = self.reactor_reaction_rates();
        let mut reactor_vapor_derivative = [0.0f64; 3];
        let mut reactor_liquid_derivative = [0.0f64; 5];
        for i in 0..8 {
            let value = compressor_vapor[i] * flow[6] - reactor_vapor[i] * flow[7] + reaction_rates[i];
            if i < 3 {
                reactor_vapor_derivative[i] = value;
            } else {
                reactor_liquid_derivative[i - 3] = value;
            }
        }
        let reactor_enthalpy_derivative = enthalpy_compressor_recycle * flow[6]
            - enthalpy_reactor_outlet * flow[7]
            + self.reactor_heat_of_reaction()
            + self.reactor_heat();

        /* ===== Separador — YP(10..17), YP(18): FCM(·,8) - FCM(·,9) - FCM(·,10) - FCM(·,11) ===== */
        let mut separator_vapor_derivative = [0.0f64; 3];
        let mut separator_liquid_derivative = [0.0f64; 5];
        for i in 0..8 {
            let value = reactor_vapor[i] * flow[7]
                - separator_vapor[i] * flow[8]
                - separator_vapor[i] * flow[9]
                - separator_liquid[i] * flow[10];
            if i < 3 {
                separator_vapor_derivative[i] = value;
            } else {
                separator_liquid_derivative[i - 3] = value;
            }
        }
        let separator_enthalpy_derivative = enthalpy_reactor_outlet * flow[7]
            - compressor_discharge_enthalpy * flow[8]
            - enthalpy_separator_vapor_uncorrected * flow[9]
            - enthalpy_separator_liquid * flow[10]
            + self.separator_heat();

        /* ===== Stripper — YP(19..26), YP(27): FCM(·,12) - FCM(·,13) ===== */
        let mut stripper_liquid_derivative = [0.0f64; 8];
        for i in 0..8 {
            stripper_liquid_derivative[i] = flash_liquid_flow[i] - stripper_liquid[i] * flow[12];
        }
        let stripper_enthalpy_derivative = enthalpy_feed_ac * flow[3] + enthalpy_separator_liquid * flow[10]
            - enthalpy_flash_vapor * flow[4]
            - enthalpy_stripper_liquid * flow[12]
            + self.condenser_heat();

        /* ===== Compressor — YP(28..35), YP(36): FCM(·,1)+FCM(·,2)+FCM(·,3)+FCM(·,5)+FCM(·,9)-FCM(·,6) ===== */
        let mut compressor_vapor_derivative = [0.0f64; 8];
        for i in 0..8 {
            compressor_vapor_derivative[i] = FEED_D_COMPOSITION[i] * flow[0]
                + FEED_E_COMPOSITION[i] * flow[1]
                + FEED_A_COMPOSITION[i] * flow[2]
                + flash_vapor_flow[i]
                + separator_vapor[i] * flow[8]
                - compressor_vapor[i] * flow[5];
        }
        let compressor_enthalpy_derivative = enthalpy_feed_d * flow[0]
            + enthalpy_feed_e * flow[1]
            + enthalpy_feed_a * flow[2]
            + enthalpy_flash_vapor * flow[4]
            + compressor_discharge_enthalpy * flow[8]
            - enthalpy_compressor_recycle * flow[5];

        self.set_reactor_vapor_derivative(reactor_vapor_derivative);
        self.set_reactor_liquid_derivative(reactor_liquid_derivative);
        self.set_reactor_enthalpy_derivative(reactor_enthalpy_derivative);
        self.set_separator_vapor_derivative(separator_vapor_derivative);
        self.set_separator_liquid_derivative(separator_liquid_derivative);
        self.set_separator_enthalpy_derivative(separator_enthalpy_derivative);
        self.set_stripper_liquid_derivative(stripper_liquid_derivative);
        self.set_stripper_enthalpy_derivative(stripper_enthalpy_derivative);
        self.set_compressor_vapor_derivative(compressor_vapor_derivative);
        self.set_compressor_enthalpy_derivative(compressor_enthalpy_derivative);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::{Proxy, StateRegistry};

    /* Handle por grupo de chave que `Derivatives` precisa (~86 `#[need]`s) — mesma ordem dos campos
    do struct. Cada teste seta só o que importa; o resto fica em 0.0 (default do buffer), que é
    seguro aqui porque `mixture_enthalpy` é soma ponderada pura (sem divisão) — composição/vazão
    zeradas não geram NaN, só termos zerados.
    */
    struct Seeded {
        reactor_temperature: Proxy,
        reactor_vapor_composition: [Proxy; 8],
        reactor_reaction_rates: [Proxy; 8],
        reactor_heat_of_reaction: Proxy,
        separator_temperature: Proxy,
        separator_vapor_composition: [Proxy; 8],
        separator_liquid_composition: [Proxy; 8],
        stripper_temperature: Proxy,
        stripper_liquid_composition: [Proxy; 8],
        compressor_temperature: Proxy,
        compressor_vapor_composition: [Proxy; 8],
        stream_flow: [Proxy; 13],
        flash_vapor_component_flow: [Proxy; 8],
        flash_liquid_component_flow: [Proxy; 8],
        compressor_discharge_enthalpy: Proxy,
        reactor_heat: Proxy,
        separator_heat: Proxy,
        condenser_heat: Proxy,
    }

    fn array8(offered: &[Proxy], start: usize) -> [Proxy; 8] {
        std::array::from_fn(|i| offered[start + i].clone())
    }

    fn array13(offered: &[Proxy], start: usize) -> [Proxy; 13] {
        std::array::from_fn(|i| offered[start + i].clone())
    }

    fn seed_registry(registry: &mut StateRegistry) -> Seeded {
        let mut keys: Vec<String> = Vec::new();
        keys.push("reactor.temperature".to_string());
        let reactor_vapor_composition_start = keys.len();
        for c in ["a", "b", "c", "d", "e", "f", "g", "h"] { keys.push(format!("reactor.vapor_composition.{c}")); }
        let reactor_reaction_rates_start = keys.len();
        for c in ["a", "b", "c", "d", "e", "f", "g", "h"] { keys.push(format!("reactor.reaction_rates.{c}")); }
        let reactor_heat_of_reaction_idx = keys.len();
        keys.push("reactor.heat_of_reaction".to_string());

        let separator_temperature_idx = keys.len();
        keys.push("separator.temperature".to_string());
        let separator_vapor_composition_start = keys.len();
        for c in ["a", "b", "c", "d", "e", "f", "g", "h"] { keys.push(format!("separator.vapor_composition.{c}")); }
        let separator_liquid_composition_start = keys.len();
        for c in ["a", "b", "c", "d", "e", "f", "g", "h"] { keys.push(format!("separator.liquid_composition.{c}")); }

        let stripper_temperature_idx = keys.len();
        keys.push("stripper.temperature".to_string());
        let stripper_liquid_composition_start = keys.len();
        for c in ["0", "1", "2", "3", "4", "5", "6", "7"] { keys.push(format!("stripper.liquid_composition.{c}")); }

        let compressor_temperature_idx = keys.len();
        keys.push("compressor.temperature".to_string());
        let compressor_vapor_composition_start = keys.len();
        for c in ["0", "1", "2", "3", "4", "5", "6", "7"] { keys.push(format!("compressor.vapor_composition.{c}")); }

        let stream_flow_start = keys.len();
        for c in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"] { keys.push(format!("flows.stream_flow.{c}")); }
        let flash_vapor_component_flow_start = keys.len();
        for c in ["a", "b", "c", "d", "e", "f", "g", "h"] { keys.push(format!("flows.flash_vapor_component_flow.{c}")); }
        let flash_liquid_component_flow_start = keys.len();
        for c in ["a", "b", "c", "d", "e", "f", "g", "h"] { keys.push(format!("flows.flash_liquid_component_flow.{c}")); }
        let compressor_discharge_enthalpy_idx = keys.len();
        keys.push("flows.compressor_discharge_enthalpy".to_string());

        let reactor_heat_idx = keys.len();
        keys.push("heat.reactor_heat".to_string());
        let separator_heat_idx = keys.len();
        keys.push("heat.separator_heat".to_string());
        let condenser_heat_idx = keys.len();
        keys.push("heat.condenser_heat".to_string());

        let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let (offered, _) = registry.subscribe(&key_refs, &[]);

        Seeded {
            reactor_temperature: offered[0].clone(),
            reactor_vapor_composition: array8(&offered, reactor_vapor_composition_start),
            reactor_reaction_rates: array8(&offered, reactor_reaction_rates_start),
            reactor_heat_of_reaction: offered[reactor_heat_of_reaction_idx].clone(),
            separator_temperature: offered[separator_temperature_idx].clone(),
            separator_vapor_composition: array8(&offered, separator_vapor_composition_start),
            separator_liquid_composition: array8(&offered, separator_liquid_composition_start),
            stripper_temperature: offered[stripper_temperature_idx].clone(),
            stripper_liquid_composition: array8(&offered, stripper_liquid_composition_start),
            compressor_temperature: offered[compressor_temperature_idx].clone(),
            compressor_vapor_composition: array8(&offered, compressor_vapor_composition_start),
            stream_flow: array13(&offered, stream_flow_start),
            flash_vapor_component_flow: array8(&offered, flash_vapor_component_flow_start),
            flash_liquid_component_flow: array8(&offered, flash_liquid_component_flow_start),
            compressor_discharge_enthalpy: offered[compressor_discharge_enthalpy_idx].clone(),
            reactor_heat: offered[reactor_heat_idx].clone(),
            separator_heat: offered[separator_heat_idx].clone(),
            condenser_heat: offered[condenser_heat_idx].clone(),
        }
    }

    fn read_back(registry: &mut StateRegistry, keys: &[&str]) -> Vec<Proxy> {
        let (_, needed) = registry.subscribe(&[], keys);
        registry.resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");
        needed
    }

    #[test]
    fn reactor_mass_derivative_reflects_inflow_and_outflow_terms() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        seeded.compressor_vapor_composition[0].set(2.0); // componente a, entra via flow[6]
        seeded.stream_flow[6].set(3.0);
        seeded.reactor_vapor_composition[3].set(3.0); // componente d, sai via flow[7]
        seeded.stream_flow[7].set(4.0);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(
            &mut registry.borrow_mut(),
            &["reactor.state.vapor_a.derivative", "reactor.state.liquid_d.derivative"],
        );
        assert_eq!(out[0].get(), 2.0 * 3.0, "entrada: compressor_vapor[a]*flow[6], CRXR(a)=0");
        assert_eq!(out[1].get(), -3.0 * 4.0, "saída: -reactor_vapor[d]*flow[7], CRXR(d)=0");
    }

    #[test]
    fn reactor_mass_derivative_includes_reaction_rate_term() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        // flow[6]/flow[7] ficam em 0.0 — isola CRXR puro
        seeded.reactor_reaction_rates[0].set(1.5);
        seeded.reactor_reaction_rates[3].set(-0.7);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(
            &mut registry.borrow_mut(),
            &["reactor.state.vapor_a.derivative", "reactor.state.liquid_d.derivative"],
        );
        assert_eq!(out[0].get(), 1.5);
        assert_eq!(out[1].get(), -0.7);
    }

    #[test]
    fn reactor_energy_derivative_isolates_heat_terms_when_flows_are_zero() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        // flow[6]/flow[7] ficam em 0.0 — zera os dois termos de entalpia*vazão
        seeded.reactor_heat_of_reaction.set(11.0);
        seeded.reactor_heat.set(5.0);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(&mut registry.borrow_mut(), &["reactor.state.enthalpy.derivative"]);
        assert_eq!(out[0].get(), 11.0 + 5.0);
    }

    #[test]
    fn separator_mass_derivative_reflects_reactor_outlet_inflow() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        seeded.reactor_vapor_composition[0].set(5.0); // componente a
        seeded.stream_flow[7].set(2.0);
        // flow[8]/flow[9]/flow[10] ficam em 0.0 — zera os três termos de saída

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(&mut registry.borrow_mut(), &["separator.state.vapor_a.derivative"]);
        assert_eq!(out[0].get(), 5.0 * 2.0);
    }

    #[test]
    fn separator_energy_derivative_isolates_heat_term_when_flows_are_zero() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        // flow[7]/flow[8]/flow[9]/flow[10] ficam em 0.0 — zera os quatro termos de entalpia*vazão
        seeded.separator_heat.set(9.0);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(&mut registry.borrow_mut(), &["separator.state.enthalpy.derivative"]);
        assert_eq!(out[0].get(), 9.0);
    }

    #[test]
    fn stripper_mass_derivative_matches_hand_computed_values() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        seeded.flash_liquid_component_flow[0].set(6.0); // componente 0
        seeded.stripper_liquid_composition[2].set(4.0); // componente 2
        seeded.stream_flow[12].set(1.5);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(
            &mut registry.borrow_mut(),
            &["stripper.state.0.derivative", "stripper.state.2.derivative"],
        );
        assert_eq!(out[0].get(), 6.0, "flash_liquid_flow[0] - stripper_liquid[0]*flow[12] (=0)");
        assert_eq!(out[1].get(), -4.0 * 1.5, "flash_liquid_flow[2] (=0) - stripper_liquid[2]*flow[12]");
    }

    #[test]
    fn stripper_energy_derivative_isolates_heat_term_when_flows_are_zero() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        // flow[3]/flow[4]/flow[10]/flow[12] ficam em 0.0 — zera os quatro termos de entalpia*vazão
        seeded.condenser_heat.set(7.0);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(&mut registry.borrow_mut(), &["stripper.state.8.derivative"]);
        assert_eq!(out[0].get(), 7.0);
    }

    #[test]
    fn compressor_mass_derivative_includes_feed_flash_and_separator_terms() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        let flow_d = 2.0;
        seeded.stream_flow[0].set(flow_d);
        seeded.flash_vapor_component_flow[0].set(7.0); // componente 0
        seeded.separator_vapor_composition[4].set(4.0); // componente 4
        let flow_8 = 2.0;
        seeded.stream_flow[8].set(flow_8);
        // flow[1]/flow[2]/flow[5] ficam em 0.0 — zera feed E, feed A e o termo de recycle

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(
            &mut registry.borrow_mut(),
            &["compressor.state.0.derivative", "compressor.state.4.derivative"],
        );
        assert_eq!(out[0].get(), FEED_D_COMPOSITION[0] * flow_d + 7.0);
        assert_eq!(out[1].get(), FEED_D_COMPOSITION[4] * flow_d + 4.0 * flow_8);
    }

    #[test]
    fn compressor_energy_derivative_is_zero_when_all_flows_are_zero() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());
        seeded.reactor_heat.set(999.0); // não deveria vazar pra cá — só reactor tem termo de calor

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let out = read_back(&mut registry.borrow_mut(), &["compressor.state.8.derivative"]);
        assert_eq!(out[0].get(), 0.0, "sem termo de calor próprio — só soma entalpia*vazão, tudo zerado aqui");
    }

    #[test]
    fn evaluate_does_not_panic_with_realistic_operating_values() {
        let registry = StateRegistry::shared();
        let seeded = seed_registry(&mut registry.borrow_mut());

        seeded.reactor_temperature.set(120.4);
        seeded.reactor_vapor_composition[0].set(1.0);
        for r in &seeded.reactor_reaction_rates { r.set(0.5); }
        seeded.reactor_heat_of_reaction.set(-200.0);

        seeded.separator_temperature.set(80.1);
        seeded.separator_vapor_composition[0].set(1.0);
        seeded.separator_liquid_composition[0].set(1.0);

        seeded.stripper_temperature.set(65.7);
        seeded.stripper_liquid_composition[0].set(1.0);

        seeded.compressor_temperature.set(95.3);
        seeded.compressor_vapor_composition[0].set(1.0);

        for flow in &seeded.stream_flow { flow.set(20.0); }
        seeded.flash_vapor_component_flow[0].set(15.0);
        seeded.flash_liquid_component_flow[0].set(10.0);
        seeded.compressor_discharge_enthalpy.set(500.0);

        seeded.reactor_heat.set(100.0);
        seeded.separator_heat.set(-50.0);
        seeded.condenser_heat.set(30.0);

        let config = Snapshot::from_pairs(&[]);
        let derivatives = Derivatives::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        derivatives.evaluate();

        let mut keys: Vec<String> = vec![
            "reactor.state.vapor_a.derivative".into(),
            "reactor.state.vapor_b.derivative".into(),
            "reactor.state.vapor_c.derivative".into(),
            "reactor.state.liquid_d.derivative".into(),
            "reactor.state.liquid_e.derivative".into(),
            "reactor.state.liquid_f.derivative".into(),
            "reactor.state.liquid_g.derivative".into(),
            "reactor.state.liquid_h.derivative".into(),
            "reactor.state.enthalpy.derivative".into(),
            "separator.state.vapor_a.derivative".into(),
            "separator.state.vapor_b.derivative".into(),
            "separator.state.vapor_c.derivative".into(),
            "separator.state.liquid_d.derivative".into(),
            "separator.state.liquid_e.derivative".into(),
            "separator.state.liquid_f.derivative".into(),
            "separator.state.liquid_g.derivative".into(),
            "separator.state.liquid_h.derivative".into(),
            "separator.state.enthalpy.derivative".into(),
        ];
        for c in ["0", "1", "2", "3", "4", "5", "6", "7"] { keys.push(format!("stripper.state.{c}.derivative")); }
        keys.push("stripper.state.8.derivative".to_string());
        for c in ["0", "1", "2", "3", "4", "5", "6", "7"] { keys.push(format!("compressor.state.{c}.derivative")); }
        keys.push("compressor.state.8.derivative".to_string());

        let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let out = read_back(&mut registry.borrow_mut(), &key_refs);
        for proxy in &out {
            assert!(proxy.get().is_finite(), "derivada não pode ser NaN/Inf com valores realistas de operação");
        }
    }
}
