/* tep/sensors/shutdown_detected.rs */

/* Block 36 de teprob.f — 1.0 se qualquer condição de shutdown for verdadeira. `status.`, não
`xmeas.`: não é uma das 41 XMEAS canônicas do TEP, é um diagnóstico à parte (equivalente ao antigo
`isd_active` do gRPC). Publicado por Measured.
*/
#[monjolo::sensor(key = "status.shutdown_detected")]
pub struct ShutdownDetected;
