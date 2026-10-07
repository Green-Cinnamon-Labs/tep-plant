/* tep/sensors/reactor_cooling_water_outlet_temperature.rs */

/* XMEAS(21), Reactor Cooling Water Outlet Temp (°C) — publicado por Measured. */
#[monjolo::sensor(key = "xmeas.reactor.cooling_water_outlet_temperature", noise = 0.01)]
pub struct ReactorCoolingWaterOutletTemperature;
