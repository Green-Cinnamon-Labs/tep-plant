# Separador (`src/units/separator.rs`) — o que cada parte faz e por quê

Este documento reúne o que antes estava em comentários dentro de `separator.rs` (o código ficou só com o código, no mesmo padrão de `docs/12-reator.md`). O separador é um vaso de volume fixo, com vapor e líquido, que recebe o vapor descarregado pelo reator (stream 7), separa uma parte para reciclo ao compressor (stream 8), purga outra parte (stream 9) e escoa o líquido para o stripper (stream 10, underflow). Os testes ficam em `tests/units/separator.rs`.

## Origem

O separador foi a terceira unidade migrada para o scheduler de dataflow topológico (issue 10), depois de Feed e Compressor. Absorveu de `flows.rs` os blocos 22 e 25 (os slots 9 e 10, purge e underflow), de `heat.rs` o bloco 33 (a troca térmica do separador), de `derivatives.rs` a seção "Separador" do balanço de massa e energia (bloco 40, YP(10..18)) e de `purge_analyzer.rs` as XMEAS 29 a 36 (Purge Gas Analysis).

## Estado próprio

O estado tem 9 números, no mesmo split do reator e pelo mesmo motivo: vapor A/B/C, líquido D–H e a entalpia total, divididos em 3 campos porque a chave de configuração não é uniforme entre eles (`state.separator_vapor.*` para os 8 componentes, `state.separator.energy` para a entalpia).

## Constantes

`SEPARATOR_VOLUME` (3500) é o volume total do vaso, em pé cúbico, a mesma unidade interna do `teprob.f` que o reator usa. `SEPARATOR_UNDERFLOW_RANGE` (1500) é a faixa da válvula de underflow (VRNG em TEINIT).

## A temperatura de retorno da água de resfriamento (`tws`) é uma constante congelada

`SEPARATOR_COOLING_WATER_RETURN` (77,29698353) é, assim como o `REACTOR_COOLING_WATER_RETURN` do reator, fiel ao `teprob.f` original: o bloco 40 do modelo monolítico (`v1.0.0`) marca a derivada de `tws` como sempre zero ("tws kept at snapshot value"), então não existe balanço de calor quase-estático nenhum a resolver aqui — só o valor congelado que o snapshot inicial semeou.

Antes de uma correção em 2026-09-15, essa constante era 40,0 — o `s_zero` do canal de distúrbio 5 (TCWS, a temperatura de ENTRADA da água de resfriamento do separador), não o `tws` de saída. É o mesmo tipo de erro de troca entrada/saída já corrigido em `Reactor::heat_exchange`, achado ao comparar contra `application.toml`/`te_exp3_snapshot.toml`: `[state.cooling].separator_water_temp = 77,29698353` é o valor congelado de verdade. A consequência do erro era o separador resfriar mais forte que o correto, empurrando entalpia fria para o reciclo do compressor — um dos dois termos dominantes do balanço de energia do reator — o que contribuía para o colapso rápido de temperatura do reator observado nos experimentos (120°C para ~42°C em cerca de 21 minutos simulados), mesmo já com a correção do reator sozinha aplicada.

## Tarefas

**`physical_state` (bloco 1).** O balanço de energia próprio: a partir do estado (vapor, líquido e entalpia) acha temperatura, pressão, volume de líquido, densidade e composição, igual ao `compute()` monolítico de antes, agora uma tarefa entre várias. A pressão parcial de A/B/C vem do gás ideal a partir dos moles de vapor, e a de D–H vem de Antoine multiplicada pela fração líquida.

**`outlet_flows` (bloco 2, ex-Flows, blocos 22/25).** A purga (slot 9) depende da pressão e da composição próprias do separador; o underflow (slot 10) é puramente linear na posição da válvula, sem acoplamento nenhum, mas fica junto por ser a outra saída direta do vaso.

**`heat_exchange` (bloco 3, ex-Heat, bloco 33).** A troca térmica do separador: o UAS depende da vazão reator→separador. A temperatura de referência é a do REATOR, não a do próprio separador — `TST(8)` aponta para o reator no `teprob.f`, bloco 20 — preservado por fidelidade ao original, mesmo sendo contraintuitivo à primeira vista.

**`mass_and_energy_balance` (bloco 4, ex-Derivatives, bloco 40 YP(10..18)).** O balanço de massa e energia do próprio estado: entrada do reator menos as três saídas (reciclo ao compressor, purga, underflow), por componente, e o mesmo para a entalpia. A entalpia do vapor do separador aqui é calculada SEM a correção que o bloco 24 do compressor aplica — é o que `HST(10)` preserva no original, por ter sido copiada antes daquela correção rodar.

**`purge_analysis` (bloco 5, ex-purge_analyzer.rs).** As XMEAS 29 a 36 (Purge Gas Analysis, stream 9): a mesma composição de vapor que alimenta o reciclo na stream 8 (bloco 27 do `teprob.f`, onde `FCM(I,9)`/`FCM(I,8)` usam o mesmo `XST(.,9) = XST(.,8)`), convertida de fração molar para mol%.

**`xmeas_readings` (bloco 6, ex-measured.rs, bloco 35).** As medidas XMEAS 10 (Purge Rate, stream 9), 11 a 13 (temperatura, nível e pressão do separador), 14 (Separator Underflow, stream 10) e 22 (temperatura de saída da água de resfriamento), com as conversões preservadas do original.
