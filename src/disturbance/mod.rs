/* tep/disturbance/mod.rs */

/* NOTA (2026-10-01): este arquivo já hospedou um `Disturbance` (singular) de canal cúbico/pulso —
código morto desde a migração pro scheduler de dataflow topológico (issue #10), nunca chamado,
apagado nesta data (ver histórico do git se precisar recuperar algo). Os IDVs "aleatório"/"deriva
lenta" (8-13, 16-18, 20) vão precisar de uma mecânica equivalente (canal com `time`, estado que
avança por tick), ainda a reconstruir sob o padrão novo quando chegar a vez deles — a interceptação
atual (ver `Disturbances`, abaixo) só suporta funções PURAS.
*/

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
