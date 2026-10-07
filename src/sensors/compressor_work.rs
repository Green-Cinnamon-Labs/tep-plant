/* tep/sensors/compressor_work.rs */

/* XMEAS(20), Compressor Work (kW) — publicado por Measured. */
#[monjolo::sensor(key = "xmeas.compressor.work", noise = 0.2)]
pub struct CompressorWork;
