/* tep/disturbance/mod.rs */

/** Disturbance NÃO implementa DynamicModel — sua dinâmica real depende de `time`, e
`evaluate(&self)` não recebe parâmetro nenhum (nem `time` nem nada mais). Em vez disso, se inscreve
no StateRegistry igual qualquer componente (ganha Proxy pros próprios outputs), mas expõe um método
próprio, `advance(&mut self, time: f64)`, que quem orquestra a simulação (Simulation) chama
diretamente, antes de disparar o `evaluate()` da árvore de DynamicModel — assim Reactor/Flows/Heat
já encontram os valores atualizados quando forem ler via `need`.

Escopo atual: só os 4 valores de DisturbanceOut (reaction_factor_1/2,
reactor/separator_cooling_water_temp) são oferecidos — os canais 0..3 (composição/temperatura de
feed, que mutavam stream_comps/stream_temps no código original) ainda não têm consumidor (Flows não
existe), então não são expostos ainda. As 12 posições continuam avançando por trás (blocks 8-11 do
teprob.f), só a leitura (block 12) está reduzida ao que já é consumido.

NOTA (2026-10-01): este `Disturbance` (singular, canal cúbico/pulso) é código morto desde a
migração pro scheduler de dataflow topológico (issue #10) — nunca chamado. Convive, por ora, com
`Disturbances` (plural, no fim deste arquivo), o mecanismo NOVO por interceptação (issue #73) — ver
`docs/17-disturbio-por-interceptacao.md`. Os IDVs "aleatório"/"deriva lenta" (8-13, 16-18, 20) vão
precisar da MESMA mecânica de canal cúbico que este struct já tem, só que portada pro padrão novo
(a interceptação atual só suporta funções PURAS, sem `time` nem estado que avança por tick) — é por
isso que este código ainda não foi removido.
*/

use monjolo::disturbance::cubic::{eval_disturbance, lcg_rand, update_segment};
use monjolo::state_registry::{Proxy, StateRegistry};

pub mod idv1;
pub mod idv2;
pub mod idv3;
pub mod idv4;
pub mod idv5;
pub mod idv6;
pub mod idv7;
pub mod idv8;
pub mod idv9;
pub mod idv10;
pub mod idv11;
pub mod idv12;
pub mod idv13;
pub mod idv14;
pub mod idv15;
pub mod idv16;
pub mod idv17;
pub mod idv18;
pub mod idv19;
pub mod idv20;
pub mod state;
use state::TepDisturbanceState;

pub struct Disturbance {
    state: TepDisturbanceState,
    flags: [i32; 20],
    step_magnitudes: [f64; 20],
    reaction_factor_1: Proxy,
    reaction_factor_2: Proxy,
    reactor_cooling_water_temp: Proxy,
    separator_cooling_water_temp: Proxy,
}

impl Disturbance {
    pub fn new(registry: &mut StateRegistry) -> Self {
        let (offered, _) = registry.subscribe(&[
            "disturbance.reaction_factor_1",
            "disturbance.reaction_factor_2",
            "disturbance.reactor_cooling_water_temp",
            "disturbance.separator_cooling_water_temp",
        ], &[]);

        Self {
            state: TepDisturbanceState::new(),
            flags: [0; 20],
            step_magnitudes: [1.0; 20],
            reaction_factor_1: offered[0].clone(),
            reaction_factor_2: offered[1].clone(),
            reactor_cooling_water_temp: offered[2].clone(),
            separator_cooling_water_temp: offered[3].clone(),
        }
    }

    pub fn set_disturbances(&mut self, idv: &[i32]) {
        for (i, &v) in idv.iter().enumerate().take(20) {
            self.flags[i] = v;
        }
    }

    pub fn set_step_magnitude(&mut self, idx: usize, magnitude: f64) {
        if idx < 20 { self.step_magnitudes[idx] = magnitude; }
    }

