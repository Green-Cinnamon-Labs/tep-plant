/* tep/subsystems/mod.rs */

/** Os 7 subsistemas químicos do TEP (Reactor/Separator/Stripper/Compressor/Flows/Heat/Measurements)
— ver comentário em reactor.rs sobre por que esses 7 formam um grupo (a física original de teprob.f,
antes de virar DynamicModel real cada um). model.rs (o composto, TennesseeEastmanModel) fica fora
dessa pasta de propósito: não é um subsistema, é quem os orquestra.

`actuators` (plural — mais de um tipo no mesmo arquivo: `Valve`/`Agitator`) mora aqui também: são
componentes físicos reais da planta, com estado e derivada próprios, igual aos 7 acima — a diferença
é só que também implementam `monjolo::actuator::Actuator` (aceitam comando de fora).
*/
pub mod actuators;
pub mod compressor;
pub mod flows;
pub mod heat;
pub mod measurements;
pub mod reactor;
pub mod separator;
pub mod stripper;
