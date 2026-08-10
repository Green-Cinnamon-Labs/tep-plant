/* tep/subsystems/sensors.rs */

/** Sensores da planta — cada um é uma instância de `monjolo::sensor::model::Sensor` (chave do
StateRegistry + `SensorBehavior`), sem tipo Rust próprio — mesma ideia de subsystems/actuators.rs.

Escopo representativo por enquanto (mesmo critério já usado pros XMEAS antes da fusão em
Composite): só as grandezas que já têm chave real publicada por
Reactor/Separator/Stripper/Compressor hoje. A maioria dos 41 XMEAS clássicos depende de
Flows/Heat/Measurements pra existir, ainda `todo!()` — pressão/temperatura dos 4 subsistemas já são
termodinâmica de verdade, então entram; "nível" (%) não, porque só existe volume
(`separator.liquid_volume`/`stripper.liquid_volume`), grandeza fisicamente diferente, não um
substituto direto.

Todos usam `Ideal` (sem ruído) por enquanto — `Noisy`/`Hysteresis` existem
(`monjolo::sensor::model`) e os desvios-padrão reais por variável estão documentados em
`docs/06-ruidos.md`, pra quando isso entrar em escopo.

`Sensor::new()` já registra o sensor no catálogo do StateRegistry sob a própria chave — devolve
`Arc<Sensor>`, não `Sensor`; ninguém aqui chama `offer_sensor()` à parte.
*/

use std::sync::Arc;

use monjolo::sensor::model::{Ideal, Sensor};
use monjolo::state_registry::StateRegistry;

/* Análogo a XMEAS(9), Reactor Temperature. */
pub fn reactor_temperature(registry: &mut StateRegistry) -> Arc<Sensor> {
    Sensor::new(registry, "reactor.temperature", Box::new(Ideal))
}

/* Análogo a XMEAS(7), Reactor Pressure. */
pub fn reactor_pressure(registry: &mut StateRegistry) -> Arc<Sensor> {
    Sensor::new(registry, "reactor.pressure", Box::new(Ideal))
}

/* Análogo a XMEAS(11), Product Separator Temperature. */
pub fn separator_temperature(registry: &mut StateRegistry) -> Arc<Sensor> {
    Sensor::new(registry, "separator.temperature", Box::new(Ideal))
}

/* Análogo a XMEAS(13), Product Separator Pressure. */
pub fn separator_pressure(registry: &mut StateRegistry) -> Arc<Sensor> {
    Sensor::new(registry, "separator.pressure", Box::new(Ideal))
}

/* Análogo a XMEAS(18), Stripper Temperature. */
pub fn stripper_temperature(registry: &mut StateRegistry) -> Arc<Sensor> {
    Sensor::new(registry, "stripper.temperature", Box::new(Ideal))
}

/* Pressão do compressor/condensador — sem XMEAS clássico correspondente direto, mas grandeza real
já publicada por Compressor.
*/
pub fn compressor_pressure(registry: &mut StateRegistry) -> Arc<Sensor> {
    Sensor::new(registry, "compressor.pressure", Box::new(Ideal))
}
