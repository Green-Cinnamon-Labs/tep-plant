/* tep/dynamics/mod.rs */

/** Os 5 blocos químicos do TEP: Feed/Reactor/Separator/Stripper/Compressor, todos `#[monjolo::
tasks]` (issue 10) — vários métodos nomeados por unidade, cada um seu próprio `needs`/`offers`.
`flows.rs`/`heat.rs`/`derivatives.rs`/`measured.rs` (toda a álgebra transversal do FORTRAN
original — vazões entre unidades, cargas térmicas, o balanço de massa/energia final, conversões
pra XMEAS) foram totalmente dissolvidos: cada pedaço absorvido pela unidade que o produz. Os 3
analisadores de composição também (`reactor_feed_analyzer`→Compressor, `purge_analyzer`→
Separator, `product_analyzer`→Stripper). O único pedaço de `measured.rs` sem dono natural
(detecção de shutdown, agrega 3 unidades) virou `diagnostics::shutdown_detector`.

Ordem de avaliação da fase (A): desde a extensão de `component::sort_phase_a` (issue 10), a ordem
não é mais uma cadeia `after=[...]` só — é inferida automaticamente casando `needs`↔`offers` entre
TODOS os nós (struct inteira ou tarefa de método), com `after` como desempate. Nenhuma das 5
unidades declara `after` hoje — a ordem inteira (Reactor→Separator/Compressor→Stripper e as
dependências cruzadas de cada tarefa) sai só do casamento de chave.
*/
pub mod compressor;
pub mod feed;
pub mod reactor;
pub mod separator;
pub mod stripper;
