# Registro Científico de Intervenções — TEP Digital Twin

Estrutura de cada entrada: **Observação → Hipótese → Intervenção → Resultado → Conclusão**

---

## Experimento 1 — Instrumentação dos gráficos de startup

### Observação

Com `ramp_duration=2.0h` e `real_time=false`, a simulação avançou até t≈1.32h antes de
disparar ISD por `Reactor Lv low (<10%)`. O reator drenava continuamente de ~77% para 4.5%
enquanto o feed era rampado de 0% → nominal ao longo de 2 horas.

Os gráficos disponíveis não mostravam:
- A abertura das válvulas de feed (XMV(1–4)) ao longo do tempo — era impossível ver a
  progressão do ramp visualmente
- O acoplamento entre pressão do reator, fluxo de purge e abertura da válvula de purge
  (três variáveis distribuídas em painéis separados e não relacionados)
- Marcos temporais de fase do startup (25 / 50 / 75 / 100 % do `ramp_duration`)

### Hipótese

Adicionando um painel de **Feed Valve Ramp** (XMV(1–4) em %) e combinando
**Purge Flow + Purge Valve** em um único painel diagnóstico, além de marcadores verticais
de fase, será possível:
- Identificar em que fração do ramp o reator perde inventário crítico
- Observar se o purge controller contribui para a drenagem ao fechar prematuramente a
  válvula em resposta à queda de pressão (resultado esperado: sim, pela fórmula
  `mv[5] = 40.06 + 0.10*(P − 2705)` que fecha a válvula quando P < 2705 kPa)

### Intervenção

Arquivo modificado: `analysis/tep_analysis/plot.py`

- Substituídos os painéis "Recycle & Purge Flow" e "Purge Valve (MV)" por três novos painéis:
  - **Feed Valve Ramp** — XMV(1), XMV(2), XMV(3), XMV(4) em %, todos no mesmo eixo
  - **Recycle Flow** — XMEAS(5) isolado
  - **Purge: Flow & Valve** — XMEAS(10) [kscmh] + XMV(6) [%] sobrepostos (painel diagnóstico;
    unidades heterogêneas, mas tendências diretamente comparáveis)
- Adicionado argumento CLI `--ramp <horas>` que desenha linhas verticais em
  25 / 50 / 75 / 100 % do `ramp_duration` em todos os painéis

### Resultado

A hipótese foi parcialmente confirmada. Os novos painéis de fato tornaram o mecanismo do startup visível: o Feed Valve Ramp mostrou claramente que, com `ramp_duration=2.0h`, as alimentações ainda estavam longe do nominal quando o reator perdeu inventário crítico; e o painel conjunto de **Purge Flow + Purge Valve** mostrou de forma explícita o acoplamento causal entre queda de pressão e fechamento da purge. No instante do shutdown, a simulação parou em **t = 1.320 h**, com **Reactor Level = 4.5%**, **Reactor Pressure ≈ 2997.6 kPa**, **Purge Valve = 69.36%** e **Purge Flow ≈ 3000 kscmh**, o que indica que o evento terminal foi de fato low level do reator, não runaway térmico nem overpressure.

### Conclusão

A conclusão principal é: **o problema dominante deste experimento não foi mais a explosão inicial, e sim drenagem excessiva do reator durante uma rampa de feed longa demais**. A instrumentação nova validou isso. Ela também mostrou que a hipótese específica “o purge controller fecha prematuramente e contribui para a drenagem” não explica o shutdown final deste run como causa principal. O purge até fecha no começo quando a pressão cai, mas, no fim do experimento, ele está amplamente aberto e a planta ainda assim atinge low level. Portanto, a leitura mais forte agora é: **com `ramp_duration=2.0h`, a reposição de massa pelas alimentações é lenta demais para sustentar o inventário do reator ao longo do startup. Em termos de próximo experimento, a ação mais coerente é reduzir `ramp_duration` e repetir o teste mantendo a mesma instrumentação.**

---

## Experimento 2 — Redução do `ramp_duration` para 0.5 h

### Observação

Com `ramp_duration=2.0h`, a simulação disparou ISD por `Reactor Lv low (<10%)` em **t = 1.320 h** (t_operational = 1 186 h). O reator drenava de ~77% para 4.5% enquanto o feed estava em apenas ~13% do nominal no momento do shutdown. O purge valve (XMV(6) = 69.36%) e o purge flow (XMEAS(10) ≈ 3000 kscmh) estavam amplamente abertos no instante final — confirmando que a drenagem não foi causada pelo fechamento da purge, mas pela insuficiência de feed durante a rampa longa.

### Hipótese

Com `ramp_duration=2.0h`, o feed nominal demora 2 horas para chegar ao reator. O inventário inicial (~77% de nível) não sustenta essa espera: a análise indica taxa de drenagem de ~55 %/h, ou seja, em t≈1.25h o nível atinge o limite ISD de 10 %.

Reduzindo para `ramp_duration=0.5h`, o feed nominal chega em 30 minutos. A estimativa conservadora é que o reator estará em ~50 % de nível quando o feed pleno entrar, com margem suficiente para recuperar antes de atingir o limite inferior.

### Intervenção

Arquivo modificado: `tennessee-eastman-service/service/src/main.rs`

```rust
ramp_duration: 0.5,   // reduzido de 2.0 → 0.5 h
```

### Resultado

Com `ramp_duration=0.5h`, o reator atingiu nível mínimo de **~59%** durante o cold start (margem ampla acima do limite ISD de 10%), e a simulação rodou por **13+ horas** sem disparar ISD. Após t≈3h, todos os principais estados convergiram para valores estáveis: pressão ~2680 kPa, temperatura ~120°C, sep/stripper levels ~50%, deriv_norm ~200–300 unidades/h. Observou-se uma deriva lenta no nível do reator (~0.6 %/h, de ~59% → ~69% ao longo de 13h) com taxa decrescente (côncava para baixo), sugerindo convergência assintótica.

O operating point encontrado difere do Mode 1 nominal: recycle flow ~26 kscmh (vs ~35 nominal), A Feed ~22% (vs ~63% nominal), pressão ~2680 kPa (vs 2705 kPa nominal).

### Conclusão

Hipótese confirmada: `ramp_duration=0.5h` é suficiente para evitar a drenagem crítica do reator durante o cold start. A planta atingiu um regime operacional estável e sustentável dentro dos limites ISD.

O **attractor encontrado não é o Mode 1 SS** pelos seguintes motivos: (a) IDV(4) está ativo (+5°C no CW do reator), o que desloca o ponto de equilíbrio; (b) nossos 3 controladores proporcionais não fixam completamente o operating point — o nível do reator não tem malha fechada. A deriva lenta residual (~0.6 %/h) indica que o sistema ainda não convergiu completamente; provavelmente precisaria de ~20h adicionais para estabilizar totalmente.

**Próximo passo natural:** Remover IDV(4) para isolar o comportamento sem distúrbio, rodar por tempo determinado (20h) e capturar o estado final como novo `initial_state.toml` — base para experimentos de controle futuros.

---

## Experimento 3 — Busca do SS sem distúrbio + snapshot de estado

### Observação

O Experimento 2 encontrou um attractor operável, mas com IDV(4) ativo e sem controlador de nível do reator. Não há evidência de que esse attractor coincide com o Mode 1 SS do FORTRAN. Adicionalmente, a simulação não tinha limite de tempo configurável nem capacidade de exportar
o estado final — o operador precisava interromper manualmente e não havia forma de reusar o estado para reinicializações.

### Hipótese

Removendo IDV(4) e rodando por 20h de tempo simulado, a planta deve convergir para um attractor mais próximo do Mode 1 SS (sem a perturbação do CW do reator). Capturando o vetor YY ao final como TOML, teremos um estado inicial pré-estabilizado que elimina o cold start transiente em
experimentos futuros.

