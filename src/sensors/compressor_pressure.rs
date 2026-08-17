/* tep/sensors/compressor_pressure.rs */

/* Pressão do compressor/condensador — sem XMEAS clássico correspondente direto, mas grandeza real
já publicada por Compressor.
*/
#[monjolo::sensor(key = "compressor.pressure")]
pub struct CompressorPressure;