    /** Avança os 12 canais cúbicos até `time` e escreve os 4 outputs consumidos hoje nos
    respectivos Proxy. Chamado por quem orquestra a simulação, não pelo `evaluate()` de ninguém.
    */
    pub fn advance(&mut self, time: f64) {
        let inner = &mut self.state.inner;

        let mut flags = self.flags;
        for v in flags.iter_mut() {
            *v = if *v > 0 { 1 } else { 0 };
        }

        /* Block 8 — liga/desliga canais conforme flags IDV ativos */
        inner.channels[0].active = flags[7];
        inner.channels[1].active = flags[7];
        inner.channels[2].active = flags[8];
        inner.channels[3].active = flags[9];
        inner.channels[4].active = flags[10];
        inner.channels[5].active = flags[11];
        inner.channels[6].active = flags[12];
        inner.channels[7].active = flags[12];
        inner.channels[8].active = flags[15];
        inner.channels[9].active = flags[16];
        inner.channels[10].active = flags[17];
        inner.channels[11].active = flags[19];

        /* Block 9 — avança segmentos cúbicos dos canais 0-8 (contínuos) */
        for i in 0..9 {
            if time >= inner.channels[i].t_next {
                let segment_duration = inner.channels[i].t_next - inner.channels[i].t_last;
                let ch = &inner.channels[i];
                let end_value = ch.a + segment_duration * (ch.b + segment_duration * (ch.c + segment_duration * ch.d));
                let end_slope = ch.b + segment_duration * (2.0 * ch.c + 3.0 * segment_duration * ch.d);
                inner.channels[i].t_last = inner.channels[i].t_next;
                update_segment(i, end_value, end_slope, inner);
            }
        }

        /* Block 10 — avança segmentos cúbicos dos canais 9-11 (pulsos aleatórios) */
        for i in 9..12 {
            if time >= inner.channels[i].t_next {
                let segment_duration = inner.channels[i].t_next - inner.channels[i].t_last;
                let ch = &inner.channels[i];
                let end_value = ch.a + segment_duration * (ch.b + segment_duration * (ch.c + segment_duration * ch.d));
                let end_slope = ch.b + segment_duration * (2.0 * ch.c + 3.0 * segment_duration * ch.d);
                inner.channels[i].t_last = inner.channels[i].t_next;
                if end_value > 0.1 {
                    inner.channels[i].a = end_value;
                    inner.channels[i].b = end_slope;
                    inner.channels[i].c = -(3.0 * end_value + 0.2 * end_slope) / 0.01;
                    inner.channels[i].d = (2.0 * end_value + 0.1 * end_slope) / 0.001;
                    inner.channels[i].t_next = inner.channels[i].t_last + 0.1;
                } else {
                    let next_duration = inner.channels[i].h_span * lcg_rand(-1, inner) + inner.channels[i].h_zero;
                    inner.channels[i].a = 0.0;
                    inner.channels[i].b = 0.0;
                    inner.channels[i].c = inner.channels[i].active as f64 / (next_duration * next_duration);
                    inner.channels[i].d = 0.0;
                    inner.channels[i].t_next = inner.channels[i].t_last + next_duration;
                }
            }
        }

        /* Block 11 — inicializa todos os canais no instante zero */
        if time == 0.0 {
            for i in 0..12 {
                inner.channels[i].a = inner.channels[i].s_zero;
                inner.channels[i].b = 0.0;
                inner.channels[i].c = 0.0;
                inner.channels[i].d = 0.0;
                inner.channels[i].t_last = 0.0;
                inner.channels[i].t_next = 0.1;
            }
        }

        /* Block 12 (reduzido) — só os 4 canais que já têm consumidor */
        let reactor_cw_temp = eval_disturbance(4, time, inner) + flags[3] as f64 * self.step_magnitudes[3];
        let separator_cw_temp = eval_disturbance(5, time, inner) + flags[4] as f64 * self.step_magnitudes[4];
        let reaction_factor_1 = eval_disturbance(6, time, inner);
        let reaction_factor_2 = eval_disturbance(7, time, inner);

        self.reaction_factor_1.set(reaction_factor_1);
        self.reaction_factor_2.set(reaction_factor_2);
        self.reactor_cooling_water_temp.set(reactor_cw_temp);
        self.separator_cooling_water_temp.set(separator_cw_temp);
    }
}

/** Dona de todos os `#[disturbance(...)]` por INTERCEPTAÇÃO (issue spec-tennessee-eastman#73) — um
`impl Disturbances { ... }` por arquivo (`idv1.rs`, `idv2.rs`, ...), cada um com seu próprio método
`#[disturbance(key = "...", intercepts = "...", [components = [...]])]`. O struct em si não guarda
estado nenhum (`{}`) — não é usado por essa parte do mecanismo (`registry.instance::<Disturbances>(
...)` nunca é chamado pra um `#[disturbance]` sozinho, sem `#[need]`/`#[offer]` empilhado; ver
`monjolo-macros/macros/tasks.rs::build_disturbance_interceptor`); existe só como ponto de ancoragem
pra macro, que precisa de um `impl TIPO { ... }` real pra se anexar.

Cada método marcado é uma função PURA, sem `&self` — vira um `fn(f64, &[f64]) -> Vec<f64>` sem
nenhuma captura, chamado de dentro de `Proxy::get()` pra quem lê a chave interceptada (`need::`),
sem acesso ao `StateRegistry` nem a qualquer instância. `key` cataloga o próprio método como o
comando externo liga/desliga (`Actuator`, exposição OPC-UA automática); `intercepts`/`components` é
a chave (ou prefixo de uma `Mixture`) que passa a ser lida já transformada — quem publica (`offer::`)
e quem consome (`need::`) nunca sabem que isto existe. Ver `docs/05-disturbios.md` e
`docs/17-disturbio-por-interceptacao.md`.
*/
#[monjolo::dynamic_model(tasks)]
pub struct Disturbances {}
