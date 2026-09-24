/* tep/units/compressor.rs */

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use crate::units::feed::{FEED_A_COMPOSITION, FEED_D_COMPOSITION, FEED_E_COMPOSITION, FEED_TEMPERATURE};
use monjolo::chemistry::{temperature_from_enthalpy, Mixture, Phase};

const COMPRESSOR_VESSEL_VOLUME: f64 = 5000.0; /* volume do vaso do compressor/condensador [m³] */
const GAS_CONSTANT: f64 = 998.9; /* R em [mmHg·m³/(kmol·K)] */
const COMPRESSOR_FLOW_MAX: f64 = 280275.0; /* vazão mássica máxima do compressor [kg/h] */
const COMPRESSOR_PRESSURE_RATIO_MAX: f64 = 1.3;

/** Segunda unidade migrada pro scheduler de dataflow topológico (issue 10), depois de `Feed` —
primeira com `#[state]` própria E várias tarefas cruzando fronteira de outras unidades. Absorve de
`flows.rs`: Block 23 (slot 5, recycle compressor→reator), Block 24 (curva característica + anti-
surge, slot 8 — separador→compressor, mas é a curva do PRÓPRIO compressor que decide quanto ele
puxa, daí o dono ser este arquivo), Block 31 (slot 6, bypass = cópia do slot 5). Absorve de
`derivatives.rs`: a seção "Compressor" do balanço de massa/energia (Block 40, YP(28..36)).
*/
#[monjolo::dynamic_model(tasks)]
pub struct Compressor {
    #[state]
    #[config(prefix = "state.compressor_vapor", components = ["A", "B", "C", "D", "E", "F", "G", "H"])]
    #[offer(prefix = "compressor.state", components = ["0", "1", "2", "3", "4", "5", "6", "7"])]
    vapor: [f64; 8],

    #[state]
    #[config(key = "state.compressor.energy")]
    #[offer(key = "compressor.state.8")]
    enthalpy: f64,

    constants: TepConstants,
}

