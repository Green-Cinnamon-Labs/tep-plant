/* tep/dynamics/mod.rs */

/** Os 8 blocos químicos do TEP: Reactor/Separator/Stripper/Compressor (acumuladores de massa e
energia, cada um com EDO própria — `#[state]` em pelo menos um campo, estado que o integrador
avança) e Flows/Heat/Derivatives/Measured (álgebra transversal sobre os quatro acumuladores, só
`#[need]`/`#[offer]`, sem estado próprio). Todos `#[dynamic_model]`, auto-descobertos via
inventory — nenhum é construído manualmente em lugar nenhum.

Cadeia da fase (A): Reactor→Separator→Stripper→Compressor→Flows→Heat→Derivatives→Measured.
`Derivatives` é quem de fato fecha o balanço de massa/energia (Block 40 de teprob.f) e escreve nas
`.derivative` dos quatro acumuladores — roda depois de `Flows` E `Heat` porque precisa dos dois ao
mesmo tempo (streams de `Flows`, cargas térmicas de `Heat`), o que só é possível nesta posição
(ver `dynamics::derivatives`).
*/
pub mod compressor;
pub mod derivatives;
pub mod flows;
pub mod heat;
pub mod measured;
pub mod reactor;
pub mod separator;
pub mod stripper;
