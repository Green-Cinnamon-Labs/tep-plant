# Compressor (`src/units/compressor.rs`) — o que cada parte faz e por quê

Este documento reúne o que antes estava em comentários dentro de `compressor.rs`. O compressor é o vaso que recebe o vapor de reciclo do separador (stream 8, o que não foi para a purga) e o mistura com os feeds frescos de A (stream 1), D (stream 2) e E (stream 3) e com o vapor vindo do flash do stripper (stream 5), formando o vapor que recicla de volta ao reator (stream 6, gerado por uma curva característica com anti-surge). Os testes ficam em `tests/units/compressor.rs`.

## Origem

O compressor foi a segunda unidade migrada para o scheduler de dataflow topológico (issue 10), depois de `Feed` — a primeira com `#[state]` própria e várias tarefas cruzando fronteira de outras unidades. Absorveu de `flows.rs` o bloco 23 (o slot 5, reciclo compressor→reator), o bloco 24 (a curva característica mais o anti-surge, slot 8 — fisicamente é separador→compressor, mas é o desempenho do próprio compressor que decide o quanto ele puxa, por isso o dono é este arquivo) e o bloco 31 (o slot 6, bypass, cópia do slot 5). Absorveu de `derivatives.rs` a seção "Compressor" do balanço de massa e energia (bloco 40, YP(28..36)).

## Estado próprio

Diferente do reator e do separador, o compressor não tem líquido: um único campo `vapor: [f64; 8]` com os 8 componentes A–H, mais a entalpia. Todo o inventário do compressor é vapor.

## Constantes

`COMPRESSOR_VESSEL_VOLUME` (5000) é o volume do vaso, em pé cúbico, a mesma unidade interna do `teprob.f`. `COMPRESSOR_FLOW_MAX` (280275) é a vazão mássica máxima do compressor, em kg/h. `COMPRESSOR_PRESSURE_RATIO_MAX` (1,3) limita a razão de pressão usada na curva característica.

## Tarefas

**`physical_state` (bloco 1).** O balanço de energia próprio: a partir do estado (o vapor retido e a entalpia) acha temperatura, pressão e composição, igual ao `compute()` monolítico de antes, agora uma tarefa entre várias.

**`outlet_flows` (bloco 2, ex-Flows, blocos 23/24/31).** Duas vazões: o reciclo ao reator (stream 6, `1937,6·√(pressão do compressor − pressão do reator)`) e a curva característica com anti-surge (stream 8) — fisicamente é separador→compressor, mas é o desempenho do próprio compressor que decide quanto ele puxa. A vazão mássica parte de `COMPRESSOR_FLOW_MAX` e é corrigida pela razão de pressão elevada ao cubo, depois descontada pela vazão de reciclo (que a válvula anti-surge controla). A entalpia do vapor do separador é recomputada fresca aqui, mesmo padrão já usado em `derivatives.rs` para as entalpias de feed. O `teprob.f` (bloco 31) calcula essa mesma vazão duas vezes sob dois índices internos diferentes (um "bypass", cópia bit a bit do outro) só por conveniência de implementação — como as duas são sempre idênticas e nada consumia o bypass separadamente, este código publica um único valor, sob a chave canônica (stream 6).

**`mass_and_energy_balance` (bloco 3, ex-Derivatives, bloco 40 YP(28..36)).** O balanço de massa e energia do próprio estado: os feeds frescos D, E e A entram diretamente (cada um com sua composição fixa do Feed), somados ao vapor do flash do stripper e ao reciclo do separador, menos o que sai pelo reciclo ao reator. É a tarefa que mais depende de vazões cruzadas de outras unidades — a física não muda em relação ao código original, só quem a hospeda.

**`reactor_feed_analysis` (bloco 4, ex-reactor_feed_analyzer.rs).** As XMEAS 23 a 28 (Reactor Feed Analysis, stream 6): a composição de vapor do próprio compressor — é essa composição, multiplicada pela vazão do stream 6, que forma o que entra no reator — convertida de fração molar para mol% (o `teprob.f` documenta essa medida como "Units = Mole %"). Só os 6 primeiros componentes (A–F) saem como XMEAS.

**`xmeas_readings` (bloco 5, ex-measured.rs, bloco 35).** As medidas XMEAS 5 (Recycle Flow, stream 8), 6 (Reactor Feed Rate, stream 6) e 20 (Compressor Work), todas vazões ou potência do próprio compressor. A XMEAS 16 (Stripper Pressure) é a exceção conhecida do original: o nome sugere o stripper, mas o valor É a pressão do compressor — não é bug de digitação, é preservado tal qual do `teprob.f`, sob a mesma chave externa `xmeas.stripper.pressure` de sempre.