### Intervenção

Arquivos modificados:
- `tennessee-eastman-service/service/src/config.rs` — novos campos `max_sim_time_h` e `snapshot_path`
- `tennessee-eastman-service/service/src/runtime.rs` — lógica de parada por tempo + escritor de snapshot TOML
- `tennessee-eastman-service/service/src/dashboard.rs` — fix label "s" → "h" no título
- `tennessee-eastman-service/service/src/main.rs` — config do Exp 3: sem IDV, 20h, snapshot habilitado

```rust
active_idv: vec![],                                  // sem distúrbio
max_sim_time_h: Some(20.0),                          // parar em t=20h
snapshot_path: Some("cases/te_exp3_snapshot.toml"),  // salvar estado final
```

### Resultado

A simulação completou 20 h de tempo simulado sem disparar ISD (clean exit). O snapshot foi escrito com sucesso em `cases/te_exp3_snapshot.toml`.

**Estado final em t = 20.0 h:**

| Variável               | Exp 3 (t=20h)  | Exp 2 (t=13h)  | Mode 1 nominal |
| ---------------------- | -------------- | -------------- | -------------- |
| Pressão reator         | ~2680 kPa      | ~2680 kPa      | 2705 kPa       |
| Temperatura reator     | ~120 °C        | ~120 °C        | 122.9 °C       |
| Nível reator           | ~69 % (deriva) | ~69 % (deriva) | —              |
| Purge valve (XMV(6))   | 39.07 %        | ~40 %          | 40.06 %        |
| Sep underflow (XMV(7)) | 36.83 %        | ~38 %          | 38.10 %        |
| CW temp reator         | 94.60 °C       | —              | 94.60 °C       |
| CW temp separador      | 77.30 °C       | —              | 77.30 °C       |

O attractor encontrado em Exp 3 é **virtualmente idêntico ao de Exp 2**: a remoção do IDV(4) não alterou o ponto de operação. As temperaturas de água de resfriamento convergiram exatamente para os valores nominais do FORTRAN TEINIT (ausência de IDV(4) → sem forçamento externo). O nível do reator continuava em deriva lenta positiva ao final de 20h, porém a taxa era decrescente (côncava para baixo), indicando convergência assintótica ainda em curso. O `deriv_norm` atingiu o mínimo histórico observado até o momento.

### Conclusão

A hipótese foi **parcialmente confirmada**: sem IDV(4), as CW temps voltaram ao nominal do FORTRAN — confirmando que o IDV(4) perturbava o equilíbrio térmico. Contudo, o ponto de operação macro (pressão, recycle flow, temperatura do reator) permaneceu o mesmo do Exp 2, demonstrando que **o attractor é determinado pelos setpoints dos 3 controladores proporcionais, não pelo distúrbio IDV(4)**.

A divergência em relação ao Mode 1 nominal do FORTRAN (2680 kPa vs 2705 kPa, recycle ~26 kscmh vs ~35 kscmh) é, portanto, estrutural: os controladores P atuam em setpoints fixos que não reproduzem exatamente o ponto de operação do FORTRAN. Isso não é um problema — é o attractor natural da nossa planta com a camada de controle atual.

O snapshot `cases/te_exp3_snapshot.toml` é a melhor condição inicial disponível: está muito mais próximo do SS do que o FORTRAN TEINIT, elimina o transiente de cold start e tem CW temps corretas. **Próximo passo natural: validar o snapshot como ponto de partida direto (ramp_duration=0, sem cold start), confirmando que a planta mantém operação estável sem o transiente de inicialização.**

---

## Experimento 4 — Validação do snapshot como ponto de partida direto

### Observação

O Exp 3 gerou `cases/te_exp3_snapshot.toml` em t = 20.0 h. Esse vetor de estado está próximo do attractor da nossa camada de controle — CW temps no nominal, válvulas de feed nos valores nominais de FORTRAN, controladores P estabilizando pressão e níveis. Nunca foi testado se esse snapshot pode substituir completamente o cold start: se `initial_state_path = te_exp3_snapshot.toml` com `ramp_duration = 0.0` produz operação estável imediata ou se o estado YY exportado introduz inconsistências algébricas que causam ISD.

### Hipótese

Com `initial_state_path = te_exp3_snapshot.toml` e `ramp_duration = 0.0` (feed valves já no valor nominal desde t=0), a planta deve iniciar diretamente no attractor sem transiente de cold start, mantendo todos os estados dentro dos limites ISD desde o primeiro passo. O deriv_norm deve partir do mesmo patamar que terminamos no Exp 3 (~200–300 unidades/h) e continuar decaindo. Se a simulação sobreviver 5h sem alarme, o snapshot é validado como condição inicial reutilizável.

### Intervenção

Arquivo modificado: `tennessee-eastman-service/service/src/main.rs`

```rust
initial_state_path: "cases/te_exp3_snapshot.toml".into(), // boot from Exp 3 snapshot
ramp_duration: 0.0,                                        // no cold start (Exp 4)
active_idv: vec![],                                        // no disturbances
max_sim_time_h: Some(5.0),                                 // stop at t=5h
snapshot_path: Some("cases/te_exp4_snapshot.toml".into()), // save final state
```

### Resultado

A simulação completou 5 h sem disparar ISD. O snapshot foi escrito em `cases/te_exp4_snapshot.toml`.

**Comportamento observado:**

- **Feed Valve Ramp**: todas as 4 válvulas de feed constantes desde t=0 (D≈63%, E≈54%, A≈25%, A&C≈61%) — sem transiente de cold start
- **deriv_norm**: spike para ~20 000 unidades/h exatamente em t=0 (inconsistência algébrica de inicialização do integrador RK4), resolvido em <0.01 h, retornando a ~200–300 unidades/h
- **Reactor Level**: plano em ~69% durante toda a run — sem a deriva lenta observada nos Exps 2 e 3 (já estamos no attractor)
- **Reactor Pressure / Temperature**: absolutamente estáveis em ~2690 kPa e ~120 °C
- **Sep & Stripper Levels**: estáveis em ~48% com ruído de pequena amplitude
- **Purge valve**: ~39% constante; Purge Flow: ~0.4 kscmh constante
- **Recycle Flow**: média ~26.8 kscmh com **oscilações persistentes de alta frequência** (~±0.5 kscmh, amplitude consistente ao longo de 5h)

### Conclusão

Hipótese **confirmada**: o snapshot `te_exp3_snapshot.toml` é uma condição inicial válida e o boot direto com `ramp_duration=0.0` é seguro. A planta inicia diretamente no attractor sem transiente de cold start e sem acionar nenhum alarme ISD.

O spike de `deriv_norm` em t=0 não representa instabilidade — é um artefato de inicialização algébrica do RK4 que se dissipa em menos de 0.01 h simulado. A ausência de deriva no nível do reator confirma que o Exp 3 realmente convergiu para o attractor antes de terminar.

**Observação pendente — oscilações do recycle flow**: As oscilações de ~±0.5 kscmh em recycle são persistentes e não atenuam ao longo de 5h. Duas hipóteses: (a) são dinâmica inerente da planta nesse ponto de operação (oscilações naturais do loop de reciclo); ou (b) são artefato numérico do RK4 com dt=0.001 h. Essa questão deve ser investigada no Exp 5.

**Base estabelecida**: a partir deste experimento, todos os experimentos futuros podem usar `te_exp3_snapshot.toml` com `ramp_duration=0.0` como ponto de partida padrão, eliminando os primeiros 0.5h de overhead de cold start.

---

## Experimento 5 — Investigação das oscilações do recycle flow

