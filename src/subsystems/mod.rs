// tep/subsystems/mod.rs

/** Os 7 subsistemas químicos do TEP (Reactor/Separator/Stripper/Compressor/
 Flows/Heat/Measurements) — ver comentário em reactor.rs sobre por que
 esses 7 formam um grupo (a física original de teprob.f, antes de virar
 DynamicModel real cada um). model.rs (o composto, TennesseeEastmanModel)
 fica fora dessa pasta de propósito: não é um subsistema, é quem os
 orquestra.
*/
pub mod compressor;
pub mod flows;
pub mod heat;
pub mod measurements;
pub mod reactor;
pub mod separator;
pub mod stripper;
