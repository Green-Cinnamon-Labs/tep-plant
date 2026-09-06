/* tep/dynamics/mod.rs */

/** Os 5 blocos químicos do TEP com identidade própria: só `Reactor` ainda é `compute()` único
(último passo da migração, issue 10) — Feed/Compressor/Separator/Stripper já são
`#[monjolo::tasks]` (vários métodos nomeados, cada um seu próprio `needs`/`offers`), tendo
absorvido por completo o que antes morava em `flows.rs`/`heat.rs`/`derivatives.rs` e os 3
analisadores de composição (`reactor_feed_analyzer`→Compressor, `purge_analyzer`→Separator,
`product_analyzer`→Stripper — todos dissolvidos).

Flows/Heat/Derivatives/Measured continuam existindo, mas só com o que resta pro Reactor absorver:
Flows (Block 23, slot 7 + `agitation_factor`), Heat (Block 32, reactor_heat), Derivatives (a seção
"Reator" do Block 40). Todos `#[dynamic_model]`, auto-descobertos via inventory — nenhum é
construído manualmente em lugar nenhum.

Ordem de avaliação da fase (A): desde a extensão de `component::sort_phase_a` (issue 10), a ordem
não é mais uma cadeia `after=[...]` só — é inferida automaticamente casando `needs`↔`offers` entre
TODOS os nós (struct inteira ou tarefa de método), com `after` como desempate.
*/
pub mod compressor;
pub mod derivatives;
pub mod feed;
pub mod flows;
pub mod heat;
pub mod measured;
pub mod reactor;
pub mod separator;
pub mod stripper;
