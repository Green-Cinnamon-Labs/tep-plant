# Reator (`src/units/reactor.rs`) — o que cada parte faz e por quê

Este documento reúne o que antes estava em comentários dentro de `reactor.rs` (o código ficou só com o código). O reator é um vaso de volume fixo com uma mistura em duas fases, vapor e líquido. Ele recebe o vapor de reciclo do compressor, faz as reações químicas dentro do vaso, troca calor com a água de resfriamento, descarrega vapor para o separador e publica as medidas (XMEAS) que sensores e controladores leem. Os testes ficam em `tests/units/reactor.rs` (a pasta `tests/units/` espelha `src/units/`, com um `harness.rs` que roda cada tarefa isolada por chave).

## Origem

O reator foi a quinta e última unidade migrada para o scheduler de dataflow topológico (issue 10), o que fechou a migração. Ele absorveu de `flows.rs` o bloco 22 (o fator de agitação AGSP, fundido direto na troca térmica sem publicar chave própria, mesmo tratamento do `condenser_ua` em `units::stripper`) e o bloco 23 (a vazão do slot 7, reator para separador). Absorveu de `heat.rs` o bloco 32 (a troca térmica do reator) e de `derivatives.rs` a seção "Reator" do balanço de massa e energia (bloco 40, YP(1..9)), a última EDO que ainda não calculava a própria derivada. Os arquivos `flows.rs`, `heat.rs` e `derivatives.rs` foram deletados junto com essa migração.

## Estado próprio

O estado integrável do reator tem 9 números: vapor A/B/C, líquido D–H e a entalpia total. Eles estão divididos em 3 campos porque a chave de configuração não é uniforme entre os 9: vapor e líquido usam `state.reactor_vapor.*` e a entalpia usa `state.reactor.energy`. A divisão é por padrão de chave, não por fase físico-química. Esses são os únicos campos com `#[state]`, `#[config]` e `#[offer]`; todo o resto do reator conversa por sinais dentro dos métodos.

## Como os sinais funcionam

Cada método marcado com `#[task]` lê um sinal com `need::nome` e escreve com `offer::nome = valor;`. O nome do identificador é a chave, e `__` vira `.`, então `reactor__temperature` é a chave `reactor.temperature`. Uma composição é uma `Mixture` lida ou escrita como um valor só, com a fase no fim (`::<Vapor>`), e a macro a publica como 8 chaves `.a` a `.h`. O tamanho e o catálogo de espécies vêm de `#[monjolo::tasks(species = TEP_SPECIES, len = 8)]`. A ordem de execução entre as tarefas sai do que cada uma lê e escreve.

## Constantes

`REACTOR_VOLUME` (1300 m³) é o volume total do vaso. `GAS_CONSTANT` (998,9) é o R dos gases em mmHg·m³/(kmol·K). `TEMPERATURE_SEED` (120) é o chute inicial do Newton-Raphson que acha a temperatura a partir da entalpia; ele não afeta a raiz. `REACTION_FACTOR_1_NOMINAL` e `REACTION_FACTOR_2_NOMINAL` (ambos 1,0) são os fatores multiplicativos das reações 1 e 2 que o distúrbio IDV(13) perturba; hoje são constantes e deveriam virar um `need` de um Disturbance quando o IDV(13) for implementado.

## As reações (`TEP_REACTIONS`)

São 4 reações sobre as espécies A a H (índices 0 a 7). Cada linha de `stoichiometry` diz quanto de cada espécie é consumido (negativo) ou produzido (positivo) por unidade de avanço da reação, e `enthalpies` diz o calor liberado por unidade de avanço, em kJ/kmol. As reações são: 1) A + C + D → G, 2) A + C + E → H, 3) A + E → F, 4) 1,5 D → F. Só as reações 1 e 2 liberam calor no modelo original (`teprob.f`), por isso as duas últimas têm entalpia zero.

## Cinética (`kinetics`)

Calcula a taxa bruta de cada uma das 4 reações a partir da temperatura e das pressões parciais, com a forma de Arrhenius do `teprob.f`, multiplicada pelo volume de vapor. As reações 1 e 2 dependem das pressões parciais de A e C (com expoentes 1,1544 e 0,3735) e da pressão parcial de D ou E; se A ou C não têm pressão parcial positiva, as duas taxas são zero. A estequiometria (quem consome e produz o quê) e o calor total ficam por conta do `Reaction`, não de quem chama.

## Tarefas

**`physical_state` (bloco 1).** É o balanço de energia próprio: a partir do estado (vapor, líquido e entalpia) acha temperatura, pressão, volume de líquido, composição do vapor e a cinética das reações. Não lê nenhum `need::`, só o próprio estado. `vapor()` e `liquid()` são os dois grupos do estado (A/B/C e D–H); `Mixture::at` posiciona cada um no seu lugar de um total de 8, com zero no resto. A pressão parcial de A/B/C vem do gás ideal a partir dos moles de vapor, e a de D–H vem de Antoine multiplicada pela fração líquida. Como cada `Mixture` só é diferente de zero na sua própria faixa de índices, a soma `+` já junta as duas.

**`flow_to_separator` (bloco 2, ex-Flows, slot 7).** A vazão para o separador depende da diferença de pressão entre reator e separador, sem válvula. O fator `(1 − 0,25·0)` é o canal de distúrbio 11, deixado neutro.

