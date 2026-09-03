/* tep/sensors/stripper_steam_flow.rs */

/* XMEAS(19), Stripper Steam Flow (kg/hr) — publicado por Measured. Fiel ao teprob.f original:
deriva do calor do condensador (QUC), não de uma vazão mássica de vapor calculada à parte
(teprob.f:980, `XMEAS(19) = QUC * 1.04D3 * 0.454`) — simplificação do modelo clássico, não bug.
*/
#[monjolo::sensor(key = "xmeas.stripper.steam_flow_rate")]
pub struct StripperSteamFlow;
