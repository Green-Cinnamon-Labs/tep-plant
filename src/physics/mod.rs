/* tep/physics/mod.rs */

/** Números do TEP compartilhados entre os subsistemas (`TepConstants`) — não é um subsistema em
si, ninguém aqui implementa DynamicModel. As correlações termodinâmicas em si (entalpia, Antoine,
densidade) moram em `monjolo::chemistry` (feature `chemistry`) — genéricas, não específicas do TEP.
*/

pub mod constants;