### Observação

No Exp 4, o recycle flow (XMEAS(5)) exibiu oscilações persistentes de ~±0.5 kscmh ao longo de toda a run de 5h, sem amortecimento visível. Todos os outros sinais monitorados (pressão, temperatura, níveis, purge) são estáveis. Não é possível determinar, apenas observando o plot, se essas oscilações são física real ou artefato numérico do integrador RK4 com dt=0.001 h.

### Hipótese

Se as oscilações forem artefato numérico do passo de integração, reduzir dt de 0.001 h para 0.0001 h deve reduzir ou eliminar as oscilações no recycle flow enquanto mantém todos os demais estados estáveis. Se as oscilações persistirem com dt menor, são dinâmica real da planta.

### Intervenção

Arquivo modificado: `tennessee-eastman-service/service/src/main.rs`

```rust
dt: 0.0001,   // reduzido de 0.001 h → 0.0001 h (10× menor)
```

Todos os demais parâmetros idênticos ao Exp 4 (snapshot Exp 3, ramp=0, 5h, sem IDV).

### Resultado

Com dt reduzido 10×, as oscilações de XMEAS(5) persistiram com amplitude e padrão visual idênticos ao Exp 4: mesma faixa (~26.2–27.8 kscmh), mesma rugosidade, mesma persistência ao longo de 5h. Os demais sinais (pressão, temperatura, níveis, purge) permaneceram estáveis e lisos. Adicionalmente, o `deriv_norm` também exibiu oscilações persistentes ao longo de toda a run (faixa aproximada 500–5000 unidades/h), com o mesmo padrão temporal que XMEAS(5). Esse detalhe é relevante: `deriv_norm = max‖dy/dt‖` é calculado diretamente dos derivativos dos 50 estados ODE, sem qualquer componente de ruído de medição.

### Conclusão

A hipótese "as oscilações são artefato numérico do passo de integração RK4" foi **refutada** (ou pelo menos fortemente enfraquecida): a amplitude não variou com redução de 10× no dt.

O fato de `deriv_norm` oscilar em sincronia com XMEAS(5) é evidência de que **o vetor de estado em si está oscilando**, não apenas a medição. Isso elimina a hipótese de ruído puro de medição como causa dominante e aponta para dinâmica real do loop de reciclo — provavelmente uma oscilação não amortecida emergente da interação entre o controlador P de pressão (purge valve) e a dinâmica do compressor/reciclo nesse ponto de operação.

A próxima separação necessária é: **qual estado ODE específico está oscilando?** A hipótese mais forte é que são os holdups de vapor do compressor (UCVV, estados YY[27–34]) que alimentam o cálculo de XMEAS(5). Se o estado interno for liso e apenas XMEAS(5) oscilar, é ruído de medição do modelo; se UCVV oscilar com a mesma frequência, a oscilação é física.

---

## Experimento 6 — Identificação do estado ODE oscilante

### Observação

O Exp 5 demonstrou que o `deriv_norm` oscila em sincronia com XMEAS(5) (recycle flow), indicando que pelo menos um dos 50 estados ODE está oscilando. O CSV atual registra apenas XMEAS(1–22) e XMV(1–12) — não há logging dos componentes individuais do vetor de estado YY que permitiria identificar qual estado específico carrega a oscilação.

### Hipótese

A oscilação está nos holdups de vapor do vaso/compressor (UCVV, YY[27–34]) que são as variáveis de estado diretamente ligadas ao fluxo de reciclo. Se UCVV oscilar com a mesma frequência e fase que XMEAS(5), a oscilação é uma dinâmica real do loop de reciclo. Se UCVV for liso e apenas XMEAS(5) oscilar, o ruído vem da camada de medição (TESUB8 no modelo FORTRAN/Rust).

### Intervenção

Arquivo modificado: `tennessee-eastman-service/service/src/runtime.rs`

Adicionados ao header e às linhas do CSV:
- **YY[0..10]** — UCVR (holdups de vapor do reator, componentes A–H) + ETR (energia do reator): permite detectar se a oscilação nasce no reator antes de se propagar para o reciclo
- **YY[27..35]** — UCVV (holdups de vapor do compressor/vaso, componentes A–H) + ETV (energia do vaso): estado interno que alimenta diretamente o cálculo de XMEAS(5)

`dt` revertido para 0.001 h e snapshot renomeado para `te_exp6_snapshot.toml`.

### Resultado

**Σ UCVR (reator, YY[0–7]):** tendência monotônica crescente de ~350.4 → ~352.5 kmol ao longo de 5h (+2.1 kmol, ~0.42 kmol/h). Curva completamente lisa — nenhuma oscilação de alta frequência visível.

**Σ UCVV (compressor, YY[27–34]):** spike de inicialização em t≈0.1h (~+12 kmol, artefato algébrico do RK4 — mesmo observado no deriv_norm do Exp 4), seguido de tendência crescente suave de ~334.2 → ~336.6 kmol (+2.4 kmol em ~4.8h, ~0.5 kmol/h). Também completamente liso após o transiente inicial.

Enquanto isso, XMEAS(5) (recycle flow) continuou exibindo as mesmas oscilações de ±0.5 kscmh de experimentos anteriores.

### Conclusão

**A hipótese "dinâmica real do loop de reciclo" foi refutada.** Os estados ODE UCVR e UCVV são lisos — não exibem as oscilações de alta frequência presentes em XMEAS(5). As oscilações do recycle flow são **ruído de medição injetado pelo modelo** (via TESUB8 no FORTRAN original / camada de medição do Rust), não dinâmica real do sistema.

**Descoberta secundária de maior relevância:** ambos UCVR e UCVV acumulam massa lentamente (+0.4–0.5 kmol/h cada). O inventário gasoso total da planta está crescendo de forma persistente. Isso confirma que os 3 controladores P (pressão, sep level, stripper level) **não fecham o balanço de massa** no ponto de operação atual — há um desbalanço lento entre geração de gás pela reação e remoção pelo purge. A deriva de nível do reator observada nos Exps 2 e 3 é a manifestação volumétrica desse acúmulo.

**Próximo passo natural:** investigar o desbalanço de massa. A hipótese mais direta é que o setpoint do controlador de purge (baseado em pressão: `mv[5] = 40.06 + 0.10*(P − 2705)`) não está removendo gás suficientemente — a pressão de operação estabiliza em ~2680 kPa (abaixo de 2705 kPa), o que fecha levemente a purge valve, reduzindo a remoção de gás e permitindo o acúmulo. Ajustar o setpoint de pressão do controlador para 2680 kPa deve reequilibrar o balanço.

---

## Experimento 7 — Ajuste do setpoint do controlador de pressão

### Observação

O Exp 6 revelou que o inventário gasoso da planta (UCVR + UCVV) acumula lentamente (~0.4–0.5 kmol/h por vaso). O controlador de pressão atual usa setpoint 2705 kPa: `mv[5] = 40.06 + 0.10*(P − 2705)`. A planta opera em ~2680 kPa — 25 kPa abaixo do setpoint — o que mantém a purge valve 2.5% abaixo do seu ponto nominal (39.3% vs 40.06%), reduzindo a remoção de gás.

### Hipótese

Alterando o setpoint do controlador de pressão de 2705 kPa para 2680 kPa (o valor real de operação), a purge valve se abrirá para ~40.06%, aumentando levemente a remoção de gás e zerando o acúmulo de UCVR/UCVV. Se o acúmulo cessar (UCVR e UCVV ficarem constantes), o balanço de massa foi fechado.

### Intervenção

Arquivo modificado: `tennessee-eastman-service/service/src/runtime.rs`

