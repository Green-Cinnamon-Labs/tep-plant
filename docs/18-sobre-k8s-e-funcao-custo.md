# Sobre o k8s e a função de custo

Pelo visto, Downs & Vogel já trazem uma função de custo (ou função objetivo) na tabela 9 [*1]. Além disso, existe uma tabela de modos que não me parecem ter a ver exatamente com essa função objetivo — no sentido de ter sido derivado dela; é lógico que o modo afeta a função de custo [*2]. O que é, fundamentalmente, a minha tese. Pegar essa planta digital [*3] (que fala OPC-UA com o mundo externo) e acompanhar a função de custo que eles, Downs & Vogel, propuseram. Simples assim [*4].

- **[*1]** Sim. É um custo operacional em $/h, soma de: matéria-prima perdida na purga, matéria-prima perdida no produto, formação de F, trabalho do compressor e vapor. Todos os sinais necessários já existem como XMEAS na planta (purga, produto, composições, compressor, vapor).
- **[*2]** Correto, os modos não derivam de J. O modo é uma especificação de negócio (razão G/H, taxa de produção); J é o que se minimiza *dentro* do modo. Ou seja: modo = restrições/metas, J = critério. Juntos formam o que chamei de "política".
- **[*3]** "Planta digital" é ambíguo — lembra "digital twin", que já evitamos. Sugiro "planta simulada" ou "CPS".
- **[*4]** É uma tese válida e enxuta. Só observar J já é o primeiro marco; para ter conclusão, J precisa de uma referência (ex.: custo do caso base de Downs & Vogel, ou J sem distúrbio) para dizer se está "bom" ou "ruim".

Para isso, eu procurei algumas metodologias em outros artigos para transforar esses dados crús da planta em flags ou "vereditos" [*5] para o plant-supervisor. E o que eu entendo ser o plant-supervisor? Para mim é uma espécie de código que ao ser buildado pelo 'kube-builer' produz um artefato que pode ser 'instalado' em uma instância k8s (que no meu caso é um kind) [*6]. Daí, esse artefato expande aquela instância possibilitando que ela entenda esses vereditos emitidos [*7].

- **[*5]** Esse é o papel do historiador/middleware. O CLPM (Bradu 2018, já no Cap2) é um bom precedente de método: janela de tempo, limiar, e persistência por N janelas para não gerar falso positivo. Dá para aplicar a mesma lógica a J.
- **[*6]** Quase isso. O Kubebuilder só *gera o esqueleto* do código Go. O que se instala são duas coisas: (a) o **CRD**, um YAML que ensina a API do k8s um novo tipo de objeto; (b) o **controller**, uma imagem de container que roda como Pod e observa esses objetos.
- **[*7]** O k8s não "entende" o veredito — ele só o **armazena** (no `status` do objeto) e **avisa** quem estiver observando. Quem interpreta é o código do controller/historiador.

Eu entendo que esse artefato possibilitaria duas coisas:
1. Declarar uma função de custo a ser observada [*8];
2. Declarar políticas para essa função de custo [*9].

- **[*8]** Ajuste fino: o usuário não declara a *fórmula* (ela é fixa, Tabela 9, vive no código). Ele declara *quais termos, pesos e limites* usar.
- **[*9]** Sim. Uma política = modo (metas) + restrições + objetivo (minimizar custo, maximizar produção…). Várias declaradas, uma ativa por vez.

Assim, o k8s consegue acompanhar a planta e determinar quando ela está fora da função de custo [*10]. Qual é a vantagem de ter um k8s? Não sei, sinceramente [*11]. Considerando que hoje temos IA que pode fazer um software em 5 minutos, talvez seja um overhead ter um k8s para basicamente observar se a planta está nesses limites [*12].

- **[*10]** Quem determina é o controller; o k8s registra o resultado como `condition` (ex.: `PolicyCompliant=False`) e emite eventos.
- **[*11]** Vantagens reais: API declarativa padrão (spec/status) que qualquer ferramenta consome; histórico/auditoria de quem mudou a política e quando; políticas versionáveis em git (GitOps); reconciliação que se recupera sozinha de falhas; RBAC; ecossistema pronto (Prometheus, alertas).
- **[*12]** Para *só observar*, sim, é overhead — um serviço simples resolve. O custo de software não está em escrevê-lo, mas em operá-lo (contrato, auditoria, múltiplos consumidores, reinício). A tese fica mais forte se a pergunta for "k8s serve como plano de controle declarativo para supervisão industrial (Nível 4)?" — e o overhead vira um resultado a discutir, não uma fraqueza escondida.


