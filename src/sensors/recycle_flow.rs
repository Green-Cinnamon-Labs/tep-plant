/* tep/sensors/recycle_flow.rs */

/* XMEAS(5), Recycle Flow (Stream 8) — kscmh, publicado por Measured. A stream física é a 8, não a
5 — confirmado no próprio cabeçalho de docs/fortran-original/teprob.f, não é erro de digitação.
*/
#[monjolo::sensor(key = "xmeas.stream8.flow_rate")]
pub struct RecycleFlow;