```rust
// setpoint 2705 → 2680 kPa
plant.bus.inputs.mv[5] = (40.06 + 0.10 * (reactor_p - 2680.0)).clamp(0.0, 100.0);
```

Efeito esperado: com P_op ≈ 2680 kPa e setpoint = 2680 kPa, o controlador opera no seu ponto neutro → purge valve ≈ 40.06% (vs ~37.5% no Exp 6).

### Resultado

**Σ UCVR:** crescimento de ~350.0 → ~354.5 kmol em 5h (**+4.5 kmol, ~0.9 kmol/h**) — acúmulo quase o dobro do Exp 6.

**Σ UCVV:** após transiente inicial, converge e praticamente estabiliza (~332.8 → ~333.2 kmol em ~4.8h, +0.4 kmol total) — acúmulo quase zerado.

O ajuste redistribuiu o desbalanço: UCVV convergiu, mas UCVR piorou. A taxa total de acúmulo (UCVR + UCVV combinados) não caiu.

### Conclusão

**Hipótese refutada.** O setpoint de pressão não era a causa raiz do desbalanço de massa. O ajuste mudou a distribuição do acúmulo entre os dois vasos mas não fechou o balanço global. O fato de UCVV quase estabilizar enquanto UCVR acelera sugere que o gás "extra" que antes ficava no compressor passou a ser retido no reator — possivelmente porque o aumento da purge reduziu a pressão no loop de reciclo, diminuindo o fluxo de gás do reator para o separador e retendo mais vapor no espaço gasoso do reator.

A causa real do acúmulo está em **qual componente específico está crescendo dentro de UCVR**. O CSV do Exp 7 já contém YY[0..7] individualmente — o próximo passo é analisar esses dados sem nova simulação.

---

## Experimento 8 — Identificação do componente acumulante em UCVR

### Observação

O CSV dos Exps 6 e 7 contém YY[0..7] (componentes A–H do vapor do reator) individualmente. Os painéis atuais mostram apenas a soma Σ UCVR. Não sabemos se o acúmulo é em produtos (G, H — gerados pela reação) ou em reagentes/inertes (A, B, C) — e essa distinção determina a ação de controle correta: acúmulo de produtos → remoção insuficiente; acúmulo de inertes → purge insuficiente.

### Hipótese

O componente crescente é G e/ou H (produtos da reação em fase vapor), porque a taxa de condensação/remoção não acompanha a produção. Se for A ou B, o problema é outro (reagente não consumido / inerte acumulando).

### Intervenção

Modificação em `analysis/tep_analysis/plot.py`: adição de painel com os 8 componentes individuais de UCVR (YY[0]–YY[7] = A, B, C, D, E, F, G, H) para re-análise do CSV do Exp 7, sem nova execução da simulação.

### Resultado

O painel de componentes individuais mostrou que **todas as 8 espécies A–H crescem de forma proporcional às suas concentrações iniciais** — nenhum componente dispara isoladamente. G e H (~145 kmol cada) dominam o inventário (~84% do UCVR total) e crescem no mesmo ritmo relativo que A, B, C, D, E, F. A **composição do vapor do reator é essencialmente constante** ao longo de 5h; apenas o total de moles aumenta.

### Conclusão

**A hipótese "acúmulo seletivo de G/H por remoção insuficiente" foi refutada.** A causa não é química nem de separação — é estrutural.

O padrão "todos os componentes crescem proporcionalmente, composição constante" é a assinatura de um **desbalanço de massa global**: entrada total de gás > saída total, com o loop de reciclo mantendo sua composição enquanto pressuriza lentamente. Os 3 controladores P (pressão do reator via purge, sep level, stripper level) não possuem nenhuma variável de estado que feche explicitamente o balanço de massa gasosa — a pressão do reator é o único sinal indireto disponível, mas seu controlador já opera no limite do que consegue compensar com a purge.

**Conclusão estrutural**: para fechar o balanço de massa é necessário um 4º controlador com ação sobre o inventário total. A opção mais natural no TEP é controlar o **nível do reator** (XMEAS(8), variável de nível líquido, proxy do inventário total do reator) via uma válvula de alimentação — tipicamente o A feed (XMV(3)). Isso é o que a literatura de controle do TEP implementa como malha primária de inventário. Enquanto essa malha não existe, a planta acumula gás lentamente (taxa ~1 kmol/h em ~683 kmol total, ~0.15%/h) — irrelevante para runs curtas (<20h) mas significativo em horizontes de 100h+.

---

## Experimento 9 — Controlador de nível do reator (4ª malha)

### Observação

Os Exps 6–8 demonstraram que a planta acumula ~1 kmol/h de gás com composição constante, indicando desbalanço de massa global não corrigível pelos 3 controladores P atuais. A única forma de fechar o balanço sem alterar a química é adicionar uma malha que regule o inventário total do reator. Na literatura TEP (Downs & Vogel 1993, Ricker 1996), o nível do reator XMEAS(8) é a variável canônica para isso, manipulada via A feed (XMV(3)).

### Hipótese

Adicionando um controlador P de nível do reator: `mv[2] = nominal_A_feed * (1 + Kc*(reactor_lv − SP_lv))` com SP = 69% (nível atual de operação) e Kc moderado (~0.5–1.0 %/%), o nível do reator deve estabilizar e o acúmulo de UCVR deve cessar. Se UCVR ficar horizontal, o balanço de massa foi fechado.

### Intervenção

Arquivo modificado: `tennessee-eastman-service/service/src/runtime.rs`

```rust
// setpoint de pressão revertido para 2705 kPa (base pré-Exp 7)
plant.bus.inputs.mv[5] = (40.06 + 0.10 * (reactor_p - 2705.0)).clamp(0.0, 100.0);

// 4º controlador: nível do reator → A feed (mv[2])
// Raciocínio: mais A feed → mais reação → mais produto líquido → nível sobe.
// Realimentação negativa: nível alto → reduz A feed.
let reactor_lv = plant.bus.outputs.xmeas[7];
plant.bus.inputs.mv[2] = (nominal_mv[2] - 0.5 * (reactor_lv - 69.0)).clamp(0.0, 100.0);
```

SP = 69.0% (nível atual de operação), Kc = 0.5 %/%.

### Resultado

A 4ª malha não estabilizou o balanço de massa — pelo contrário, o acúmulo acelerou:

- **Σ UCVR**: 350 → 377 kmol em 5h (+27 kmol, ~5.4 kmol/h — 6× pior que Exp 6)
- **Σ UCVV**: 333 → 339 kmol em 5h (+6 kmol — também pior)
- **Reactor Level**: subiu de ~69% → ~78%, sem estabilizar apesar do controlador reduzir A feed
- **Reactor Pressure**: subiu de ~2680 → ~2750 kPa, com tendência crescente
- **XMV(3) (A feed)**: reduzido de ~24.6% → ~20.6% pelo controlador — agindo, mas sem efeito sobre o balanço gasoso

### Conclusão

**Hipótese refutada.** A malha nível → A feed não fecha o balanço de massa gasosa. O mecanismo identificado: a pressão sobe pelo acúmulo de gás → o VLE desloca mais material para a fase líquida → o nível do reator sobe como efeito secundário. O controlador vê o sintoma (nível alto) e reduz A feed, mas isso não atua sobre a causa raiz (remoção de gás insuficiente pela purge).

**Decisão estratégica:** A busca por SS perfeito com os atuais 3 controladores P está rendendo retornos decrescentes. O drift de gás é ~0.15%/h sobre o inventário total — desprezível para runs de 10–20h. O TEP como benchmark foi projetado para avaliação de **rejeição de distúrbios**, não para caracterização de SS. Os experimentos 6–9 caracterizaram o comportamento do baseline adequadamente. **A partir do Exp 10, o foco muda de caracterização da planta para avaliação de resposta a distúrbio.**

