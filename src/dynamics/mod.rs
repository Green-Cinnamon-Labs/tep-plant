/* tep/dynamics/mod.rs */

/** Os 8 blocos químicos do TEP: Reactor/Separator/Stripper/Compressor (acumuladores de massa e
energia, cada um com EDO própria — `#[state]` em pelo menos um campo, estado que o integrador
avança) e Flows/Heat/Derivatives/Measured (álgebra transversal sobre os quatro acumuladores, só
`#[need]`/`#[offer]`, sem estado próprio), mais os 3 analisadores de composição (XMEAS 23-41 —
`reactor_feed_analyzer`/`purge_analyzer`/`product_analyzer`, mesmo molde de `Measured`: leem
composição já publicada, convertem fração molar → mol%, sem estado próprio). Todos
`#[dynamic_model]`, auto-descobertos via inventory — nenhum é construído manualmente em lugar
nenhum.

Cadeia da fase (A): Reactor→Separator→Stripper→Compressor→Flows→Heat→Derivatives→Measured→
ReactorFeedAnalyzer→PurgeAnalyzer→ProductAnalyzer. `Derivatives` é quem de fato fecha o balanço de
massa/energia (Block 40 de teprob.f) e escreve nas `.derivative` dos quatro acumuladores — roda
depois de `Flows` E `Heat` porque precisa dos dois ao mesmo tempo (streams de `Flows`, cargas
térmicas de `Heat`), o que só é possível nesta posição (ver `dynamics::derivatives`). Os 3
analisadores rodam por último, depois de `Measured`, porque só precisam de composição já publicada
pelos 4 acumuladores — a posição exata na cadeia não importa fisicamente, só precisa vir depois de
quem publica `compressor.vapor_composition`/`separator.vapor_composition`/
`stripper.liquid_composition` (cadeia única de propósito, ver `monjolo::component`).
*/
pub mod compressor;
pub mod derivatives;
pub mod flows;
pub mod heat;
pub mod measured;
pub mod product_analyzer;
pub mod purge_analyzer;
pub mod reactor;
pub mod reactor_feed_analyzer;
pub mod separator;
pub mod stripper;