**`heat_exchange` (bloco 3, ex-Heat, bloco 32).** A troca térmica entre o reator e a água de resfriamento. O UARLEV é a fração da serpentina submersa: zero abaixo de nível 10 (seca, sem troca), rampa linear até nível 50 e platô em 1,0 dali para cima (totalmente submersa, mais líquido não aumenta mais nada). O nível vem do volume de líquido dividido por 7,8, um fator de conversão deste bloco. O fator `(1 − 0,35·0)` é o canal de distúrbio 9 (IDV 17), deixado neutro. A temperatura de retorno da água é a constante congelada `REACTOR_COOLING_WATER_RETURN`.

**`mass_and_energy_balance` (bloco 4, ex-Derivatives, bloco 40 YP(1..9)).** O balanço de massa e energia do próprio estado: entrada do compressor menos saída para o separador mais a reação, por componente, e o mesmo para a entalpia com o calor de reação e o calor trocado. As entalpias são recomputadas frescas a partir de composição e temperatura, mesmo padrão das outras unidades.

**`xmeas_readings` (bloco 5, ex-measured.rs, bloco 35).** As medidas XMEAS 7 a 9 (pressão, nível e temperatura do reator) e a 21 (temperatura de saída da água de resfriamento), com as conversões preservadas do original: (P − 760)/760 · 101,325 (mmHg manométrico para kPa manométrico) e volume/666,7 · 100 (calibração do instrumento de nível).

## A temperatura de saída da água de resfriamento (TWR) é uma constante congelada

`REACTOR_COOLING_WATER_RETURN` (94,59927549) é fiel ao FORTRAN original: em `TEFUNC`, `YP(37)` nunca é atribuído e o TWR fica constante no valor de inicialização, que é também o `[state.cooling].reactor_water_temp` de `application.toml`. É o mesmo padrão de `Separator::heat_exchange` (`SEPARATOR_COOLING_WATER_RETURN`).

Existiu aqui uma fórmula quase-estática (o `twr` como média ponderada de `uar`, `tcr` e a água de entrada, a partir da abertura da válvula `valve.reactor_cooling_water.position`, XMV 10) que foi removida em 2026-09-16, no Exp 24. Não foi uma regressão: foi a correção de um erro estrutural descoberto ao investigar por que a `v1.0.0`, usada como "ground truth" nos Exp 18–23, tinha essa fórmula ativa. A arqueologia, via `git log --follow` em `tennessee-eastman-service/core/src/dynamics/tep/model.rs`, foi esta:

- 2026-03-09 (`b4c2077`): existia uma EDO própria para `twr`/`tws` (não a fórmula quase-estática, uma dinâmica de primeira ordem ainda mais forte). Foi revertida por "causar colapso da temperatura de resfriamento para ~35°C, destabilizando o balanço de energia do reator". O fiel ao FORTRAN (`YP(37)` nunca atribuído) é o TWR congelado, sem exceção.
- 2026-05-27 (`123a6a3`): a fórmula quase-estática foi introduzida de novo, especificamente para dar ao IDV(4) algum efeito observável (com `twr` congelado, o IDV(4) literalmente não faz nada, o que estava emperrando o Exp 14 daquela época).
- 2026-05-28: a tag `v1.0.0` foi cortada, um dia depois, capturando a fórmula quase-estática ainda ativa e não validada.
- 2026-06-01 (`ad08ea0`, "EXP14 Done"): a mesma fórmula foi comentada de volta para `twr` congelado. O autor documentou o motivo: "quando tcr cai abaixo de ~67°C o balanço produz twr < tcr → QUR < 0 (trocador 'aquece' o reator), comportamento ausente no FORTRAN original".

Ou seja, a mesma classe de mecanismo foi tentada e revertida duas vezes por instabilidade, e a `v1.0.0` que este repositório vinha usando como referência validada é, por coincidência de datas, o único ponto da história onde ela ficou ativa. Isso foi confirmado empiricamente contra `docs/simulations/simulation_log_13.csv`: ao longo de 20 horas sem distúrbio, `reactor.temperature` (XMEAS(9)) varia cerca de 0,49°C, mas XMEAS(21) (o `twr`) varia só cerca de 0,076°C, bem menos do que a fórmula quase-estática preveria (um peso de ~0,69 em `tcr` implicaria ~0,34°C de variação em `twr`) e plenamente consistente com `twr` congelado mais ruído de medição (σ = 0,01). O próprio baseline validado nunca usou essa fórmula. Como consequência, `valve.reactor_cooling_water.position` deixou de ser uma entrada da troca térmica do reator.

A fórmula removida, preservada aqui só como referência (nunca compilada):

```rust
let fcwr = cooling_water_position * REACTOR_COOLING_WATER_RANGE * 0.001;
let cw_capacity = fcwr * REACTOR_COOLING_WATER_CAPACITY; // REACTOR_COOLING_WATER_CAPACITY = 0.00942
let total_capacity = cw_capacity + uar;
let twr = if total_capacity > 1e-12 {
    (cw_capacity * REACTOR_COOLING_WATER_INLET + uar * reactor_temperature) / total_capacity // REACTOR_COOLING_WATER_INLET = 38.5
} else {
    reactor_temperature
};
```

## Onde os distúrbios entram no reator

Hoje o reator tem dois pontos de distúrbio deixados neutros: o fator `(1 − 0,25·0)` na vazão para o separador (canal 11) e o fator `(1 − 0,35·0)` no calor trocado (canal 9, IDV 17), além dos dois `REACTION_FACTOR_*_NOMINAL` do IDV(13). A tabela completa de onde cada IDV entra no código está em `docs/05-disturbios.md`.