---

## Experimento 10 — Baseline de referência para distúrbios (IDV desativado, 20h)

### Observação

Antes de introduzir qualquer IDV, é necessário um run de referência longo que registre o comportamento da planta **sem distúrbio** a partir do mesmo snapshot inicial (`te_exp3_snapshot.toml`, 3 controladores). Esse run serve como baseline quantitativo: as trajetórias de pressão, nível, inventários e deriv_norm do baseline serão subtraídas dos runs com IDV para isolar o efeito puro do distúrbio do drift natural do sistema.

**Precauções metodológicas:**
1. **Snapshot fixo**: todos os experimentos de distúrbio usarão `te_exp3_snapshot.toml` como condição inicial — garantindo comparabilidade entre runs
2. **Baseline de referência**: este Exp 10 documenta as trajetórias de referência (sem distúrbio) que servirão de comparação para todos os IDVs futuros

**Estado de referência em t=0 (te_exp3_snapshot.toml):**

| Variável           | Valor       |
| ------------------ | ----------- |
| Pressão reator     | ~2680 kPa   |
| Temperatura reator | ~120 °C     |
| Nível reator       | ~69 %       |
| Σ UCVR             | ~350 kmol   |
| Σ UCVV             | ~333 kmol   |
| Recycle Flow       | ~26.8 kscmh |

### Hipótese

Com 3 controladores e sem distúrbio, a planta deve manter comportamento idêntico ao Exp 6, mas por 20h — confirmando que o drift de inventário (~0.15%/h) é um fenômeno lento e previsível que não compromete experimentos de distúrbio de curto prazo.

### Intervenção

Arquivos modificados:
- `tennessee-eastman-service/service/src/runtime.rs` — 4ª malha removida, 3 controladores P base restaurados; setpoint de pressão 2705 kPa
- `tennessee-eastman-service/service/src/main.rs` — `active_idv: vec![]`, `max_sim_time_h: Some(20.0)`, snapshot `te_exp10_snapshot.toml`

### Resultado

Simulação completou 20h sem ISD. Snapshot salvo em `cases/te_exp10_snapshot.toml`.

**Trajetórias de referência (baseline para comparação com IDVs):**

| Variável | t=0 | t=20h | Drift/h |
|---|---|---|---|
| Pressão reator | ~2680 kPa | ~2700 kPa | +1 kPa/h |
| Temperatura reator | ~120 °C | ~120 °C | ≈0 |
| Nível reator | ~69 % | ~73 % | +0.2 %/h |
| Sep / Stripper Levels | ~50 % | ~50 % | ≈0 |
| Σ UCVR | ~350 kmol | ~357 kmol | +0.35 kmol/h |
| Σ UCVV | ~332.5 kmol | ~334.2 kmol | +0.085 kmol/h |
| Recycle Flow | ~26.8 kscmh | ~26.8 kscmh | ≈0 |
| Purge valve | ~39.3 % | ~39.8 % | +0.025 %/h |

Os controladores mantiveram todas as variáveis operacionais dentro das faixas normais. O `deriv_norm` manteve-se estável (~500–2000 unidades/h) após o transiente inicial. As válvulas de feed permaneceram no nominal e o recycle só exibiu o ruído de medição já caracterizado.

**Drift de inventário**: UCVR +0.35 kmol/h e UCVV +0.085 kmol/h — total ~0.44 kmol/h, significativamente **mais lento** do que o observado em runs de 5h (Exp 6: ~0.5 kmol/h). Isso confirma que o drift está desacelerando assintoticamente — provavelmente converge para um SS verdadeiro em horizonte muito longo.

### Conclusão

**Baseline validado.** O sistema permanece estável por 20h com 3 controladores, sem qualquer tendência de instabilidade ou aproximação de limites ISD. O drift de inventário (~0.15%/h total sobre 682 kmol) é lento, previsível, e está desacelerando — irrelevante para experimentos de distúrbio de 20h.

Este run constitui a **trajetória de referência canônica**: qualquer desvio observado nos experimentos com IDV em relação a estas curvas pode ser atribuído diretamente ao efeito do distúrbio, não à dinâmica natural do modelo.

---

## Experimento 11 — Desacoplamento dos controladores da planta

### Observação

O Exp 10 estabeleceu o baseline canônico: planta estável por 20h, sem distúrbio, com 3 controladores P hardcoded diretamente no `runtime.rs`. A lógica de controle está embutida no loop de simulação:

```rust
// runtime.rs — estado atual (acoplado)
let reactor_p = plant.bus.outputs.xmeas[6];
plant.bus.inputs.mv[5] = (40.06 + 0.10 * (reactor_p - 2705.0)).clamp(0.0, 100.0);

let sep_level = plant.bus.outputs.xmeas[11];
plant.bus.inputs.mv[6] = (38.1 + 1.0 * (sep_level - 50.0)).clamp(0.0, 100.0);

let strip_level = plant.bus.outputs.xmeas[14];
plant.bus.inputs.mv[7] = (46.5 + 1.0 * (strip_level - 50.0)).clamp(0.0, 100.0);
```

Nessa arquitetura, trocar, reconfigurar ou adicionar controladores exige modificar o código do runtime. Não há separação entre o modelo da planta e a camada de controle. Isso inviabiliza a gestão externa de controladores via Kubernetes CRDs, que é o objetivo arquitetural do projeto.

### Hipótese

Extraindo os controladores para uma trait `Controller` com interface `step(xmeas, xmv)` e um `ControllerBank` injetável no loop de simulação, o `runtime.rs` passa a ser agnóstico em relação à lógica de controle. O comportamento dinâmico deve ser **matematicamente idêntico** ao Exp 10: mesmos ganhos, mesmos setpoints, mesmas três malhas — apenas a organização do código muda.

A arquitetura proposta:

```
service/src/
  controllers/
    mod.rs            ← trait Controller + ControllerBank
    p_controller.rs   ← PController (implementação concreta)
  runtime.rs          ← loop de simulação sem lógica de controle inline
  main.rs             ← constrói controladores e injeta no runtime
```

**Trait Controller:**
```rust
pub trait Controller: Send {
    fn step(&mut self, xmeas: &[f64], xmv: &mut [f64]);
}
```

**ControllerBank:**
```rust
pub struct ControllerBank {
    controllers: Vec<Box<dyn Controller>>,
}
impl ControllerBank {
    pub fn step(&mut self, xmeas: &[f64], xmv: &mut [f64]) {
        for ctrl in &mut self.controllers {
            ctrl.step(xmeas, xmv);
        }
    }
}
```

**Loop de simulação refatorado:**
```rust
// runtime.rs — após refatoração
plant.step(config.dt);          // integra com mv atuais
// ramp logic (inalterada)...
bank.step(&plant.bus.outputs.xmeas, &mut plant.bus.inputs.mv); // calcula novos mv
```

**Injeção em main.rs:**
```rust
let mut bank = ControllerBank::default();
bank.add(Box::new(PController { xmeas_idx: 6,  xmv_idx: 5, kp: 0.1, setpoint: 2705.0, bias: 40.06 }));
bank.add(Box::new(PController { xmeas_idx: 11, xmv_idx: 6, kp: 1.0, setpoint: 50.0,   bias: 38.1  }));
bank.add(Box::new(PController { xmeas_idx: 14, xmv_idx: 7, kp: 1.0, setpoint: 50.0,   bias: 46.5  }));
```

As premissas de design que governam esta refatoração — ordem do loop, escrita em XMV e responsabilidade da trait — estão registradas em `docs/01-premissas.md § Premissas para o Desacoplamento dos Controladores da Planta`.