#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Compressor {
    /* Bloco 1: balanço de energia próprio → temperatura/pressão/composição — igual ao `compute()`
    monolítico de antes, só que agora é UMA tarefa entre várias, com seu próprio `needs`/`offers`.
    */
    #[task]
    fn physical_state(&self) {
        let vapor = Mixture::new(self.vapor(), Phase::Vapor, &TEP_SPECIES);
        let vapor_composition = vapor.mole_fractions();
        let total_vapor_moles = vapor.total();

        let specific_enthalpy = self.enthalpy() / total_vapor_moles;
        let temperature = temperature_from_enthalpy(&vapor_composition.as_array(), need::separator__temperature, specific_enthalpy, 2, &self.constants);
        let temperature_k = temperature + 273.15;
        let pressure = total_vapor_moles * GAS_CONSTANT * temperature_k / COMPRESSOR_VESSEL_VOLUME;

        offer::compressor__temperature = temperature;
        offer::compressor__pressure = pressure;
        offer::compressor__vapor_composition::<Vapor> = vapor_composition;
    }

    /* Bloco 2 (ex-Flows, Blocks 23/24/31): vazão de recycle (slot 5), bypass (slot 6, cópia do
    5), e a curva característica + anti-surge do próprio compressor (slot 8 — fisicamente
    separador→compressor, mas é o desempenho do COMPRESSOR que decide o quanto). `enthalpy[8]`
    do Flows original (entalpia do vapor do separador, SEM a correção de Block 24) é recomputada
    aqui, fresca — mesmo padrão já usado em `derivatives.rs` pras entalpias de feed.
    */
    #[task]
    fn outlet_flows(&self) {
        let compressor_pressure = need::compressor__pressure;
        let separator_pressure = need::separator__pressure;
        let separator_temperature = need::separator__temperature;
        let separator_vapor = need::separator__vapor_composition::<Vapor>;
        let compressor_vapor = need::compressor__vapor_composition::<Vapor>;

        let mw5 = compressor_vapor.dot(&self.constants.xmw);
        let mw8 = separator_vapor.dot(&self.constants.xmw);

        let flow5 = 1937.6 * (compressor_pressure - need::reactor__pressure).max(0.0).sqrt() / mw5;
        let flow6 = flow5;

        let pressure_ratio = (compressor_pressure / separator_pressure).max(1.0).min(COMPRESSOR_PRESSURE_RATIO_MAX);
        let flow_coeff = COMPRESSOR_FLOW_MAX / 1.197;
        let mut compressor_mass_flow = COMPRESSOR_FLOW_MAX + flow_coeff * (1.0 - pressure_ratio.powi(3));
        let compressor_work = compressor_mass_flow * (separator_temperature + 273.15) * 1.8e-6 * 1.9872
            * (compressor_pressure - separator_pressure)
            / (mw8 * separator_pressure);
        compressor_mass_flow -= need::valve__compressor_recycle__position * 53.349 * (compressor_pressure - separator_pressure).max(0.0).sqrt();
        compressor_mass_flow = compressor_mass_flow.max(1e-3);
        let flow8 = compressor_mass_flow / mw8;

        let separator_vapor_enthalpy = separator_vapor.enthalpy(separator_temperature, 1, &self.constants);
        let compressor_discharge_enthalpy = separator_vapor_enthalpy + compressor_work / flow8;

        offer::flows__stream_flow__5 = flow5;
        offer::flows__stream_flow__6 = flow6;
        offer::flows__stream_flow__8 = flow8;
        offer::flows__compressor_work = compressor_work;
        offer::flows__compressor_discharge_enthalpy = compressor_discharge_enthalpy;
    }

    /* Bloco 3 (ex-Derivatives, Block 40 YP(28..36)): balanço de massa/energia do próprio estado —
    igual às outras 3 unidades, quem publica a derivada agora é a própria unidade, não mais um
    componente à parte. Precisa de tanta coisa cruzada quanto o original precisava — a física não
    muda, só quem a hospeda.
    */
    #[task]
    fn mass_and_energy_balance(&self) {
        let flow0 = need::flows__stream_flow__0;
        let flow1 = need::flows__stream_flow__1;
        let flow2 = need::flows__stream_flow__2;
        let flow4 = need::flows__stream_flow__4;
        let flow5 = need::flows__stream_flow__5;
        let flow8 = need::flows__stream_flow__8;
        let flash_vapor_flow = need::flows__flash_vapor_component_flow::<Vapor>;
        let separator_vapor = need::separator__vapor_composition::<Vapor>;
        let compressor_vapor = need::compressor__vapor_composition::<Vapor>;

        /* Composição do vapor do flash (slot 4) só existe normalizando FCM — mesmo cálculo de
        `derivatives.rs` pra este mesmo termo.
        */
        let flash_vapor_total = flash_vapor_flow.total();
        let mut flash_vapor_composition = [0.0f64; 8];
        if flash_vapor_total > 0.0 {
            for i in 0..8 {
                flash_vapor_composition[i] = flash_vapor_flow.component(i) / flash_vapor_total;
            }
        }
        let enthalpy_flash_vapor = Mixture::new(flash_vapor_composition, Phase::Vapor, &TEP_SPECIES).enthalpy(need::stripper__temperature, 1, &self.constants);
        let enthalpy_feed_d = Mixture::new(FEED_D_COMPOSITION, Phase::Vapor, &TEP_SPECIES).enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_feed_e = Mixture::new(FEED_E_COMPOSITION, Phase::Vapor, &TEP_SPECIES).enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_feed_a = Mixture::new(FEED_A_COMPOSITION, Phase::Vapor, &TEP_SPECIES).enthalpy(FEED_TEMPERATURE, 1, &self.constants);
        let enthalpy_compressor_recycle = compressor_vapor.enthalpy(need::compressor__temperature, 1, &self.constants);

        let mut derivative = [0.0f64; 8];
        for i in 0..8 {
            derivative[i] = FEED_D_COMPOSITION[i] * flow0
                + FEED_E_COMPOSITION[i] * flow1
                + FEED_A_COMPOSITION[i] * flow2
                + flash_vapor_flow.component(i)
                + separator_vapor.component(i) * flow8
                - compressor_vapor.component(i) * flow5;
        }

        offer::compressor__state__0__derivative = derivative[0];
        offer::compressor__state__1__derivative = derivative[1];
        offer::compressor__state__2__derivative = derivative[2];
        offer::compressor__state__3__derivative = derivative[3];
        offer::compressor__state__4__derivative = derivative[4];
        offer::compressor__state__5__derivative = derivative[5];
        offer::compressor__state__6__derivative = derivative[6];
        offer::compressor__state__7__derivative = derivative[7];
        offer::compressor__state__8__derivative = enthalpy_feed_d * flow0
            + enthalpy_feed_e * flow1
            + enthalpy_feed_a * flow2
            + enthalpy_flash_vapor * flow4
            + need::flows__compressor_discharge_enthalpy * flow8
            - enthalpy_compressor_recycle * flow5;
    }

    /* Bloco 4 (ex-reactor_feed_analyzer.rs): XMEAS 23-28, Reactor Feed Analysis (Stream 6) — a
    composição de vapor do próprio Compressor (stream 6 = cópia bit-a-bit do bypass, Block 31 de
    teprob.f) convertida de fração molar pra mol% (teprob.f: "Units = Mole %"). Só os 6 primeiros
    componentes (A-F) saem como XMEAS.
    */
    #[task]
    fn reactor_feed_analysis(&self) {
        let composition = need::compressor__vapor_composition::<Vapor>;
        offer::xmeas__stream6__component__a = composition.component(0) * 100.0;
        offer::xmeas__stream6__component__b = composition.component(1) * 100.0;
        offer::xmeas__stream6__component__c = composition.component(2) * 100.0;
        offer::xmeas__stream6__component__d = composition.component(3) * 100.0;
        offer::xmeas__stream6__component__e = composition.component(4) * 100.0;
        offer::xmeas__stream6__component__f = composition.component(5) * 100.0;
    }

    /* Bloco 5 (ex-measured.rs, Block 35): XMEAS 5 (Recycle Flow, stream8), 6 (Reactor Feed Rate,
    stream6), 20 (Compressor Work) — todos vazões/potência do próprio Compressor. XMEAS 16
    (Stripper Pressure) é a exceção conhecida do original: o nome sugere Stripper, mas o valor É a
    pressão do Compressor (teprob.f, não é bug de digitação — preservado tal qual, mesma chave
    externa `xmeas.stripper.pressure` de sempre).
    */
    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__stream8__flow_rate = need::flows__stream_flow__8 * 0.359 / 35.3145;
        offer::xmeas__stream6__flow_rate = need::flows__stream_flow__5 * 0.359 / 35.3145;
        offer::xmeas__stripper__pressure = (need::compressor__pressure - 760.0) / 760.0 * 101.325;
        offer::xmeas__compressor__work = need::flows__compressor_work * 0.29307e3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::harness::Harness;
    use crate::units::separator::Separator;
    use monjolo::snapshot::Snapshot;
    use monjolo::state_registry::StateRegistry;

    /* Investigação do Experimento 20 (spec-tennessee-eastman/experimentos.md), continuação do Exp
    19: o Exp 19 descartou `Reactor::physical_state`/`mass_and_energy_balance` como origem do
    colapso ainda observado depois dos dois fixes de água de resfriamento (Exp 18). Esta seção
    testa as tarefas análogas de `Compressor` — cada uma isolada, com entradas semeadas por chave
    — não precisa da planta inteira.
    */

    #[test]
    fn physical_state_matches_nominal_operating_point_from_application_toml() {
        /* `separator.temperature` é uma entrada externa aqui — em vez de chutar um número,
        computamos ele de verdade a partir de `Separator::physical_state` com o próprio estado
        nominal do separador (mesma técnica de "confiar só em valores já validados", não em números
        de memória). Exp 19 já confirmou `Reactor::physical_state` correto; usamos 120°C (nominal
        documentado) como a temperatura do reator que o separador precisa. As duas unidades vivem
        no MESMO registry, e a tarefa do compressor lê o slot que a do separador escreve.
        */
        let harness = Harness::new();
        let _separator = harness.unit(|registry| {
            Separator::new(
                registry,
                &Snapshot::from_pairs(&[
                    ("state.separator_vapor.A", 63.337045809835196),
                    ("state.separator_vapor.B", 27.67808577797886),
                    ("state.separator_vapor.C", 45.480330241132435),
                    ("state.separator_vapor.D", 0.2398728094022691),
                    ("state.separator_vapor.E", 14.845882843614561),
                    ("state.separator_vapor.F", 1.9220259408956704),
                    ("state.separator_vapor.G", 52.521987477541764),
                    ("state.separator_vapor.H", 41.289131421711694),
                    ("state.separator.energy", 0.571326790156296),
                ]),
            )
        });

        let initial = Snapshot::from_pairs(&[
            ("state.compressor_vapor.A", 107.18718027273283),
            ("state.compressor_vapor.B", 30.801210941908113),
            ("state.compressor_vapor.C", 87.22311903671716),
            ("state.compressor_vapor.D", 22.921606015837646),
            ("state.compressor_vapor.E", 61.9020790931974),
            ("state.compressor_vapor.F", 5.777503238613575),
            ("state.compressor_vapor.G", 12.022672768816811),
            ("state.compressor_vapor.H", 5.609107990171663),
            ("state.compressor.energy", 0.9193937935424599),
        ]);
        let _compressor = harness.unit(|registry| Compressor::new(registry, &initial));

        harness.seed("reactor.temperature", 120.0);
        let separator_state = harness.task("Separator::physical_state");
        let compressor_state = harness.task("Compressor::physical_state");
        harness.resolve();

        separator_state.evaluate();
        compressor_state.evaluate();

        let separator_temperature = harness.read("separator.temperature");
        let temperature = harness.read("compressor.temperature");
        let pressure = harness.read("compressor.pressure");

        /* Faixas com folga generosa (mesmo espírito do teste equivalente em reactor.rs) — o
        objetivo é pegar um colapso grosseiro, não validar casas decimais de um nominal que este
        arquivo não documentava antes.
        */
        assert!(
            (40.0..90.0).contains(&temperature),
            "esperava compressor.temperature numa faixa plausível (a partir de separator_temperature={separator_temperature}°C), obteve {temperature}°C"
        );

        /* XMEAS(16) roda genuinamente mais alto que a pressão do reator/separador (~2700/~2633) —
        não é bug, é a pressão de descarga do próprio compressor. Faixa (2900,3300) escolhida
        depois de rodar este teste pela primeira vez com um chute errado (~2700) e ver 3091.3 kPa
        sair — valor plausível (perto do nominal ~3100 kPa comumente citado na literatura do TEP
        pra esta variável), não uma falha.
        */
        let xmeas_stripper_pressure = (pressure - 760.0) / 760.0 * 101.325;
        assert!(
            (2900.0..3300.0).contains(&xmeas_stripper_pressure),
            "esperava XMEAS(16) (pressão do compressor, apesar do nome) numa faixa plausível (~3100 kPa), obteve {xmeas_stripper_pressure} kPa"
        );
    }

    /* `outlet_flows`: `flow6` é definido como cópia bit-a-bit de `flow5` (Block 31 do teprob.f,
    "bypass") — um invariante estrutural fácil de quebrar sem querer numa refatoração futura
    (ex.: trocar `flow6` por outra fórmula por engano). Barato de testar, caro de deixar passar.
    */
    #[test]
    fn outlet_flows_bypass_is_an_exact_copy_of_recycle_flow() {
        let harness = Harness::new();
        let _compressor = harness.unit(|registry| Compressor::new(registry, &Snapshot::from_pairs(&[])));

        let separator_vapor = [0.1, 0.05, 0.1, 0.05, 0.2, 0.1, 0.2, 0.2];
        harness.seed("compressor.pressure", 25000.0); /* compressor_pressure (mmHg) */
        harness.seed("reactor.pressure", 21000.0);
        harness.seed("separator.pressure", 20000.0);
        harness.seed("separator.temperature", 80.1);
        harness.seed_mixture("separator.vapor_composition", &separator_vapor);
        harness.seed_mixture("compressor.vapor_composition", &separator_vapor); /* mesma composição só pra simplificar o teste */
        harness.seed("valve.compressor_recycle.position", 22.21); /* nominal, docs/07-controle.md */

        let task = harness.task("Compressor::outlet_flows");
        harness.resolve();
        task.evaluate();

        assert_eq!(
            harness.read("flows.stream_flow.6"),
            harness.read("flows.stream_flow.5"),
            "flow6 (bypass) deveria ser sempre uma cópia exata de flow5 (recycle)"
        );
    }

    /* `mass_and_energy_balance`: com feeds frescos zerados (flow0/1/2=0), sem vapor de flash
    (flash_vapor_flow=0) e o reciclo balanceado (mesma composição/entalpia entrando e saindo, na
    mesma vazão), nada deveria se acumular — mesma técnica de "fluxo balanceado" já usada nos
    testes de `Reactor::mass_and_energy_balance`.
    */
    #[test]
    fn mass_and_energy_balance_cancels_when_only_recycle_flow_is_present_and_balanced() {
        let harness = Harness::new();
        let _compressor = harness.unit(|registry| Compressor::new(registry, &Snapshot::from_pairs(&[])));

        let composition = [0.1, 0.05, 0.1, 0.05, 0.2, 0.1, 0.2, 0.2];
        let temperature = 45.0;
        let flow = 300.0;
        let discharge_enthalpy = Mixture::new(composition, Phase::Vapor, &TEP_SPECIES).enthalpy(temperature, 1, &TepConstants::new());

        harness.seed("flows.stream_flow.0", 0.0); /* sem feed fresco */
        harness.seed("flows.stream_flow.1", 0.0);
        harness.seed("flows.stream_flow.2", 0.0);
        harness.seed("flows.stream_flow.4", 0.0); /* sem vazão de flash (o vetor flash_vapor_flow já é zero) */
        harness.seed("flows.stream_flow.5", flow); /* reciclo saindo */
        harness.seed("flows.stream_flow.8", flow); /* reciclo entrando, mesma vazão */
        harness.seed_mixture("flows.flash_vapor_component_flow", &[0.0; 8]); /* sem vapor de flash */
        harness.seed_mixture("separator.vapor_composition", &composition); /* mesma composição do compressor */
        harness.seed("stripper.temperature", temperature); /* não importa aqui (flash_vapor_flow=0) */
        harness.seed("flows.compressor_discharge_enthalpy", discharge_enthalpy);
        harness.seed_mixture("compressor.vapor_composition", &composition);
        harness.seed("compressor.temperature", temperature);

        let task = harness.task("Compressor::mass_and_energy_balance");
        harness.resolve();
        task.evaluate();

        for i in 0..8 {
            let key = format!("compressor.state.{i}.derivative");
            assert_eq!(harness.read(&key), 0.0, "reciclo com mesma composição/vazão entrando e saindo não deveria acumular nada em {key}");
        }
        assert_eq!(
            harness.read("compressor.state.8.derivative"),
            0.0,
            "reciclo balanceado (mesma entalpia/vazão) não deveria gerar entalpia líquida"
        );
    }

    #[test]
    fn new_seeds_own_state_with_initial_condition() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[
            ("state.compressor_vapor.A", 1.0),
            ("state.compressor_vapor.B", 2.0),
            ("state.compressor_vapor.C", 3.0),
            ("state.compressor_vapor.D", 4.0),
            ("state.compressor_vapor.E", 5.0),
            ("state.compressor_vapor.F", 6.0),
            ("state.compressor_vapor.G", 7.0),
            ("state.compressor_vapor.H", 8.0),
            ("state.compressor.energy", 42.0),
        ]);

        let compressor = Compressor::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(compressor.vapor(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(compressor.enthalpy(), 42.0);
    }

    #[test]
    fn new_defaults_missing_keys_to_zero() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);

        let compressor = Compressor::new(&mut registry.borrow_mut(), &initial);

        assert_eq!(compressor.vapor(), [0.0; 8]);
        assert_eq!(compressor.enthalpy(), 0.0);
    }
}
