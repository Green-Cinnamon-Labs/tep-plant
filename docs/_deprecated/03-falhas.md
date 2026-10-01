# Relatório de Falhas da Simulação

---

## [8] Simulação encerra imediatamente ao iniciar em terminal novo

**Status:** RESOLVIDO

### Contexto
Toda vez que o programa era iniciado num terminal recém-aberto (VS Code integrated terminal,
Git Bash ou similar), a simulação encerrava em frações de segundo sem qualquer mensagem de erro
visível. Na segunda execução no **mesmo** terminal, o programa rodava normalmente pelo tempo
desejado. O comportamento era 100% reproduzível: novo terminal → morte rápida; mesmo terminal,
segundo run → funcionamento normal.

### Diagnóstico
O culpado é a linha em `dashboard.rs`:
```rust
if event::poll(std::time::Duration::from_millis(0))? {
    if let Event::Key(key) = event::read()? { ... }
}
```
`poll(0ms)` verifica o buffer de stdin **sem esperar**. Ao abrir um terminal novo, o shell ou
a IDE envia sequências de inicialização (focus events, resize notifications, cursor position
reports) que ficam no buffer de stdin. Quando `enable_raw_mode()` é chamado, essas sequências
já estão disponíveis; `poll(0)` retorna `true` imediatamente e `event::read()` consome um
desses bytes. Se o crossterm os interpreta como `KeyCode::Char('q')` ou `Ctrl+C`, o loop de
simulação é encerrado antes do primeiro step. Na segunda execução, o buffer está vazio e o
problema não ocorre.

### Solução
Drenar o buffer de eventos logo após `enable_raw_mode()` em `Dashboard::new()`:
```rust
while event::poll(std::time::Duration::from_millis(0))? {
    let _ = event::read()?;
}
```
Isso descarta todos os eventos pendentes da inicialização do terminal antes de entrar no loop
de simulação.

Histórico técnico das falhas de simulação encontradas durante o desenvolvimento do sistema ciber-físico (CPS) do Tennessee Eastman Process em Rust. Ordenadas da mais recente para a mais antiga.

---

## [7] Cold start substitui settle como estratégia de startup

**Status:** IMPLEMENTADO

### Contexto
A abordagem de settle (issue [5b]) não convergiu porque o estado FORTRAN tem `ptv/pts ≈ 2.2`, acima de `CPPRMX = 1.3`, causando ISD durante o settling. Qualquer tentativa de iniciar a planta diretamente no ponto de operação nominal resultava em acúmulo de não-condensáveis e disparo de shutdown por pressão alta.

### Solução
Cold start: manter o estado FORTRAN (holdups não-zero em todos os vasos, evitando divisão por zero) mas rampar as válvulas de feed (`mv[0..4]`) de 0% a nominal linearmente em `ramp_duration` horas de tempo simulado. Controladores P de pressão (`mv[5]`), nível do separador (`mv[6]`) e nível do stripper (`mv[7]`) ativos desde `t=0`. Distúrbios IDV desligados durante a rampa e ativados automaticamente ao final. Configurável via `Config::ramp_duration`.

---

## [6] Variáveis `fwr` e `fws` declaradas sem uso após remoção das ODEs de CW

**Status:** RESOLVIDO

### Contexto
Ao remover as ODEs dinâmicas de TWR/TWS (falha [2]), as variáveis `fwr` (vazão de água de resfriamento do reator) e `fws` (separador) continuaram declaradas no código. Isso gerou avisos de compilador `unused variable`.

### Diagnóstico
Essas variáveis eram calculadas exclusivamente para alimentar as ODEs removidas. Sem as ODEs, não há uso legítimo delas no código atual. Foram removidas junto com as ODEs.

---

## [5b] Estratégia de resolução: modo de assentamento dinâmico (settle)

**Status:** SUBSTITUÍDO por cold start — see [7]

### Contexto
Duas abordagens foram avaliadas para resolver [5]: assentamento dinâmico (integrar com controladores mínimos até convergência) e solver direto de steady-state (Newton/Broyden sobre f(x) = 0). O problema requer uma decisão de design antes de qualquer implementação.

### Diagnóstico
O solver direto foi descartado porque `DynamicModel::derivatives()` tem assinatura `&mut self` — o modelo muta estado interno cacheado (pressões parciais, densidades, frações molares) durante cada avaliação. Construir um Jacobiano por diferenças finitas exigiria snapshot/restore completo do modelo a cada coluna, que a trait atual não suporta. Adicionalmente, o TEP tem descontinuidades (curva do compressor em `PR = CPPRMX`, flash VLE implícito) que degradam a convergência de métodos Newton. A abordagem de settle foi implementada em `Plant::settle()` mas não convergiu na prática devido ao problema [4] — o ISD disparava antes da convergência. Substituída pela estratégia de cold start [7].

---

## [5] Estado inicial do TOML não é steady-state do modelo Rust

**Status:** RESOLVIDO (via cold start — see [7])