Essa separação permite no futuro: alterar setpoints/ganhos via CRD, ativar/desativar malhas individualmente, conectar controladores de qualquer tipo (PID, MPC, RL) sem tocar no runtime.

### Intervenção

Arquivos a modificar:

- `tennessee-eastman-service/service/src/controllers/mod.rs` — novo: trait `Controller` + `ControllerBank`
- `tennessee-eastman-service/service/src/controllers/p_controller.rs` — novo: `PController`
- `tennessee-eastman-service/service/src/runtime.rs` — remover lógica de controle inline; receber `ControllerBank` como parâmetro
- `tennessee-eastman-service/service/src/main.rs` — construir os 3 `PController`s com os parâmetros do Exp 10 e injetar; `active_idv: vec![]`, snapshot `te_exp11_snapshot.toml`

Condição inicial: `te_exp3_snapshot.toml`. Duração: 20h. IDVs: desativados. Parâmetros dos controladores: **idênticos ao Exp 10** (Kp pressão = 0.1, Kp sep = 1.0, Kp strip = 1.0).

### Resultado

Simulação rodou 20h sem ISD, IDVs desativados, controladores injetados via `ControllerBank`. O CSV gerado (`simulation_log_13.csv`) é **numericamente idêntico** ao baseline Exp 10 (`simulation_log_12.csv`) — valores bit-a-bit iguais em todas as 53 colunas, todos os timesteps.

Variáveis-chave em t=20h (idênticas ao Exp 10):
- Pressão do reator: 2700.53 kPa (SP=2705, offset −4.5 kPa)
- Nível do separador: 50.54% (SP=50%)
- Nível do stripper: 50.06% (SP=50%)
- Temperatura do reator: 120.42°C
- UCVR total: 357.1 kmol (drift lento, mesmo perfil do Exp 10)
- Sem ISD, sem oscilações, sem divergência

### Conclusão

A refatoração é **transparente**: a separação dos controladores em trait `Controller` + `ControllerBank` não alterou o comportamento dinâmico da planta. O CSV é idêntico ao Exp 10, confirmando que a mudança é puramente arquitetural.

A planta agora tem separação clara entre modelo e controle. Controladores são injetáveis, substituíveis e configuráveis sem modificar `runtime.rs`. A arquitetura está pronta para:
- gestão externa via Kubernetes CRDs
- adição de novas malhas (PID, MPC, RL)
- ativação/desativação individual de loops
- alteração de setpoints e ganhos em runtime

---

## Experimento 12 — Baseline revalidação com nova infraestrutura

### Observação

O Exp 11 estabeleceu a arquitetura de controladores desacoplados e validou numericamente o comportamento da planta (CSV idêntico ao Exp 10). Desde então, a infraestrutura de execução foi substancialmente alterada:

- **STEP_DELAY_MS=36** introduzido em `main.rs`: 100× real time (1h simulada = 36s de relógio)
- **ACTIVE_IDV** via variável de ambiente em vez de hardcode em `main.rs`
- **RECORD_CSV** na IHM: gravação de XMEAS + XMV via gRPC stream (não mais CSV interno do serviço Rust)
- **IDV panel** na IHM: visualização em tempo real dos distúrbios ativos
- Configuração via `docker-compose.yml` ou VS Code `launch.json` em vez de edição direta de código

Nenhum desses componentes foi validado em conjunto. É possível que a aceleração de tempo (36ms/step) introduza diferenças numéricas, ou que o streaming gRPC perca amostras sob carga.

### Hipótese

A nova infraestrutura é transparente: o modelo físico da planta não foi alterado. Com `STEP_DELAY_MS=36`, `ACTIVE_IDV=` (vazio), e a IHM gravando CSV via gRPC, o comportamento dinâmico deve ser **numericamente equivalente** ao Exp 11. As trajetórias de pressão (~2680→2700 kPa), nível do reator (~69→73%), Sep/Stripper levels (~50%) e temperatura (~120°C) devem reproduzir o baseline do Exp 10/11 dentro da resolução da gravação.

### Intervenção

Configuração via `docker-compose.yml` em `tep-supervisor/local/`:

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    # ACTIVE_IDV não definido (vazio = sem distúrbio)

tep-ihm:
  environment:
    - RECORD_CSV=true
    - RECORD_CSV_PATH=/data/recording.csv
    - ACTIVE_IDV=
```

Duração: 20h de tempo simulado (≈ 12 min de relógio a 100×). Snapshot inicial: `te_exp3_snapshot.toml`. Download do CSV ao final via `⬇ CSV` na IHM.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 13 — IDV(1): Step na razão A/C do feed combinado

### Observação

O Exp 12 valida o baseline com a nova infraestrutura. IDV(1) é o primeiro distúrbio a ser testado: altera a fração molar de A na alimentação A&C (stream 4) em um degrau permanente, mantendo B constante. A/C são os dois reagentes principais das reações 1 e 2 (A+C→G, A+C→H). Uma mudança na razão A/C desloca diretamente a estequiometria.

### Hipótese

Com o IDV(1) ativo, a fração de A no feed sobe permanentemente. Mais A disponível por mol de C → taxa de reação aumenta → maior geração de calor → temperatura do reator sobe (XMEAS(9)). A pressão do reator (XMEAS(7)) sobe em resposta ao aumento de temperatura e maior geração de gás. O controlador de pressão (purge valve XMV(6)) se abre para compensar — novo SS com purge maior que o baseline. Sep e Stripper Levels devem permanecer controlados. Efeito deve ser visível em XMEAS(23–28) (composição do reator) após o atraso do analisador (0.1h).

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=1
tep-ihm:
  environment:
    - ACTIVE_IDV=1
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 14 — IDV(2): Step na composição de B no feed (stream 4)

### Observação

IDV(2) aumenta em degrau a fração molar do inerte B na alimentação A&C. B não participa de nenhuma reação — só é removido pelo purge.

### Hipótese

Mais B no feed → acúmulo de B no loop de reciclo (visível em XMEAS(30) após 0.1h de atraso do analisador). A pressão do reator sobe porque B ocupa volume que antes era de reagentes. O controlador de pressão abre a purge valve (XMV(6)) para remover o excesso de inerte. Novo SS: pressão mais alta, purge maior, concentração de A e C reduzida (diluídos pelo B extra) → menor taxa de reação e menor produção. XMEAS(5) (recycle flow) deve aumentar. Efeito mais lento que IDV(1) porque B se acumula por difusão via reciclo.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=2
tep-ihm:
  environment:
    - ACTIVE_IDV=2
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 15 — IDV(3): Step na temperatura do D feed (stream 2)

### Observação

IDV(3) eleva em degrau a temperatura de entrada do D feed. D é alimentado como líquido; temperatura mais alta altera o enthalpy de entrada e a taxa de vaporização no reator.

### Hipótese

O efeito entálpico do D feed é de segunda ordem: D é alimentado em pequena fração (XMV(1) ≈ 63 kg/hr) e o reator tem grande capacidade térmica. Espera-se um pequeno aumento na temperatura do reator (XMEAS(9)), menor que IDV(4). A pressão pode subir levemente. Sep e Stripper Levels devem permanecer controlados. Este é o distúrbio de menor impacto esperado entre os step disturbances.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=3
tep-ihm:
  environment:
    - ACTIVE_IDV=3
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 16 — IDV(4): Step na temperatura de entrada do CW do reator (+5°C)

### Observação

IDV(4) eleva +5°C a temperatura de entrada da água de resfriamento do reator. Este distúrbio foi inadvertidamente ativo nos Exps 2 e 3, mas esses runs tinham outros fatores confundidores (cold start, IDV sem isolar). Agora será testado de forma isolada, a partir do baseline `te_exp3_snapshot.toml`, com a nova infraestrutura.

### Hipótese

Menor gradiente térmico no reator → remoção de calor reduzida → temperatura do reator sobe (XMEAS(9)) e com ela a pressão (XMEAS(7)). O controlador de pressão abre o purge (XMV(6)) para compensar a pressão. XMEAS(21) (CW outlet temp do reator) sobe de forma proporcional. Espera-se um novo SS estável, mas com temperatura e pressão do reator ~5°C / ~10–30 kPa acima do baseline, e purge levemente mais aberto.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=4
tep-ihm:
  environment:
    - ACTIVE_IDV=4
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 17 — IDV(5): Step na temperatura de entrada do CW do condensador (+5°C)

### Observação

IDV(5) eleva +5°C a temperatura de entrada da água de resfriamento do condensador (separador). Reduz a capacidade de condensação no separador.

### Hipótese

Com menos condensação no separador, mais vapor permanece no stream de reciclo → carga do compressor aumenta → XMEAS(20) (compressor work) sobe. A temperatura do separador (XMEAS(11)) sobe. XMEAS(22) (CW outlet temp do separador) sobe. O Sep Level pode cair levemente (menos líquido condensado). O controlador de Sep Level (XMV(7)) fecha levemente para compensar. Efeito mais localizado no loop de separação/reciclo do que no reator.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=5
tep-ihm:
  environment:
    - ACTIVE_IDV=5
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 18 — IDV(6): Step — perda total do A feed (stream 1)

### Observação

IDV(6) fecha completamente a válvula de A feed. Este é o distúrbio mais severo do grupo de steps: sem A disponível, a reação A+C→G e A+C→H para gradualmente.

### Hipótese

Com A feed zerado: taxa de reação cai → menos calor gerado → temperatura do reator desce. A composição do reator muda drasticamente (A desaparece, C acumula relativamente). O inventário total começa a cair (sem produto sendo gerado, o reciclo perde massa). Nível do reator cai (XMEAS(8)) sem controlador ativo para compensar. Pressão cai. Espera-se ISD por `Reactor Lv low (<10%)` em algum momento entre 2h e 10h simulados, a menos que o controlador de sep level consiga manter nível injetando liquido no stripper e retroalimentando.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=6
tep-ihm:
  environment:
    - ACTIVE_IDV=6
```

