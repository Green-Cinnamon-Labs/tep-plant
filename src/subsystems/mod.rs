/* tep/subsystems/mod.rs */

/** Os 7 subsistemas químicos do TEP (Reactor/Separator/Stripper/Compressor/Flows/Heat/Measurements)
— ver comentário em reactor.rs sobre por que esses 7 formam um grupo (a física original de teprob.f,
antes de virar DynamicModel real cada um). model.rs (o composto, TennesseeEastmanModel) fica fora
dessa pasta de propósito: não é um subsistema, é quem os orquestra.

`actuators`/`sensors`/`controllers` moram aqui também, mas nenhum é um subsistema com tipo próprio —
os componentes físicos (12 atuadores, 6 sensores, os controllers já declarados) não têm `struct`
nenhuma (ver `monjolo::actuator::model::Actuator`/`sensor::model::Sensor`/`controller::model::Controller`);
cada arquivo só agrupa funções, uma por componente, cada uma construindo uma instância pra
`model.rs::build_tep()` chamar.
*/
pub mod actuators;
pub mod compressor;
pub mod controllers;
pub mod flows;
pub mod heat;
pub mod measurements;
pub mod reactor;
pub mod sensors;
pub mod separator;
pub mod stripper;
