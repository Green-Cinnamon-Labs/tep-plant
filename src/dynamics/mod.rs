/* tep/dynamics/mod.rs */

/** Os 5 blocos químicos do TEP com identidade própria: Reactor/Separator/Stripper/Compressor
(acumuladores de massa e energia, cada um com EDO própria — `#[state]` em pelo menos um campo,
estado que o integrador avança) e `Feed` (as 4 alimentações externas D/E/A/A+C — sem EDO própria,
mas com composição/faixa de válvula/peso molecular próprios, migrado de `Flows` na issue 10). Só
`Feed` usa `#[monjolo::tasks]` até agora (vários métodos nomeados, cada um seu próprio `needs`/
`offers` — ver `dynamics::feed`); os outros 4 ainda são `compute()` único, migração em andamento.

Flows/Heat/Derivatives/Measured continuam existindo (álgebra transversal ainda não migrada pra
dentro das unidades donas — ver issue 10), mais os 3 analisadores de composição (XMEAS 23-41 —
`reactor_feed_analyzer`/`purge_analyzer`/`product_analyzer`, mesmo molde de `Measured`: leem
composição já publicada, convertem fração molar → mol%, sem estado próprio). Todos
`#[dynamic_model]`, auto-descobertos via inventory — nenhum é construído manualmente em lugar
nenhum.

Ordem de avaliação da fase (A): desde a extensão de `component::sort_phase_a` (issue 10), a ordem
não é mais uma cadeia `after=[...]` só — é inferida automaticamente casando `needs`↔`offers` entre
TODOS os nós (struct inteira ou tarefa de método), com `after` como desempate. `Feed` não declara
`after` nenhum (suas únicas dependências são chaves de atuador, satisfeitas fora da fase A) e ainda
assim `Flows` (que `#[need]`s `flows.stream_flow.3`, hoje ofertado por `Feed::ac_feed_flow`) acaba
depois dele, sem hint manual nenhum.
*/
pub mod compressor;
pub mod derivatives;
pub mod feed;
pub mod flows;
pub mod heat;
pub mod measured;
pub mod product_analyzer;
pub mod purge_analyzer;
pub mod reactor;
pub mod reactor_feed_analyzer;
pub mod separator;
pub mod stripper;