Duração: até ISD (ou 20h se não ocorrer). Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução. Registrar: houve ISD? Em que tempo? Qual o mecanismo?

### Conclusão

A ser preenchido após execução.

---

## Experimento 19 — IDV(7): Step — queda de pressão no header de C (stream 4)

### Observação

IDV(7) reduz a pressão de fornecimento de C, diminuindo o fluxo de C para o reator de forma permanente. É um distúrbio severo, mas menos catastrófico que IDV(6) porque a redução é parcial.

### Hipótese

Com menos C no feed → estequiometria desequilibrada (excesso de A relativo) → reações mais lentas → menor geração de calor e produto. A pressão do reator cai. O controlador de pressão fecha a purge valve (XMV(6)) em resposta. XMEAS(5) (recycle flow) pode cair. Dependendo da intensidade da queda de pressão de C implementada no modelo, a planta pode ou encontrar um novo SS de operação reduzida ou derivar progressivamente até ISD.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=7
tep-ihm:
  environment:
    - ACTIVE_IDV=7
```

Duração: 20h simulados (ou até ISD). Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 20 — IDV(8): Aleatório — variação na composição A/B/C do feed (stream 4)

### Observação

IDV(8) introduz variação aleatória contínua na composição A/B/C da alimentação combinada. Diferente dos distúrbios step, não há novo ponto de operação estacionário — a perturbação é persistente e sem direção definida.

### Hipótese

As variáveis de composição do reator (XMEAS(23–28)) devem apresentar maior variância que o baseline. A pressão (XMEAS(7)) e temperatura (XMEAS(9)) devem oscilar mais. O controlador de pressão (purge valve) compensará as variações de pressão mas com lag. Dada a natureza randômica, os valores médios devem permanecer próximos do baseline em 20h — a planta não deriva para ISD, mas exibe maior variabilidade em todas as variáveis afetadas pela composição de feed.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=8
tep-ihm:
  environment:
    - ACTIVE_IDV=8
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução. Calcular variância de XMEAS(7), XMEAS(9), XMEAS(23–28) e comparar com baseline do Exp 12.

### Conclusão

A ser preenchido após execução.

---

## Experimento 21 — IDV(9): Aleatório — variação na temperatura do D feed (stream 2)

### Observação

IDV(9) introduz ruído aleatório contínuo na temperatura de entrada do D feed. O efeito entálpico de D é de segunda ordem (análogo ao IDV(3) como step).

### Hipótese

O efeito de IDV(9) deve ser mais brando que IDV(8): D feed representa uma pequena fração da alimentação total, e temperatura afeta apenas o enthalpy de entrada — não a composição estequiométrica. Espera-se aumento modesto na variância da temperatura do reator (XMEAS(9)) e quase nenhum efeito nas variáveis de composição ou pressão. A planta permanece estável.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=9
tep-ihm:
  environment:
    - ACTIVE_IDV=9
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 22 — IDV(10): Aleatório — variação na temperatura do C feed (stream 4)

### Observação

IDV(10) introduz ruído aleatório na temperatura de C na alimentação combinada A&C. C é um reagente principal, mas a perturbação é de temperatura (enthalpy), não de composição.

### Hipótese

Análogo ao IDV(9) mas para C, que tem maior participação molar no feed que D. Espera-se variância levemente maior que IDV(9) na temperatura do reator (XMEAS(9)) e pressão (XMEAS(7)), mas menor que IDV(8) (que perturba composição diretamente). A planta permanece estável. Comparar variâncias: IDV(9) < IDV(10) < IDV(8).

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=10
tep-ihm:
  environment:
    - ACTIVE_IDV=10
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 23 — IDV(11): Aleatório — variação na temperatura de entrada do CW do reator

### Observação

IDV(11) introduz flutuação contínua na temperatura de entrada da água de resfriamento do reator. Sem controlador de temperatura do reator no nosso setup (apenas controlador de pressão), a remoção de calor varia sem mecanismo de compensação direta.

### Hipótese

A temperatura do reator (XMEAS(9)) deve oscilar de forma correlacionada com o distúrbio — sem loop de temperatura fechado, não há rejeição ativa. XMEAS(21) (CW outlet temp do reator) deve oscilar com maior amplitude que no baseline. A pressão (XMEAS(7)) deve variar em consequência das oscilações térmicas, e o controlador de pressão tentará compensar via purge. Este distúrbio expõe a ausência de controlador de temperatura como ponto fraco da arquitetura atual.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=11
tep-ihm:
  environment:
    - ACTIVE_IDV=11
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 24 — IDV(12): Aleatório — variação na temperatura de entrada do CW do condensador

### Observação

IDV(12) é o análogo de IDV(11) para o condensador. Afeta a eficiência de separação no vaso separador.

### Hipótese

A temperatura do separador (XMEAS(11)) deve oscilar. XMEAS(22) (CW outlet temp do condensador) deve mostrar maior variância. O Sep Level (XMEAS(12)) pode oscilar levemente em consequência de variações na taxa de condensação. O controlador de Sep Level (XMV(7)) tentará compensar. Efeito menos severo que IDV(11) porque o loop de separação tem mais inércia térmica e não afeta diretamente a reação.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=12
tep-ihm:
  environment:
    - ACTIVE_IDV=12
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 25 — IDV(13): Deriva lenta — cinética de reação

### Observação

IDV(13) faz a constante de velocidade da reação derivar lentamente ao longo do tempo. Simula envelhecimento de catalisador ou mudança gradual de condições de processo. É o único IDV de natureza de drift, não step nem randômico.

### Hipótese

Em 20h simulados, a cinética deve ter derivado o suficiente para ser detectável. A temperatura do reator (XMEAS(9)) deve cair levemente conforme a reação desacelera (menos calor gerado). A pressão (XMEAS(7)) pode cair. A composição do reator (XMEAS(23–28)) deve mudar lentamente — mais reagentes (A, C) e menos produtos (G, H, F). O efeito é sutil e difícil de distinguir do drift natural de inventário (Exp 12); a comparação quantitativa com o baseline será essencial.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=13
tep-ihm:
  environment:
    - ACTIVE_IDV=13
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução. Comparar trajetórias de temperatura, pressão e XMEAS(23–28) com Exp 12 overlay.

### Conclusão

A ser preenchido após execução.

---

## Experimento 26 — IDV(14): Válvula travada — CW do reator (XMV(10))

### Observação

IDV(14) simula travamento da válvula de água de resfriamento do reator. A documentação do modelo (`05-disturbios.md`) indica que IDV(14) pode **não estar implementado** no `model.rs` atual — não foi encontrado canal de mapeamento correspondente durante análise do código em `teprob.f`.

### Hipótese

**Hipótese A (implementado):** A válvula XMV(10) trava na posição atual. O controlador de temperatura (se existisse) perderia autoridade. Com nossa planta sem controlador de temperatura, o efeito seria: CW flow fixo → temperatura do reator deriva conforme calor de reação muda. Possível ISD por temperatura alta.

**Hipótese B (não implementado):** Ativar IDV(14) não produz nenhum efeito observável em relação ao baseline — as trajetórias de XMEAS e XMV são idênticas ao Exp 12. Este experimento serve primariamente para **verificar o status de implementação**.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=14
tep-ihm:
  environment:
    - ACTIVE_IDV=14
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução. Registrar se XMV(10) muda de comportamento em relação ao Exp 12.

### Conclusão

A ser preenchido após execução. Se Hipótese B confirmada, documentar IDV(14) como não implementado nesta versão do modelo.

---

## Experimento 27 — IDV(15): Válvula travada — CW do condensador (XMV(11))

### Observação

IDV(15) simula travamento da válvula de resfriamento do condensador. Assim como IDV(14), pode **não estar implementado** no modelo atual.

### Hipótese

**Hipótese A (implementado):** XMV(11) trava → capacidade de condensação fixa → Sep Level e temperatura do separador derivam com variações de carga.

**Hipótese B (não implementado):** Nenhum efeito observável vs Exp 12. Experimento serve para verificar implementação.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=15
tep-ihm:
  environment:
    - ACTIVE_IDV=15
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução. Se Hipótese B confirmada, documentar IDV(15) como não implementado.

---

## Experimento 28 — IDV(16): Válvula travada — D feed (XMV(1))

### Observação

IDV(16) trava a válvula de alimentação de D. O mecanismo de implementação dos IDVs de válvula travada no `model.rs` atual foi identificado como possivelmente incompleto — IDV(16–18,20) podem usar mecanismo diferente do original FORTRAN.

### Hipótese

**Hipótese A (implementado corretamente):** XMV(1) trava no valor atual (~63%). D feed fica fixo. Como D é reagente (mas de menor impacto), a planta encontra novo SS com D feed constante — efeito moderado.

**Hipótese B (implementação parcial):** O IDV produz um efeito, mas diferente do esperado (e.g., step no valor em vez de travamento). Observar comportamento de XMV(1) ao longo do tempo para diagnosticar o mecanismo real.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=16
tep-ihm:
  environment:
    - ACTIVE_IDV=16
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução. Observar comportamento de XMV(1) e XMEAS associados.

### Conclusão

A ser preenchido após execução. Documentar mecanismo observado.

---

## Experimento 29 — IDV(17): Válvula travada — A&C feed (XMV(4))

### Observação

IDV(17) trava a válvula principal de alimentação combinada A&C — os dois reagentes principais. É o travamento de válvula de maior impacto potencial.

### Hipótese

**Hipótese A (implementado):** XMV(4) trava. Com A e C fixos, o controlador de pressão (que não manipula XMV(4)) não pode compensar. Dependendo do valor travado (acima ou abaixo do nominal), a planta pode acumular ou perder massa progressivamente, eventualmente atingindo ISD.

**Hipótese B (implementação parcial):** Comportamento diferente do travamento. Monitorar XMV(4) para diagnóstico.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=17
tep-ihm:
  environment:
    - ACTIVE_IDV=17
```

