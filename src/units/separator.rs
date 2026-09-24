/* tep/units/separator.rs */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use monjolo::chemistry::{liquid_density, temperature_from_enthalpy, Mixture, Phase};

const SEPARATOR_VOLUME: f64 = 3500.0; /* volume total do separador vapor/líquido [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
const SEPARATOR_UNDERFLOW_RANGE: f64 = 1500.0; /* VRNG (TEINIT) da válvula de underflow */

/* Temperatura de RETORNO da água de resfriamento do separador (`tws`) — diferente do reator
(`Reactor::heat_exchange`), aqui é mesmo uma constante no teprob.f original: Block 40 do modelo
monolítico (`v1.0.0`) marca a derivada de `tws` como sempre zero ("tws kept at snapshot value"),
então não existe balanço de calor quase-estático nenhum pra resolver aqui — só o valor congelado
que o snapshot inicial semeou.

NOTA (2026-09-15): antes desta correção, esta constante era 40.0 — o `s_zero` do canal de
distúrbio 5 (TCWS, temperatura de ENTRADA da água de resfriamento do separador), não `tws`. Mesmo
erro de troca entrada/saída do bug já corrigido em `Reactor::heat_exchange`, achado ao comparar
contra `application.toml`/`te_exp3_snapshot.toml`: `[state.cooling].separator_water_temp =
77.29698353` é o valor congelado de verdade. Consequência: o separador resfriava mais forte que o
correto, empurrando entalpia fria pro reciclo do compressor — um dos dois termos dominantes do
balanço de energia do reator (`Reactor::mass_and_energy_balance`) — contribuindo pro colapso rápido
de temperatura do reator observado ao vivo (120°C → ~42°C em ~21 min simulados) mesmo já com a
correção do reator sozinha aplicada.
*/
const SEPARATOR_COOLING_WATER_RETURN: f64 = 77.29698353;

/** Terceira unidade migrada pro scheduler de dataflow topológico (issue 10), depois de Feed e
Compressor. Absorve de `flows.rs`: Block 22/25 (slots 9/10, purge e underflow). De `heat.rs`:
Block 33 (troca térmica do separador). De `derivatives.rs`: a seção "Separador" do balanço de
massa/energia (Block 40, YP(10..18)). De `purge_analyzer.rs`: XMEAS 29-36 (Purge Gas Analysis).
*/
#[monjolo::dynamic_model(tasks)]
pub struct Separator {
    /* Estado próprio (9 números) — mesmo split de Reactor, mesmo motivo (chave de config não
    uniforme entre vapor/líquido e entalpia).
    */
    #[state]
    #[config(prefix = "state.separator_vapor", components = ["A", "B", "C"])]
    #[offer(prefix = "separator.state", components = ["vapor_a", "vapor_b", "vapor_c"])]
    vapor: [f64; 3],

    #[state]
    #[config(prefix = "state.separator_vapor", components = ["D", "E", "F", "G", "H"])]
    #[offer(prefix = "separator.state", components = ["liquid_d", "liquid_e", "liquid_f", "liquid_g", "liquid_h"])]
    liquid: [f64; 5],

    #[state]
    #[config(key = "state.separator.energy")]
    #[offer(key = "separator.state.enthalpy")]
    enthalpy: f64,

    constants: TepConstants,
}

