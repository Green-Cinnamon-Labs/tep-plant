/* tep/sensors/product_component_f.rs */

/* XMEAS(39), Product Analysis — Component F (mol%), publicado por ProductAnalyzer. */
#[monjolo::sensor(key = "xmeas.stream11.component.f", noise = 0.01)]
pub struct ProductComponentF;
