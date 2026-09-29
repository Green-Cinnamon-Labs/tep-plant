# Feed (`src/units/feed.rs`) — o que cada parte faz e por quê

Este documento reúne o que antes estava em comentários dentro de `feed.rs`. O Feed representa as 4 alimentações externas do TEP — D, E, A e o feed combinado A&C — sem inventário próprio: nenhuma EDO, nenhum `#[state]`, porque são puramente vazão por posição de válvula, sem acumulação física nenhuma. Mesmo assim, cada uma tem identidade própria o bastante para merecer uma unidade: composição fixa (TEINIT), faixa de válvula (VRNG) e, para as duas medidas por massa (D e E), o próprio peso molecular. Os testes ficam em `tests/units/feed.rs`.

## Origem

O Feed foi migrado do antigo `dynamics/flows.rs` (issue 10). Antes, essas 4 vazões e os 2 pesos moleculares eram calculados dentro de `Flows::compute()`, sem nenhum dono próprio. As chaves publicadas (`flows.stream_flow.0..3`, `flows.d_feed_mol_weight`/`.e_feed_mol_weight`) continuam exatamente as mesmas — só quem as publica mudou.

## Constantes

`FEED_D_RANGE` (400), `FEED_E_RANGE` (400), `FEED_A_RANGE` (100) e `FEED_AC_RANGE` (1500) são a vazão máxima de cada válvula, com curva linear (posição% × faixa / 100), valores de VRNG em TEINIT.

`FEED_D_COMPOSITION`, `FEED_E_COMPOSITION` e `FEED_A_COMPOSITION` são as composições nominais dos feeds puros (TEINIT), na convenção A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7 de `physics/constants.rs`. Os três são `pub(crate)` porque o Compressor reusa esses valores no seu próprio balanço de massa e energia, em vez de duplicar os números.

`FEED_AC_COMPOSITION` (stream 4, o feed combinado A&C) também é `pub(crate)`, só pelo teste de `disturbance::idv1`, que precisa do valor nominal para montar a expectativa — quem consome de verdade (`units::stripper`) não importa isso direto: os distúrbios IDV(1), IDV(2) e IDV(8) perturbam exatamente essa composição (Table 8, Downs & Vogel 1993), então ela é publicada sob uma chave "nominal" que `disturbance::idv1::Disturbances::idv1` intercepta e reoferece sob a chave pública que o Stripper de fato lê — ver `docs/05-disturbios.md`. Só o IDV(1) está implementado por enquanto; o (2) e o (8) continuam pendentes, migrados um a um.

`FEED_TEMPERATURE` (45°C) é a temperatura nominal — os 4 feeds nascem todos a essa mesma temperatura, em TEINIT.

## Tarefas

**`d_feed_flow`, `e_feed_flow`.** Vazão linear na posição da válvula.

**`a_feed_flow`.** Vazão linear na posição da válvula. O IDV(6) (nominal = 0) atuaria aqui — fora de escopo por enquanto, mesma lacuna que já existia em `Flows`.

**`ac_feed_flow`.** Vazão linear na posição da válvula, mais `1e-10`. O IDV(7) (nominal = 0) atuaria aqui — mesma lacuna. O `1e-10` evita divisão por zero em quem usa este valor como denominador (o bloco 26 do flash split do stripper, `flow[3] / flow[10]`) quando a válvula está fechada — preservado do original.

**`ac_feed_composition`.** Publica a composição NOMINAL do feed combinado A&C (TEINIT) sob uma chave "nominal" própria, nunca a chave pública (`flows.stream4_composition`, que `units::stripper` de fato lê). O IDV(1) precisa desta grandeza nominal como `need` para poder interceptá-la e reofertá-la, alterada ou não, sob a chave pública — o Feed nunca sabe que existe distúrbio nenhum, só publica a condição de projeto, sempre igual. É o único lugar do sistema que CRIA uma `Mixture` do nada — matéria entrando de fora da planta.

**`d_feed_mol_weight`, `e_feed_mol_weight`.** O peso molecular médio de cada composição fixa, calculado uma vez por tick a partir das constantes do TEP.

**`xmeas_readings`.** As medidas XMEAS 1 a 4 (A/D/E/A&C Feed), com as conversões de unidade preservadas exatamente do original: 0,359/35,3145 (kmol/h → kscmh, para as correntes gasosas) e peso molecular × 0,454 (kmol/h → kg/h, para as duas medidas por massa).