#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Separator {
    /* Bloco 1: balanço de energia próprio → temperatura/pressão/composição/volume/densidade —
    igual ao `compute()` monolítico de antes, agora uma tarefa entre várias.
    */
    #[task]
    fn physical_state(&self) {
        let vapor = Mixture::at(0, &self.vapor(), Phase::Vapor, &TEP_SPECIES);
        let liquid = Mixture::at(3, &self.liquid(), Phase::Liquid, &TEP_SPECIES);
        let liquid_composition = liquid.mole_fractions();

        let specific_enthalpy = self.enthalpy() / liquid.total();
        let temperature = temperature_from_enthalpy(&liquid_composition.as_array(), need::reactor__temperature, specific_enthalpy, 0, &self.constants);
        let temperature_k = temperature + 273.15;
        let density = liquid_density(&liquid_composition.as_array(), temperature, &self.constants);
        let volume_liquid = liquid.total() / density;
        let volume_vapor = SEPARATOR_VOLUME - volume_liquid;

        /* A/B/C: gás ideal a partir dos moles de vapor; D-H: Antoine × fração líquida. */
        let partial_pressures = vapor.ideal_gas_pressure(temperature_k, volume_vapor, GAS_CONSTANT)
            + liquid_composition.vapor_pressure(temperature, &self.constants);
        let pressure = partial_pressures.total();
        let vapor_composition = partial_pressures.mole_fractions();

        offer::separator__temperature = temperature;
        offer::separator__pressure = pressure;
        offer::separator__liquid_volume = volume_liquid;
        offer::separator__liquid_density = density;
        offer::separator__liquid_composition::<Liquid> = liquid_composition;
        offer::separator__vapor_composition::<Vapor> = vapor_composition;
    }

    /* Bloco 2 (ex-Flows, Block 22/25): purge (slot 9, dependente de pressão+composição próprias) e
    underflow (slot 10, puramente linear na válvula — sem acoplamento nenhum, mas fica junto por
    ser a outra saída direta do vaso).
    */
    #[task]
    fn outlet_flows(&self) {
        let mol_weight = need::separator__vapor_composition::<Vapor>.dot(&self.constants.xmw);
        offer::flows__stream_flow__9 = need::valve__purge__position * 0.151169 * (need::separator__pressure - 760.0).max(0.0).sqrt() / mol_weight;
        offer::flows__stream_flow__10 = need::valve__separator_underflow__position * SEPARATOR_UNDERFLOW_RANGE / 100.0;
    }

    /* Bloco 3 (ex-Heat, Block 33): troca térmica no separador — UAS depende da vazão reator→
    separador; a temperatura de referência é a do REATOR (não do separador — TST(8) aponta pro
    reator no teprob.f, Block 20), preservado por fidelidade.
    */
    #[task]
    fn heat_exchange(&self) {
        let uas = 0.404655 * (1.0 - 1.0 / (1.0 + (need::flows__stream_flow__7 / 3528.73).powi(4)));
        offer::heat__separator_heat = uas * (SEPARATOR_COOLING_WATER_RETURN - need::reactor__temperature) * (1.0 - 0.25 * 0.0);
        offer::heat__separator_cooling_water_return = SEPARATOR_COOLING_WATER_RETURN;
    }

    /* Bloco 4 (ex-Derivatives, Block 40 YP(10..18)): balanço de massa/energia do próprio estado.
    `enthalpy_separator_liquid` é recomputada aqui (não lida de volta) — mesmo padrão já usado em
    `derivatives.rs` pras entalpias de feed: quem precisa recalcula fresco a partir de composição+
    temperatura já publicadas, em vez de depender de mais uma chave.
    */
    #[task]
    fn mass_and_energy_balance(&self) {
        let reactor_vapor = need::reactor__vapor_composition::<Vapor>;
        let separator_vapor = need::separator__vapor_composition::<Vapor>;
        let separator_liquid = need::separator__liquid_composition::<Liquid>;
        let flow7 = need::flows__stream_flow__7;
        let flow8 = need::flows__stream_flow__8;
        let flow9 = need::flows__stream_flow__9;
        let flow10 = need::flows__stream_flow__10;
        let separator_temperature = need::separator__temperature;

        let enthalpy_reactor_outlet = reactor_vapor.enthalpy(need::reactor__temperature, 1, &self.constants);
        /* SEM a correção de Block 24 (compressor) — é o que HST(10) preserva no original, por ter
        sido copiado ANTES da correção rodar.
        */
        let enthalpy_separator_vapor_uncorrected = separator_vapor.enthalpy(separator_temperature, 1, &self.constants);
        let enthalpy_separator_liquid = separator_liquid.enthalpy(separator_temperature, 0, &self.constants);

        let derivative = reactor_vapor.scaled_by(flow7)
            - separator_vapor.scaled_by(flow8)
            - separator_vapor.scaled_by(flow9)
            - separator_liquid.scaled_by(flow10);

        offer::separator__state__vapor_a__derivative = derivative.component(0);
        offer::separator__state__vapor_b__derivative = derivative.component(1);
        offer::separator__state__vapor_c__derivative = derivative.component(2);
        offer::separator__state__liquid_d__derivative = derivative.component(3);
        offer::separator__state__liquid_e__derivative = derivative.component(4);
        offer::separator__state__liquid_f__derivative = derivative.component(5);
        offer::separator__state__liquid_g__derivative = derivative.component(6);
        offer::separator__state__liquid_h__derivative = derivative.component(7);
        offer::separator__state__enthalpy__derivative = enthalpy_reactor_outlet * flow7
            - need::flows__compressor_discharge_enthalpy * flow8
            - enthalpy_separator_vapor_uncorrected * flow9
            - enthalpy_separator_liquid * flow10
            + need::heat__separator_heat;
    }

    /* Bloco 5 (ex-purge_analyzer.rs): XMEAS 29-36, Purge Gas Analysis (Stream 9) — a mesma
    composição de vapor que alimenta o recycle na stream 8 (Block 27 de teprob.f, `FCM(I,9)`/
    `FCM(I,8)` usam o mesmo `XST(.,9)=XST(.,8)`), convertida de fração molar pra mol%.
    */
    #[task]
    fn purge_analysis(&self) {
        offer::xmeas__stream9__component::<Vapor> = need::separator__vapor_composition::<Vapor>.scaled_by(100.0);
    }

    /* Bloco 6 (ex-measured.rs, Block 35): XMEAS 10 (Purge Rate, stream9), 11-13 (temperatura/
    nível/pressão do separador), 14 (Separator Underflow, stream10), 22 (temperatura de saída da
    água de resfriamento) — conversões preservadas exatamente do original.
    */
    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__stream9__flow_rate = need::flows__stream_flow__9 * 0.359 / 35.3145;
        offer::xmeas__separator__temperature = need::separator__temperature;
        offer::xmeas__separator__level = (need::separator__liquid_volume - 27.5) / 290.0 * 100.0;
        offer::xmeas__separator__pressure = (need::separator__pressure - 760.0) / 760.0 * 101.325;
        offer::xmeas__stream10__flow_rate = need::flows__stream_flow__10 / need::separator__liquid_density / 35.3145;
        offer::xmeas__separator__cooling_water_outlet_temperature = need::heat__separator_cooling_water_return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::harness::Harness;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    #[test]
    fn new_seeds_own_state_with_initial_condition() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[
            ("state.separator_vapor.A", 1.0),
            ("state.separator_vapor.B", 2.0),
            ("state.separator_vapor.C", 3.0),
            ("state.separator_vapor.D", 4.0),
            ("state.separator_vapor.E", 5.0),
            ("state.separator_vapor.F", 6.0),
            ("state.separator_vapor.G", 7.0),
            ("state.separator_vapor.H", 8.0),
            ("state.separator.energy", 42.0),
        ]);

        let separator = Separator::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(separator.vapor(), [1.0, 2.0, 3.0]);
        assert_eq!(separator.liquid(), [4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(separator.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let separator = Separator::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(separator.vapor(), [0.0; 3]);
        assert_eq!(separator.liquid(), [0.0; 5]);
        assert_eq!(separator.enthalpy(), 0.0);
    }

    /* Investigação do Experimento 20 (spec-tennessee-eastman/experimentos.md) — mesma técnica dos
    testes equivalentes de `reactor.rs`/`compressor.rs`: rodar a tarefa isolada, com entradas
    conhecidas semeadas por chave, sem precisar da planta inteira.
    */
    #[test]
    fn physical_state_matches_nominal_operating_point_from_application_toml() {
        let initial = Snapshot::from_pairs(&[
            ("state.separator_vapor.A", 63.337045809835196),
            ("state.separator_vapor.B", 27.67808577797886),
            ("state.separator_vapor.C", 45.480330241132435),
            ("state.separator_vapor.D", 0.2398728094022691),
            ("state.separator_vapor.E", 14.845882843614561),
            ("state.separator_vapor.F", 1.9220259408956704),
            ("state.separator_vapor.G", 52.521987477541764),
            ("state.separator_vapor.H", 41.289131421711694),
            ("state.separator.energy", 0.571326790156296),
        ]);

        let harness = Harness::new();
        let _separator = harness.unit(|registry| Separator::new(registry, &initial));
        harness.seed("reactor.temperature", 120.0); /* reactor.temperature nominal */
        let task = harness.task("Separator::physical_state");
        harness.resolve();
        task.evaluate();

        let temperature = harness.read("separator.temperature");
        let pressure = harness.read("separator.pressure");

        /* Faixas com folga generosa (mesmo espírito de reactor.rs/compressor.rs) — pegar um
        colapso grosseiro, não validar casas decimais de um nominal que este arquivo não
        documentava antes.
        */
        assert!(
            (60.0..100.0).contains(&temperature),
            "esperava separator.temperature numa faixa plausível (mais fria que o reator, ~120°C), obteve {temperature}°C"
        );

        let xmeas_pressure = (pressure - 760.0) / 760.0 * 101.325;
        assert!(
            (2400.0..2800.0).contains(&xmeas_pressure),
            "esperava XMEAS(13) perto do nominal ~2633 kPa, obteve {xmeas_pressure} kPa"
        );
    }

    /* Regressão direta do bug do Exp 18: `heat_exchange` sempre publica
    `SEPARATOR_COOLING_WATER_RETURN` como temperatura de retorno (é uma constante congelada, não
    calculada) — este teste existe só pra travar o valor certo (77.29698353, `[state.cooling].
    separator_water_temp` de `application.toml`) contra uma futura regressão de volta pro 40.0
    errado (`s_zero` do canal de distúrbio 5, TCWS — entrada, não retorno).
    */
    #[test]
    fn heat_exchange_returns_the_corrected_frozen_return_temperature() {
        let harness = Harness::new();
        let _separator = harness.unit(|registry| Separator::new(registry, &Snapshot::from_pairs(&[])));
        harness.seed("reactor.temperature", 120.0);
        harness.seed("flows.stream_flow.7", 4000.0);
        let task = harness.task("Separator::heat_exchange");
        harness.resolve();
        task.evaluate();

        assert_eq!(
            harness.read("heat.separator_cooling_water_return"),
            77.29698353,
            "regressão pro bug do Exp 18 — TCWS de entrada no lugar do tws de retorno"
        );
    }

    /* Semeia todas as entradas de `mass_and_energy_balance` com fluxos BALANCEADOS: o vapor do
    reator entrando (`flow7`) e saindo em duas frações que somam exatamente o que entrou
    (`flow8+flow9`, mesma composição, `flow10=0`) — nada deveria se acumular.
    */
    fn run_balanced_mass_and_energy_balance(separator_heat: f64) -> Harness {
        let harness = Harness::new();
        let _separator = harness.unit(|registry| Separator::new(registry, &Snapshot::from_pairs(&[])));

        let composition = [0.1, 0.05, 0.1, 0.05, 0.2, 0.1, 0.2, 0.2];
        let temperature = 100.0;
        let enthalpy = Mixture::new(composition, Phase::Vapor, &TEP_SPECIES).enthalpy(temperature, 1, &TepConstants::new());

        harness.seed_mixture("reactor.vapor_composition", &composition);
        harness.seed("reactor.temperature", temperature);
        harness.seed("flows.stream_flow.7", 100.0); /* flow7: entrando */
        harness.seed("flows.stream_flow.8", 60.0); /* flow8: saindo (reciclo) */
        harness.seed("flows.stream_flow.9", 40.0); /* flow9: saindo (purge) — 60+40 = 100, balanceado */
        harness.seed("flows.stream_flow.10", 0.0); /* flow10: sem underflow líquido */
        harness.seed_mixture("separator.vapor_composition", &composition); /* mesma composição */
        harness.seed_mixture("separator.liquid_composition", &[0.0; 8]); /* sem líquido (flow10=0 de qualquer forma) */
        harness.seed("separator.temperature", temperature); /* mesma do reator, pra enthalpy_reactor_outlet == enthalpy_separator_vapor_uncorrected */
        harness.seed("flows.compressor_discharge_enthalpy", enthalpy); /* igual à entalpia da mesma composição/temperatura, cancela com flow8 */
        harness.seed("heat.separator_heat", separator_heat);

        let task = harness.task("Separator::mass_and_energy_balance");
        harness.resolve();
        task.evaluate();
        harness
    }

    /* `mass_and_energy_balance`: com o vapor do reator entrando e saindo em duas frações que somam
    exatamente o que entrou, nada deveria se acumular — mesma técnica de "fluxo balanceado" de
    `reactor.rs`/`compressor.rs`.
    */
    #[test]
    fn mass_and_energy_balance_cancels_when_outflow_matches_inflow_exactly() {
        let harness = run_balanced_mass_and_energy_balance(0.0);

        for key in [
            "separator.state.vapor_a.derivative",
            "separator.state.vapor_b.derivative",
            "separator.state.vapor_c.derivative",
            "separator.state.liquid_d.derivative",
            "separator.state.liquid_e.derivative",
            "separator.state.liquid_f.derivative",
            "separator.state.liquid_g.derivative",
            "separator.state.liquid_h.derivative",
        ] {
            assert_eq!(harness.read(key), 0.0, "vazão de saída igual à de entrada não deveria acumular nada em {key}");
        }
        assert_eq!(
            harness.read("separator.state.enthalpy.derivative"),
            0.0,
            "entalpias e vazões balanceadas não deveriam gerar entalpia líquida"
        );
    }

    #[test]
    fn mass_and_energy_balance_passes_through_separator_heat_when_flows_are_balanced() {
        let separator_heat = -6.5;
        let harness = run_balanced_mass_and_energy_balance(separator_heat);

        assert_eq!(
            harness.read("separator.state.enthalpy.derivative"),
            separator_heat,
            "com fluxos e entalpias balanceados, só separator_heat deveria sobrar"
        );
    }
}
