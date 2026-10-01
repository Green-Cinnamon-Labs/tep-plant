/* tep/disturbance/mod.rs */

/** Distúrbios por INTERCEPTAÇÃO (issue spec-tennessee-eastman#73) — mesmo padrão de `#[actuator]`/
`#[sensor]`/`#[controller]`: cada `idvN.rs` declara seu PRÓPRIO struct sem campos (`Idv1`, `Idv2`,
...), anotado `#[monjolo::disturbance(key = "...", intercepts = "...", [components = [...]])]`, e
um `impl` à parte com um método convencional, `disturb(&self)` (sem parâmetro — lê via `self.raw()`,
escreve via o retorno). Ver `docs/05-disturbios.md` e `docs/17-disturbio-por-interceptacao.md`.

NOTA (2026-10-01): este arquivo já hospedou um `Disturbance` (singular) de canal cúbico/pulso —
código morto desde a migração pro scheduler de dataflow topológico (issue #10), nunca chamado,
apagado nesta data (ver histórico do git se precisar recuperar algo). Os IDVs "aleatório"/"deriva
lenta" (8-13, 16-18, 20) vão precisar de uma mecânica equivalente (canal com `time`, estado que
avança por tick), ainda a reconstruir sob o padrão novo quando chegar a vez deles — a interceptação
atual só suporta um método PURO (`disturb(&self)`, só lê o que `raw()`/`active()` já expõem).

Este arquivo NÃO declara mais um struct `Disturbances` compartilhado — cada IDV é um TIPO próprio,
"impl com o nome do distúrbio", não um método a mais num struct genérico.
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
