/* tep/dynamics/flows.rs */


use crate::dynamics::feed::FEED_AC_COMPOSITION;
use crate::physics::constants::TepConstants;
use monjolo::chemistry::mixture_enthalpy;

/* Vazão máxima de cada válvula com curva linear (posição% * range / 100) — VRNG em TEINIT,
indexado pela MESMA identidade física da válvula (não pelo número XMV, que difere do slot interno).
Feed D/E/A/AC saíram daqui — ver `dynamics::feed` (issue 10). Compressor (slots 5/6/8, curva
característica) e Separator (slots 9/10, purge+underflow) também saíram — ver
`dynamics::compressor`/`dynamics::separator`.
*/
const STRIPPER_PRODUCT_RANGE: f64 = 1000.0;
const STRIPPER_STEAM_RANGE: f64 = 0.03;

/* Sem `after` declarado: Flows não depende mais de nada do Compressor (issue 10 — temperatura/
pressão/composição/vazões de recycle saíram todos pra `dynamics::compressor`) — a ordem correta
(depois de Reactor/Separator/Stripper/Feed, únicos donos reais de quem Flows ainda precisa) sai
inteira do casamento automático de `needs`/`offers` (`monjolo::component::sort_phase_a`).
*/
#[monjolo::dynamic_model]
pub struct Flows {
    #[need(key = "reactor.temperature")]
    reactor_temperature: f64,
    #[need(key = "reactor.pressure")]
    reactor_pressure: f64,
    #[need(prefix = "reactor.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    reactor_vapor_composition: [f64; 8],

    #[need(key = "separator.temperature")]
    separator_temperature: f64,
    #[need(key = "separator.pressure")]
    separator_pressure: f64,
    #[need(prefix = "separator.vapor_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    separator_vapor_composition: [f64; 8],
    #[need(prefix = "separator.liquid_composition", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    separator_liquid_composition: [f64; 8],

    #[need(key = "stripper.temperature")]
    stripper_temperature: f64,
    #[need(prefix = "stripper.liquid_composition", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    stripper_liquid_composition: [f64; 8],

    /* D/E/A vazões (slots 0-2) não são mais lidas aqui — publicadas por `Feed` e consumidas direto
    por quem precisar delas (ninguém, hoje, dentro de Flows: eram só repassadas pra `stream_flows`,
    que `Feed` agora oferece nesses índices). Só o slot 3 (A&C) é genuinamente consumido aqui, pelo
    split do flash (Block 25 abaixo). Compressor (slots 5/6/8, temperatura/pressão/composição do
    próprio compressor) saiu pelo mesmo motivo — ver `dynamics::compressor`.
    */
    #[need(key = "flows.stream_flow.3")]
    ac_feed_flow: f64,
    /* Slot 10 (underflow) precisa ser LIDO de volta (não mais calculado aqui) — o split do flash
    (Block 25/26 abaixo) genuinamente consome essa vazão; `Separator` é quem agora a computa a
    partir da própria válvula (`dynamics::separator::outlet_flows`). Slot 9 (purge) some sem
    substituto — nada aqui consumia de volta, só repassava pra `stream_flows`.
    */
    #[need(key = "flows.stream_flow.10")]
    underflow_flow: f64,
    #[need(key = "valve.stripper_product.position")]
    stripper_product_position: f64,
    #[need(key = "valve.stripper_steam.position")]
    stripper_steam_position: f64,
    #[need(key = "agitator.speed")]
    agitator_speed: f64,

    /* Vazão molar dos slots 4/7/11/12 (kmol/h) — ver mapeamento no topo do arquivo. Slots 0-3
    (feeds) são ofertados por `Feed`; slots 5/6/8 (compressor) por `Compressor`; slots 9/10
    (purge/underflow) por `Separator` — mesma chave (`flows.stream_flow.N`), dono diferente.
    */
    #[offer(prefix = "flows.stream_flow", components = ["4", "7", "11", "12"])]
    stream_flows: [f64; 4],

    #[offer(key = "flows.condenser_ua")]
    condenser_ua: f64,
    #[offer(key = "flows.agitation_factor")]
    agitation_factor: f64,

    /* Os únicos 3 valores do flash (Blocks 26-29) e da correção de entalpia do compressor
    (Block 24) que ninguém publicava — precisos pro balanço de massa/energia real
    (`dynamics::derivatives`, `after = ["Heat"]`, roda depois daqui). O resto do que o balanço
    precisa (composição/temperatura de cada unidade, `flows.stream_flow.*`, `heat.*`) já estava
    publicado; só o flash (component_flow_4/11 — FCM(·,5)/FCM(·,12) no original) era computado
    aqui e descartado. A entalpia corrigida do compressor (Block 24) saiu pra `Compressor` — ver
    `dynamics::compressor`.
    */
    #[offer(prefix = "flows.flash_vapor_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    flash_vapor_component_flow: [f64; 8],
    #[offer(prefix = "flows.flash_liquid_component_flow", components = ["a", "b", "c", "d", "e", "f", "g", "h"])]
    flash_liquid_component_flow: [f64; 8],
}