---

Então, falando propriamente do que acho que será feito. Eu entendo que eu preciso implementar um microsserviço bem básico que farei em python. Ele será o 'historian' [*13]. Vai falar OPC-UA da planta e coletar dados. Isso está claro. O operator, que será um Pod que o Kind vai ter, irá observar esse 'historian' [*14]. Eu vou declarar políticas que ainda não sei quais serão [*15].

- **[*13]** Faz sentido. O `tep-ihm` já usa `asyncua` com a lista de nodes XMEAS/XMV mapeada — dá para reaproveitar esse trecho em vez de começar do zero.
- **[*14]** Sim. Na prática o operator consulta o historian periodicamente (ex.: HTTP a cada N segundos, via *requeue* do reconciler). O historian pode rodar fora do Kind (docker compose, como a planta) ou dentro, como Deployment — tanto faz no início.
- **[*15]** Sugestão para começar: uma única política, o **Modo 1** (caso base) de Downs & Vogel. Ele já tem metas e custo de referência publicados. As outras vêm depois.

Sinceramente não entendi ainda como esses modos e essa função de custo funcionam. Como implementar isso [*16]. Mas eu entendo que a função de custo estará dentro do Pod que terá a controller da planta [*17]. Daí vou declarar um manifesto no Kind para esse controller passar a me contar como está [*18].

- **[*16]** Versão curta: J é uma conta — a cada janela, multiplica cada vazão/composição pelo seu preço e soma, resultando em $/h. O modo é a "encomenda": produzir X kg/h com razão G/H = Y. O veredito responde três perguntas: estou cumprindo a encomenda? estou dentro das restrições? quanto J está acima da referência?
- **[*17]** Viável — é uma soma de produtos, Go resolve bem. Aí o historian só entrega médias por janela. A alternativa é calcular J no historian (Python, mais fácil de testar contra CSV). O importante é J morar em **um** lugar só.
- **[*18]** Quase: o controller é instalado uma vez (`make deploy`). O manifesto que você aplica (`kubectl apply`) declara a **política**; o controller responde preenchendo o `status` desse objeto, visível em `kubectl get` e no `tep-ihm`.

De fato, não me interessa aqui se existe utilidade ou não usar o k8s nesse controle. O que eu quero é fazer assim. Se terá vantagens, isso é outra conversa [*19].

- **[*19]** Justo — é uma tese de viabilidade arquitetural, não de comparação. Vale deixar isso explícito como decisão de escopo no Cap1, para a banca não cobrar a comparação.

---

Sobre **[*17]** → Eu gostaria que esse Controller fosse independente da planta na verdade [*20]. É uma coisa (entendo que será um Pod) que dá a uma instância k8s qualquer o poder de entender plantas industriais. Para entender plantas industriais em alto nível é preciso uma função objetivo (ou de 'custo') [*21]. Considerando que essa função é particular de TEP, realmente fica estranho implementa-la dentro do Operator. Eu acho que eu deveria conseguir parametrizar essa função. Eu acho que esse Operator deve permitir o usuário declarar a função, deveria ser algo parametrizável [*22].

- **[*20]** Boa decisão, e fortalece a tese: o operator vira genérico ("k8s entende plantas industriais") e o TEP passa a ser só o primeiro caso de uso. Consequência: os nomes do CRD também ficam genéricos (`Plant`, não `TEPPlant`).
- **[*21]** Função objetivo + restrições. Só J não basta: "custo baixo mas pressão acima do limite" tem que dar veredito ruim.
- **[*22]** Isso revisa o **[*8]**: a fórmula sai do código e vira manifesto. Dá para fazer porque a J de Downs & Vogel é uma soma de termos do tipo `coeficiente × sinal` ou `coeficiente × sinal × sinal` (vazão × composição). Então o usuário declara uma lista de termos, cada um com coeficiente e chaves de sinal; a Tabela 9 vira um YAML. Uma linguagem de expressão livre (ex.: CEL, que o k8s já usa) fica para depois, se a lista de termos não bastar.
- **[*23]** Consequência no historian: ele também precisa ser genérico — responder "média do sinal X na janela T" para qualquer chave, sem saber o que é TEP. Assim, todo conhecimento específico da planta mora só nos manifestos. Um desenho possível: `Plant` (endpoint), `CostFunction` (termos) e `OperatingPolicy` (metas/restrições, apontando para uma `CostFunction`).