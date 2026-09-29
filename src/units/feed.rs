/* tep/units/feed.rs */

/** As 4 alimentações externas do TEP (D/E/A/A+C) — sem inventário próprio (nenhuma EDO, nenhum
`#[state]`: são puramente vazão-por-posição-de-válvula, sem acumulação física nenhuma), mas com
identidade própria o bastante pra merecer uma unidade: cada uma tem sua composição fixa (TEINIT),
sua faixa de válvula (VRNG) e, pras duas medidas por massa (D/E), seu próprio peso molecular.

Migrado do antigo `dynamics/flows.rs` (issue 10, spec-tennessee-eastman) — antes, essas 4 vazões e os 2
pesos moleculares eram calculados dentro de `Flows::compute()`, sem nenhum dono próprio. As chaves
publicadas (`flows.stream_flow.0..3`, `flows.d_feed_mol_weight`/`.e_feed_mol_weight`) continuam
EXATAMENTE as mesmas — só quem as publica mudou; `Flows`/`Derivatives`/`Measured` continuam lendo
por chave, indiferentes à origem.
*/

use crate::physics::constants::{TepConstants, TEP_SPECIES};
use monjolo::chemistry::{Mixture};

/* Vazão máxima de cada válvula com curva linear (posição% * range / 100) — VRNG em TEINIT. */
const FEED_D_RANGE: f64 = 400.0;
const FEED_E_RANGE: f64 = 400.0;
const FEED_A_RANGE: f64 = 100.0;
const FEED_AC_RANGE: f64 = 1500.0;

/* Composições nominais dos feeds puros (TEINIT) — índice de componente A=0,B=1,C=2,D=3,E=4,F=5,
G=6,H=7, mesma convenção de physics/constants.rs. `pub(crate)` — Compressor (via
`crate::units::feed::{...}`) reusa FEED_D_COMPOSITION/FEED_E_COMPOSITION/FEED_A_COMPOSITION pro
balanço de massa/energia do compressor, em vez de duplicar os números.

FEED_AC_COMPOSITION (stream 4, o feed combinado A&C) continua `pub(crate)` só pelo teste de
`disturbance::idv1` (que precisa do valor nominal pra montar a expectativa) — quem CONSOME de
verdade (`units::stripper`) não importa mais isto direto: IDV(1)/(2)/(8) perturbam exatamente esta
composição (Table 8, Downs & Vogel 1993), então ela é publicada como um valor OFERECIDO
(`ac_feed_composition` abaixo, sob uma chave "nominal") que `disturbance::idv1::Disturbances::idv1`
(`#[disturbance(key = "disturbance.idv1")]`) intercepta e reoferece sob a chave pública que o
Stripper de fato lê — ver `docs/05-disturbios.md`. Só IDV(1) está implementado por enquanto;
(2)/(8) continuam pendentes, migrados um a um.
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

#[monjolo::tasks(species = TEP_SPECIES, len = 8)]
impl Feed {
    #[task]
    fn d_feed_flow(&self) {
        offer::flows__stream_flow__0 = need::valve__feed_d__position * FEED_D_RANGE / 100.0;
    }

    #[task]
    fn e_feed_flow(&self) {
        offer::flows__stream_flow__1 = need::valve__feed_e__position * FEED_E_RANGE / 100.0;
    }

    /* IDV6 (nominal = 0) atuaria aqui — fora de escopo, mesma lacuna de hoje em Flows. */
    #[task]
    fn a_feed_flow(&self) {
        offer::flows__stream_flow__2 = need::valve__feed_a__position * FEED_A_RANGE / 100.0;
    }

    /* IDV7 (nominal = 0) atuaria aqui — mesma lacuna. +1e-10: evita divisão por zero em quem usa
    este valor como denominador (Block 26 do flash split, `flow[3] / flow[10]`) quando a válvula
    está fechada — preservado do original.
    */
    #[task]
    fn ac_feed_flow(&self) {
        offer::flows__stream_flow__3 = need::valve__feed_ac__position * FEED_AC_RANGE / 100.0 + 1e-10;
    }

    /* Composição NOMINAL do feed combinado A&C (TEINIT) — publicada sob uma chave "nominal" própria,
    nunca a chave pública (`flows.stream4_composition`, que `units::stripper` de fato lê). IDV(1)
    (`tep-plant/src/disturbance/idv1.rs`, `Disturbances::idv1`) precisa desta grandeza NOMINAL como
    `#[need]` pra poder interceptá-la e reofertá-la, alterada ou não, sob a chave pública — `Feed`
    nunca sabe que existe distúrbio nenhum, só publica a condição de projeto, sempre igual. É o
    único lugar do sistema que CRIA uma `Mixture` do nada (matéria entrando de fora da planta).
    */
    #[task]
    fn ac_feed_composition(&self) {
        offer::flows__stream4_composition_nominal::<Mixture> = Mixture::new(FEED_AC_COMPOSITION, &TEP_SPECIES);
    }

    #[task]
    fn d_feed_mol_weight(&self) {
        offer::flows__d_feed_mol_weight = mol_weight(&FEED_D_COMPOSITION, &self.constants);
    }

    #[task]
    fn e_feed_mol_weight(&self) {
        offer::flows__e_feed_mol_weight = mol_weight(&FEED_E_COMPOSITION, &self.constants);
    }

    /* Bloco (ex-measured.rs, Block 35): XMEAS 1-4 (A/D/E/A&C Feed) — conversões de unidade
    preservadas exatamente do original: 0.359/35.3145 (kmol/h → kscmh, gás), XMW*0.454 (kmol/h →
    kg/h, via peso molecular, pras 2 medidas por massa).
    */
    #[task]
    fn xmeas_readings(&self) {
        offer::xmeas__stream1__flow_rate = need::flows__stream_flow__2 * 0.359 / 35.3145;
        offer::xmeas__stream2__flow_rate = need::flows__stream_flow__0 * need::flows__d_feed_mol_weight * 0.454;
        offer::xmeas__stream3__flow_rate = need::flows__stream_flow__1 * need::flows__e_feed_mol_weight * 0.454;
        offer::xmeas__stream4__flow_rate = need::flows__stream_flow__3 * 0.359 / 35.3145;
    }
}
