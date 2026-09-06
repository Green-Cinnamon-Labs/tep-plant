/* tep/diagnostics/mod.rs */

/** Diagnósticos da planta que agregam MAIS DE UMA unidade ao mesmo tempo — não são acumuladores
físicos (não pertencem a `units/`), mas também não fazem sentido como tarefa de uma unidade
específica, já que dependem de várias. Hoje só `ShutdownDetector` (Block 36 de teprob.f).
*/

pub mod shutdown_detector;
