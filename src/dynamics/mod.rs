/* tep/dynamics/mod.rs */

/** Os 5 blocos químicos do TEP com identidade própria: Reactor/Stripper (ainda `compute()` único,
migração em andamento) e Feed/Compressor/Separator (já `#[monjolo::tasks]` — vários métodos
nomeados, cada um seu próprio `needs`/`offers`, ver issue 10). `Feed` não tem EDO própria (só
composição/faixa de válvula/peso molecular fixos); Compressor/Separator têm `#[state]` E várias
tarefas, absorvendo o que antes morava em `flows.rs`/`heat.rs`/`derivatives.rs`/os analisadores de
composição (`reactor_feed_analyzer`→Compressor, `purge_analyzer`→Separator, já dissolvidos).

Flows/Heat/Derivatives/Measured continuam existindo (o que ainda não foi absorvido por Reactor/
Stripper), mais `product_analyzer` (XMEAS 37-41, ainda não dissolvido — vez do Stripper). Todos
`#[dynamic_model]`, auto-descobertos via inventory — nenhum é construído manualmente em lugar
nenhum.

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
pub mod product_analyzer;
pub mod reactor;
pub mod separator;
pub mod stripper;
