/** tep/lib.rs

Só o que é específico do TEP mora aqui. Framework de simulação genérico (DynamicModel,
StateRegistry, Integrator, atuador/sensor de 1ª ordem, distúrbio cúbico, e agora também o carregador
genérico de condição inicial — `monjolo::snapshot::Snapshot`) mora em monjolo (crate irmão).

Organização de pastas: subsystems/ (os 7 blocos químicos + actuators.rs, 12 tipos concretos de
atuador — 11 válvulas + `Agitator`),
disturbance/ (Disturbance + seu estado interno), physics/ (constantes e correlações termodinâmicas
compartilhadas). model.rs (o composto, TennesseeEastmanModel) fica solto na raiz porque é ponto de
composição/entrada, não subsistema.

NÃO existe mais um `initial_state.rs` próprio do TEP — o struct rígido
(`InitialState`/`StateSections`, um campo por chave do TOML) foi substituído pelo `Snapshot`
genérico do framework: cada subsistema (`Reactor::new()`, por enquanto) recebe um `&Snapshot` já
carregado e busca só as chaves que interessam pra ele, por nome — do mesmo jeito que já faz com
`StateRegistry`.
*/
pub mod disturbance;
pub mod model;
pub mod physics;
pub mod subsystems;

/* subsystems.rs original (física de todos os 7 acima) preservada em _deprecated_2.rs — não é módulo
do crate, só referência.
*/