### Contexto
O arquivo `te_mode1_initial_state.toml` foi construído a partir dos valores de `TEINIT` do FORTRAN original. Após cada nova correção de modelagem (energia, CW, etc.), um novo sintoma de instabilidade surge imediatamente a partir de `t=0`, indicando que o estado inicial não satisfaz `ẋ = 0` para o modelo Rust.

### Diagnóstico
O modelo Rust difere do FORTRAN em pelo menos dois aspectos estruturais: (a) dinâmica de válvulas explícita no vetor de estados em vez de posição de válvula diretamente; (b) possíveis diferenças nas fórmulas de entalpia e densidade que produzem pressões e temperaturas de equilíbrio ligeiramente diferentes. A estratégia de cold start [7] contorna o problema: em vez de exigir um estado inicial que satisfaça `ẋ = 0`, aceita o estado FORTRAN como condição inicial válida e rampa os feeds gradualmente, permitindo que a planta encontre seu próprio ponto de operação.

---

## [4] Pressão do reator sobe de 2705 → >3000 kPa em segundos (acúmulo de não-condensáveis)

**Status:** CONTORNADO (via cold start + P-controller permanente — see [7])

### Contexto
Após resolver as falhas [2] e [3], a simulação passou a rodar por ~7 segundos antes de disparar ISD por pressão alta. O componente A (não-condensável) triplicou de `10.4` para `29.9 lbmol` nesse intervalo, e a pressão do reator escalou de `2705` para `6613 kPa`.

### Diagnóstico
A curva de desempenho do compressor usa `CPPRMX = 1.3` como razão de pressão máxima. Na condição nominal do estado inicial, a razão `ptv/pts ≈ 2.2` está muito acima desse limite, produzindo fluxo de reciclo efetivamente zero pela fórmula `flms = CPFLMX + CPFLMX/1.197*(1 - PR³)`. Os não-condensáveis A, B, C não conseguem circular do separador de volta ao reator e se acumulam indefinidamente. O controlador P de pressão no purge valve (`mv[5]`) é agora permanente (não mais um patch temporário) e faz parte da estratégia de cold start [7].

---

## [3] Constantes HWR e HWS inicializadas mas nunca usadas na dinâmica

**Status:** RESOLVIDO (por omissão — ODEs removidas)

### Contexto
Durante a investigação da falha [2], verificou-se que `HWR = 7060.` e `HWS = 11138.` estão presentes no bloco `COMMON /TEPROC/` e são inicializados em `TEINIT`, mas jamais são referenciados em `TEFUNC`. O Rust havia tentado usá-los para construir uma ODE de dinâmica térmica para o circuito de resfriamento, mas sem base no comportamento do FORTRAN de referência.

### Diagnóstico
A adição das ODEs de TWR/TWS foi um pressuposto de modelagem incorreto: o modelo de referência Downs-Vogel trata essas temperaturas como parâmetros fixos durante a integração. A variável existe no vetor de estado apenas para compatibilidade estrutural com TEINIT, não para ser integrada. Qualquer tentativa de atribuir dinâmica a `yp[36]`/`yp[37]` não tem correspondência no FORTRAN.

---

## [2] Temperatura do reator colapsa para −752 °C após correção das energias

**Status:** RESOLVIDO

### Contexto
Após corrigir ETS/ETC/ETV, o reator passou a apresentar `TCR = −752 °C`, `Reactor P = 1.53×10⁹ kPa` e ETR colapsando para `−29.192` nos primeiros passos. A simulação disparava ISD imediatamente.

### Diagnóstico
O modelo Rust implementou as temperaturas de saída da água de resfriamento (TWR, TWS) como ODEs dinâmicas com denominador de massa térmica igual a `1.0`. O FORTRAN `TEFUNC` nunca atribui `YP(37)` ou `YP(38)` — TWR e TWS são lidos do vetor de estado mas permanecem constantes durante toda a simulação. Com denominador `1.0` em vez do valor correto `HWR = 7060` (cal/°C), a água de resfriamento do reator colapsava de `94.6 °C` para `~35 °C` em segundos, triplicando a remoção de calor `QUR` e drenando ETR. A correção foi: `yp[36] = 0.0; yp[37] = 0.0;`.

---

## [1] Escala incorreta nos estados de energia interna (ETS, ETC, ETV)

**Status:** RESOLVIDO

### Contexto
Ao inicializar o modelo com o estado do TOML derivado do FORTRAN TEINIT, as temperaturas do separador, stripper e compressor colapsavam imediatamente. Os valores de energia interna ETS, ETC e ETV produziam temperaturas fisicamente impossíveis já no primeiro passo da simulação.

### Diagnóstico
Os coeficientes de entalpia líquida `AH` no modelo Rust estão na escala `~1e-6` (ex.: `AH[3] = 0.960e-6`), enquanto os valores de energia copiados diretamente do FORTRAN TEINIT assumem a escala original do FORTRAN. A conversão exigiu dividir ETS, ETC e ETV por 1000. ETR estava correto porque o reator opera predominantemente em fase vapor, cujos coeficientes `AG` têm escala diferente. Os valores corrigidos foram: `ETS = 0.5699317760`, `ETC = 0.3755297257`, `ETV = 0.9218489762`.