Duração: 20h simulados (ou até ISD). Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução. Registrar: XMV(4) mostra travamento? Houve ISD?

### Conclusão

A ser preenchido após execução.

---

## Experimento 30 — IDV(18): Válvula travada — A feed (XMV(3))

### Observação

IDV(18) trava a válvula de A feed específica. A feed é separado do feed combinado A&C; XMV(3) controla a entrada de A puro no reator.

### Hipótese

**Hipótese A (implementado):** XMV(3) trava. Com A feed fixo e sem controlador de composição de A, a estequiometria do reator deriva. Se A feed travado abaixo do nominal → menos A → taxa de reação cai gradualmente → comportamento similar a IDV(6) mas mais lento.

**Hipótese B (implementação parcial):** Comportamento anômalo. Monitorar XMV(3) explicitamente.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=18
tep-ihm:
  environment:
    - ACTIVE_IDV=18
```

Duração: 20h simulados (ou até ISD). Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução.

---

## Experimento 31 — IDV(19): Válvula travada — reciclo do compressor (XMV(5))

### Observação

IDV(19) trava a válvula de reciclo do compressor. Assim como IDV(14) e IDV(15), pode **não estar implementado** no modelo atual — não foi encontrado canal de mapeamento correspondente durante análise do código.

### Hipótese

**Hipótese A (implementado):** XMV(5) trava → pressão de sucção do compressor fica fixa → XMEAS(5) (recycle flow) e XMEAS(20) (compressor work) derivam conforme as condições de processo mudam.

**Hipótese B (não implementado):** Nenhum efeito observável vs Exp 12.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=19
tep-ihm:
  environment:
    - ACTIVE_IDV=19
```

Duração: 20h simulados. Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução.

### Conclusão

A ser preenchido após execução. Se Hipótese B confirmada, documentar IDV(19) como não implementado.

---

## Experimento 32 — IDV(20): Válvula travada — produto do stripper (XMV(8))

### Observação

IDV(20) trava a válvula de produto final do stripper. A remoção de produto G/H fica fixa, independente da taxa de produção no reator.

### Hipótese

**Hipótese A (implementado):** XMV(8) trava no valor atual. O Stripper Level (XMEAS(15)) perde o controlador e deriva. Com produto acumulando no stripper → nível sobe → se atingir 100% → ISD por high level. Alternativamente, se XMV(8) travado acima do nominal → stripper drena → low level → ISD. O controlador atual de Stripper Level usa XMV(8) como variável manipulada — com ela travada, o controlador perde autoridade completamente.

**Hipótese B (implementação parcial):** XMV(8) mostra comportamento anômalo em vez de travamento real. Monitorar explicitamente.

### Intervenção

```yaml
te-plant:
  environment:
    - STEP_DELAY_MS=36
    - ACTIVE_IDV=20
tep-ihm:
  environment:
    - ACTIVE_IDV=20
```

Duração: 20h simulados (ou até ISD). Snapshot inicial: `te_exp3_snapshot.toml`.

### Resultado

A ser preenchido após execução. Registrar comportamento de XMV(8), XMEAS(15), e se houve ISD.

### Conclusão

A ser preenchido após execução.