impl Flows {
    fn compute(&self) {
        let reactor_vapor = self.reactor_vapor_composition();
        let separator_vapor = self.separator_vapor_composition();
        let separator_liquid = self.separator_liquid_composition();
        let stripper_liquid = self.stripper_liquid_composition();

        let reactor_temperature = self.reactor_temperature();
        let reactor_pressure = self.reactor_pressure();
        let separator_temperature = self.separator_temperature();
        let separator_pressure = self.separator_pressure();
        let stripper_temperature = self.stripper_temperature();

        /* Block 19: composições dos slots vindos direto de estado de unidade (cópias, sem cálculo).
        Slots 0-2 (feeds D/E/A) e 5 (compressor) somem daqui — `Feed`/`Compressor` são quem tem
        essas composições agora, e nada neste arquivo consome de volta a composição deles (só a
        vazão, via `ac_feed_flow` pro slot 3, que É consumido abaixo).
        */
        let mut composition = [[0.0f64; 8]; 13];
        composition[3] = FEED_AC_COMPOSITION;
        composition[7] = reactor_vapor;
        composition[8] = separator_vapor;
        composition[10] = separator_liquid;
        composition[12] = stripper_liquid;

        let constants = TepConstants::new();

        /* Block 19 (cont.): pesos moleculares médios — só o slot que o resto do bloco precisa pra
        si mesmo (7). Slots 0/1 (D/E) são responsabilidade de `Feed`; slots 5/8 (compressor) e 9
        (purge), de `Compressor`/`Separator` (cada um recalcula seu próprio mw em `outlet_flows`,
        sem depender de nada publicado aqui).
        */
        let mol_weight = |z: &[f64; 8]| -> f64 { (0..8).map(|i| z[i] * constants.xmw[i]).sum() };
        let mw = [
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            mol_weight(&composition[7]),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];

        /* Block 20: temperaturas dos slots. Slots 0-3 (feeds) e 5 (compressor) somem daqui pelo
        mesmo motivo das composições acima — nada aqui consome `temperature[0..=3]`/`[5]`.
        */
        let mut temperature = [0.0f64; 13];
        temperature[7] = reactor_temperature;
        temperature[8] = separator_temperature;
        temperature[10] = separator_temperature;
        temperature[12] = stripper_temperature;

        /* Block 21: entalpias dos slots (ity: 1=vapor, 0=líquido) — só as usadas depois (slot 8,
        pra correção de calor de compressão) precisam ficar em variável nomeada; o resto é
        calculado mas descartado (fiel ao original, que também calcula e não usa todas).
        */
        let mut enthalpy = [0.0f64; 13];
        for &slot in &[0usize, 1, 2, 3, 5, 7, 8] {
            enthalpy[slot] = mixture_enthalpy(&composition[slot], temperature[slot], 1, &constants);
        }
        enthalpy[9] = enthalpy[8];
        enthalpy[10] = mixture_enthalpy(&composition[10], temperature[10], 0, &constants);
        enthalpy[12] = mixture_enthalpy(&composition[12], temperature[12], 0, &constants);

        /* Block 22: vazões — válvulas lineares (posição * range / 100) + UAC + FWR/FWS + AGSP.
        Disturbance channels 8 (UAC) e 11 (ΔP reator→separador) ficam no valor neutro (0.0). Slots
        0-2 (feeds D/E/A) somem daqui — publicados por `Feed`, não consumidos de volta por ninguém
        neste arquivo. Só o slot 3 (A&C) é lido de volta, via `#[need]`, pro split do flash abaixo.
        */
        let mut flow = [0.0f64; 13];
        flow[3] = self.ac_feed_flow();

        let condenser_ua = self.stripper_steam_position() * STRIPPER_STEAM_RANGE * (1.0 + 0.0) / 100.0;
        let agitation_factor = (self.agitator_speed() + 150.0) / 100.0;

        /* Block 23: fluxos dependentes de ΔP (sem válvula). Slot 5 (compressor↔reator) saiu pra
        `Compressor` — ver `dynamics::compressor::outlet_flows`. Slot 10 (underflow) é LIDO de
        volta, não mais calculado — `Separator` é quem publica agora, mas o split do flash abaixo
        continua precisando do valor.
        */
        flow[7] = 4574.21 * (reactor_pressure - separator_pressure).max(0.0).sqrt() * (1.0 - 0.25 * 0.0) / mw[7]; /* disturbance channel 11, neutro */
        flow[10] = self.underflow_flow();
        flow[12] = self.stripper_product_position() * STRIPPER_PRODUCT_RANGE / 100.0;

        /* Block 24 (curva característica + anti-surge do compressor, slot 8) e o cálculo de purge
        (slot 9) saíram inteiros pra `Compressor`/`Separator` — ver
        `dynamics::compressor::outlet_flows`/`dynamics::separator::outlet_flows`.
        */

        /* Block 25: vazão por componente, slot a slot (só os que o split de Block 26-29 usa como
        entrada: slots 3 e 10 — A&C feed e underflow do separador).
        */
        let mut component_flow_3 = [0.0f64; 8];
        let mut component_flow_10 = [0.0f64; 8];
        for i in 0..8 {
            component_flow_3[i] = composition[3][i] * flow[3];
            component_flow_10[i] = composition[10][i] * flow[10];
        }

        /* Block 26: fração de split vapor/líquido do stripper — dependente de temperatura só pros
        componentes D-H (índices 3-7, calculados abaixo). A/B/C (índices 0-2) usam os valores FIXOS
        de TEINIT no teprob.f original (SFR(1)=0.995, SFR(2)=0.991, SFR(3)=0.990) — nunca
        recalculados depois da inicialização, constantes a simulação inteira. Sem isso, A/B/C
        ficavam com fração de vapor 0.0 (default do array) em vez de ~99% — o A&C feed (100% A+B+C)
        caía inteiro no lado líquido do flash, inundando o stripper (achado comparando contra
        teprob.f/docs/fortran-original após o nível do stripper disparar em produção).
        */
        let mut split_fraction = [0.995, 0.991, 0.990, 0.0, 0.0, 0.0, 0.0, 0.0];
        if flow[10] > 0.1 {
            let temperature_factor = if stripper_temperature > 170.0 {
                stripper_temperature - 120.262
            } else if stripper_temperature < 5.292 {
                0.1
            } else {
                363.744 / (177.0 - stripper_temperature) - 2.22579488
            };
            let vapor_over_liquid = flow[3] / flow[10] * temperature_factor;
            split_fraction[3] = 8.5010 * vapor_over_liquid / (1.0 + 8.5010 * vapor_over_liquid);
            split_fraction[4] = 11.402 * vapor_over_liquid / (1.0 + 11.402 * vapor_over_liquid);
            split_fraction[5] = 11.795 * vapor_over_liquid / (1.0 + 11.795 * vapor_over_liquid);
            split_fraction[6] = 0.0480 * vapor_over_liquid / (1.0 + 0.0480 * vapor_over_liquid);
            split_fraction[7] = 0.0242 * vapor_over_liquid / (1.0 + 0.0242 * vapor_over_liquid);
        } else {
            split_fraction[3] = 0.9999;
            split_fraction[4] = 0.999;
            split_fraction[5] = 0.999;
            split_fraction[6] = 0.99;
            split_fraction[7] = 0.98;
        }

        /* Block 27: entrada combinada do flash (A&C feed + underflow do separador). */
        let mut flash_inlet = [0.0f64; 8];
        for i in 0..8 {
            flash_inlet[i] = component_flow_3[i] + component_flow_10[i];
        }

        /* Block 28: split vapor (slot 4)/líquido (slot 11) do flash. */
        let mut component_flow_4 = [0.0f64; 8];
        let mut component_flow_11 = [0.0f64; 8];
        for i in 0..8 {
            component_flow_4[i] = split_fraction[i] * flash_inlet[i];
            component_flow_11[i] = flash_inlet[i] - component_flow_4[i];
            flow[4] += component_flow_4[i];
            flow[11] += component_flow_11[i];
        }

        /* Block 29-30 (composição/entalpia dos slots 4/11 do flash como frações) fica de fora de
        propósito — `dynamics::derivatives` usa `component_flow_4`/`component_flow_11` (FCM, não
        XST) direto nas equações de balanço, sem precisar normalizar em composição aqui; só a
        VAZÃO (`flow[4]`/`flow[11]`) entra em `stream_flows`.
        */

        /* Block 31 (bypass, slot 6 = cópia do slot 5) saiu junto com o resto do compressor — ver
        `dynamics::compressor::outlet_flows`.
        */
        self.set_stream_flows([flow[4], flow[7], flow[11], flow[12]]);
        self.set_condenser_ua(condenser_ua);
        self.set_agitation_factor(agitation_factor);
        self.set_flash_vapor_component_flow(component_flow_4);
        self.set_flash_liquid_component_flow(component_flow_11);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monjolo::dynamic_model::DynamicModel;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    /* Oferece manualmente as chaves que Flows precisa (o que Reactor/Separator/Stripper/Feed/
    atuadores normalmente ofertam sozinhos) — vapor de reactor/separator fica em A puro (100%) só
    pra evitar divisão por zero no peso molecular (mw[7] depende da composição do reator); os
    valores de interesse do teste (vazão A&C, underflow, pressões) são passados explicitamente.
    Compressor/Separator saíram inteiros das dependências de Flows (issue 10) — nenhuma chave de
    válvula deles é seedada aqui mais. `flows.stream_flow.3`/`.10` são seedadas direto aqui (não
    via `Feed`/`Separator`, que não fazem parte deste teste isolado) — mesmas chaves que eles
    ofertariam em produção.
    */
    fn seed_registry(
        registry: &mut StateRegistry,
        reactor_pressure: f64,
        separator_pressure: f64,
        ac_feed_flow: f64,
        underflow_flow: f64,
        agitator_speed: f64,
        stripper_steam_position: f64,
    ) {
        let mut keys = vec![
            "reactor.temperature", "reactor.pressure",
            "separator.temperature", "separator.pressure",
            "stripper.temperature",
            "flows.stream_flow.3", "flows.stream_flow.10",
            "valve.stripper_product.position",
            "valve.stripper_steam.position", "agitator.speed",
        ];
        let mut composition_keys = Vec::new();
        for prefix in ["reactor.vapor_composition", "separator.vapor_composition", "separator.liquid_composition"] {
            for c in ["a", "b", "c", "d", "e", "f", "g", "h"] {
                composition_keys.push(format!("{prefix}.{c}"));
            }
        }
        for c in ["0", "1", "2", "3", "4", "5", "6", "7"] {
            composition_keys.push(format!("stripper.liquid_composition.{c}"));
        }
        keys.extend(composition_keys.iter().map(String::as_str));

        let (offered, _) = registry.subscribe(&keys, &[]);
        offered[1].set(reactor_pressure);
        offered[3].set(separator_pressure);
        offered[5].set(ac_feed_flow);
        offered[6].set(underflow_flow);
        offered[8].set(stripper_steam_position);
        offered[9].set(agitator_speed);

        /* composition_keys, na ordem em que foram empurradas acima (8 cada):
        [10..18) reactor.vapor, [18..26) separator.vapor, [26..34) separator.liquid,
        [34..42) stripper.liquid — campo ".a" (primeiro de cada grupo) = 100% componente A, resto
        0, só pra mw[7] não dar zero.
        */
        let composition_start = 10;
        offered[composition_start].set(1.0); // reactor.vapor_composition.a
        offered[composition_start + 8].set(1.0); // separator.vapor_composition.a
    }

    #[test]
    fn agitation_and_condenser_match_hand_computed_values() {
        let registry = StateRegistry::shared();
        seed_registry(
            &mut registry.borrow_mut(),
            750.0, // reactor_pressure
            700.0, // separator_pressure
            1e-10, // ac_feed_flow (válvula fechada — mesmo epsilon nominal de Feed::ac_feed_flow)
            0.0,   // underflow_flow (válvula fechada)
            50.0,  // agitator_speed
            50.0,  // stripper_steam_position
        );
        let config = Snapshot::from_pairs(&[]);
        let flows = Flows::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        flows.evaluate();

        let (_, needed) = registry
            .borrow_mut()
            .subscribe(&[], &["flows.agitation_factor", "flows.condenser_ua"]);
        registry.borrow_mut().resolve().expect("chaves já ofertadas deveriam resolver de novo sem erro");

        assert_eq!(needed[0].get(), (50.0 + 150.0) / 100.0, "agitation_factor: (posição+150)/100");
        assert_eq!(
            needed[1].get(),
            50.0 * STRIPPER_STEAM_RANGE / 100.0,
            "condenser_ua: posição*range/100 (disturbance neutro)",
        );
    }

    #[test]
    fn evaluate_does_not_panic_with_realistic_pressures() {
        let registry = StateRegistry::shared();
        seed_registry(&mut registry.borrow_mut(), 2705.0, 2633.7, 1e-10, 1332.5, 22.1, 47.44);
        let config = Snapshot::from_pairs(&[]);
        let flows = Flows::new(&mut registry.borrow_mut(), &config);
        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        flows.evaluate(); // pressões/posições de operação normal (docs/07-controle.md) — sem NaN/panic
    }
}
