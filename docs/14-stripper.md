# Stripper (`src/units/stripper.rs`) — o que cada parte faz e por quê

Este documento reúne o que antes estava em comentários dentro de `stripper.rs`. O stripper recebe dois fluxos de entrada, o feed combinado A&C vindo do Feed (stream 3) e o underflow líquido do separador (stream 10), faz um flash (separação súbita) que divide essa mistura entre uma saída de vapor (stream 4, que volta ao compressor) e uma saída líquida (stream 11, o produto), e é aquecido ou resfriado por um reboiler condicional. Os testes ficam em `tests/units/stripper.rs`.

## Origem

O stripper foi a quarta unidade migrada para o scheduler de dataflow topológico (issue 10), depois de Feed, Compressor e Separator. Absorveu de `flows.rs` o bloco 22 (o slot 12, produto) e os blocos 25 a 28 (o flash split completo: entrada combinada A&C feed mais underflow do separador, a fração de split, e os slots 4 e 11 de saída). Absorveu de `heat.rs` o bloco 34 (condenser/reboiler) mais o próprio `condenser_ua` (bloco 22, UAC, que nunca teve dono próprio, só era usado ali mesmo). Absorveu de `derivatives.rs` a seção "Stripper" do balanço de massa e energia (bloco 40, YP(19..27)) e de `product_analyzer.rs` as XMEAS 37 a 41 (Product Analysis).

## Estado próprio

Diferente do reator e do separador, o stripper não separa vapor de líquido no seu estado: é um único campo `liquid: [f64; 8]` com os 8 componentes A–H, mais a entalpia. O `teprob.f` não modela uma fase vapor acumulada dentro do stripper — só o líquido retido é estado integrável.

## Constantes

`STRIPPER_PRODUCT_RANGE` (1000) é a faixa da válvula de produto (VRNG em TEINIT). `STRIPPER_STEAM_RANGE` (0,03) é a faixa da válvula de vapor de aquecimento (UAC).

## Tarefas

**`physical_state` (bloco 1).** O balanço de energia próprio: a partir do estado (o líquido retido e a entalpia) acha temperatura, volume, densidade e composição, igual ao `compute()` monolítico de antes.

**`product_flow` (bloco 2, ex-Flows, bloco 22, slot 12).** O produto do stripper é puramente linear na posição da válvula, sem acoplamento nenhum — mesmo padrão do underflow do separador.

**`flash_split` (bloco 3, ex-Flows, blocos 25 a 28).** O flash da entrada combinada: o feed A&C (stream 3) mais o underflow do separador (stream 10) formam a entrada do vaso de flash, e uma fração dessa entrada sai como vapor (stream 4, de volta ao compressor) e o resto como líquido (stream 11, o produto). A fração de split para A/B/C é fixa (SFR, em TEINIT — 0,995/0,991/0,990) e nunca recalculada; para D–H, a fração depende da própria temperatura do stripper (bloco 26 do `teprob.f`), através de um fator de temperatura com três regimes (acima de 170°C, abaixo de 5,292°C, ou a fórmula intermediária) e da razão entre a vazão de entrada A&C e a de underflow. Esse é o ponto onde um bug real já foi encontrado no histórico do projeto (documentado originalmente em `flows.rs`): se a vazão de underflow cai a zero ou perto disso, o A&C inteiro cai no líquido em vez de ser distribuído corretamente, inundando o stripper — o comportamento foi preservado tal qual está no `teprob.f`, sem correção, por fidelidade.

**`heat_exchange` (bloco 4, ex-Heat, bloco 34 mais o UAC do bloco 22).** O resfriamento condicional do reboiler: só troca calor se a temperatura do stripper estiver abaixo de 100°C. O `condenser_ua` (que nunca teve dono próprio no código original, só era usado aqui mesmo) vem da posição da válvula de vapor.

**`mass_and_energy_balance` (bloco 5, ex-Derivatives, bloco 40 YP(19..27)).** O balanço de massa e energia do próprio estado: o que entra como líquido do flash menos o que sai como produto, por componente. As entalpias são recomputadas frescas a partir de composição e temperatura, mesmo padrão das outras unidades — nada aqui lê uma entalpia publicada por outro componente.

**`product_analysis` (bloco 6, ex-product_analyzer.rs).** As XMEAS 37 a 41 (Product Analysis, stream 11): a composição líquida própria (a mesma que a válvula de produto escoa) convertida para mol%. Só os 5 componentes D–H saem como XMEAS — A, B e C não existem nessa corrente.

**`xmeas_readings` (bloco 7, ex-measured.rs, bloco 35).** As medidas XMEAS 15 (Stripper Level), 17 (Stripper Underflow, stream 11), 18 (Stripper Temperature) e 19 (Stripper Steam Flow), com as conversões preservadas do original. VTC = 156,5 (TEINIT) é a referência do nível.
