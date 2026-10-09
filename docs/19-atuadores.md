# Atuadores (`src/actuators/`) — o que cada parte faz e por quê

Este documento reúne o que antes estava em comentários dentro dos arquivos de `src/actuators/` (o código ficou só com o código, no mesmo padrão de `docs/12-reator.md`). São os 12 atuadores físicos da planta, XMV(1) a XMV(12), um arquivo por atuador, todos declarados com `#[monjolo::actuator(...)]`. Os testes ficam em `tests/actuators/`.

## A lei

Os 12 seguem a mesma dinâmica de primeira ordem: a posição da válvula persegue o comando com constante de tempo τ, `d(posição)/dt = (comando − posição) / τ`. O comando é o que um controlador ou um cliente OPC-UA escreve; a posição é o `#[state]` que o Integrator integra. O τ vem do `VTAU(n)` do `teprob.f`, em segundos, e no código aparece dividido por 3600 porque o Integrator trabalha em horas (`dt_hours`).

## Valor inicial

`config = "state.valves.*"` semeia comando e posição com o mesmo valor nominal do snapshot, então a derivada nasce em zero e nada se mexe sozinho até alguém escrever um comando diferente. Sem isso, todo atuador nascia em 0 mesmo com o valor certo em `application.toml`, e a planta não estabilizava porque as válvulas manuais nasciam fechadas.

## Os 12 atuadores

| XMV | Nome | Arquivo (struct) | Chave | `config` | τ |
|---|---|---|---|---|---|
| 1 | D Feed Flow | `feed_d.rs` (`FeedD`) | `valve.feed_d.position` | `state.valves.d_feed` | 8 s |
| 2 | E Feed Flow | `feed_e.rs` (`FeedE`) | `valve.feed_e.position` | `state.valves.e_feed` | 8 s |
| 3 | A Feed Flow | `feed_a.rs` (`FeedA`) | `valve.feed_a.position` | `state.valves.a_feed` | 6 s |
| 4 | A&C Feed Flow (alimentação combinada) | `feed_ac.rs` (`FeedAc`) | `valve.feed_ac.position` | `state.valves.a_c_feed` | 9 s |
| 5 | Compressor Recycle Valve | `compressor_recycle.rs` (`CompressorRecycle`) | `valve.compressor_recycle.position` | `state.valves.compressor_recycle_valve` | 7 s |
| 6 | Purge Valve | `purge.rs` (`Purge`) | `valve.purge.position` | `state.valves.purge_valve` | 5 s |
| 7 | Separator Pot Liquid Flow (underflow do separador) | `separator_underflow.rs` (`SeparatorUnderflow`) | `valve.separator_underflow.position` | `state.valves.separator_underflow` | 5 s |
| 8 | Stripper Liquid Product Flow | `stripper_product.rs` (`StripperProduct`) | `valve.stripper_product.position` | `state.valves.stripper_product` | 5 s |
| 9 | Stripper Steam Valve | `stripper_steam.rs` (`StripperSteam`) | `valve.stripper_steam.position` | `state.valves.stripper_steam_valve` | 120 s |
| 10 | Reactor Cooling Water Flow | `reactor_cooling_water.rs` (`ReactorCoolingWater`) | `valve.reactor_cooling_water.position` | `state.valves.reactor_cooling_water` | 5 s |
| 11 | Condenser Cooling Water Flow | `condenser_cooling_water.rs` (`CondenserCoolingWater`) | `valve.condenser_cooling_water.position` | `state.valves.condenser_cooling_water` | 5 s |
| 12 | Agitator Speed | `agitator.rs` (`Agitator`) | `agitator.speed` | `state.valves.agitator_speed` | 5 s |

## Notas

**XMV-9 e XMV-11 (`stripper_steam.rs`, `condenser_cooling_water.rs`).** O vapor do stripper é a mais lenta das 12 (120 s); a água de resfriamento do condensador tem 5 s, como a maioria. O `docs/_deprecated/_deprecated_1.rs` rotula essas duas trocadas e dá os 120 s à água do condensador. Os valores daqui vêm de `VTAU(n)` no `teprob.f`, conferidos contra as equações físicas que consomem cada `VPOS(I)`.

**XMV-12 (`agitator.rs`).** O agitador é único na planta, por isso a chave não segue o padrão `valve.<nome>.position`: é `agitator.speed`.

**XMV-1 (`feed_d.rs`).** Foi a prova de conceito do `#[actuator(...)]` (`monjolo-macros`, branch `feat/proc-macro-components`): a mesma lei física da versão então escrita à mão de `feed_e` (mesmo τ, mesma fórmula), só que declarada como struct com atributos em vez de `Actuator::new()` com closure. `command` e `position` viram getters (`self.command()`, `self.position()`); `dynamics()` é código comum, nunca tocado pela macro — só a struct é reescrita. Hoje os 12 usam a macro. É o único atuador com testes (`tests/actuators/feed_d.rs`), que provam que a versão gerada se comporta igual à escrita à mão.
