/* src/main.rs */

/** `tep-plant` — a aplicação: monta a Simulation da Tennessee Eastman Plant. `Simulation` cuida de
StateRegistry/thread por dentro — esse binário não monta isso manualmente.

NOTA (2026-08-13, branch feat/proc-macro-components): não existe mais `build_tep()`/`set_model()`
aqui — Reactor/Separator/Stripper/Compressor (agora `#[dynamic_model]`), os 12 atuadores, os 6
sensores e o controller se auto-registram via `inventory::submit!` escondido; `Simulation::run()`
descobre e monta a árvore de avaliação inteira sozinho. Este binário só diz ONDE está a condição
inicial (`set_config_path`) — "de onde vem o arquivo é problema da aplicação, não do modelo/
framework" continua valendo, só que agora é o único trabalho que sobra aqui.

NOTA (2026-08-15): adaptador OPC-UA ligado sob a feature `opcua` (default OFF) — expõe os
Sensors/Actuators já catalogados em StateRegistry (`sensor_names()`/`actuator_names()`,
`monjolo::state_registry`) via `monjolo::adapter::opcua`. Escrita de atuador não atravessa o `Rc`
pra thread do adapter: a ponte é um canal `(nome, valor)`, drenado a cada tick pela Thread da planta
— ver `Simulation::spawn_adapter_thread`/`spawn_plant_thread` (`monjolo::simulation`). Sem a
feature, este binário integra a planta no tempo sem expor nada pra fora, como antes.

Roda com: cargo run --bin tep-plant [--features opcua]
*/

/* `extern crate tennessee_eastman_process;` não é resquício de edição antiga (2018+ não precisa
disso pra USAR um crate) — é a única forma de o linker incluir o crate no binário final quando NADA
aqui referencia um símbolo dele por nome. Sem isso, `cargo build` compila a lib normalmente, mas o
`.exe` final não puxa o código de `actuators/`/`sensors/`/`controllers/`/`units/` do `.rlib` —
cada `inventory::submit!` escondido nesses módulos nunca roda, e `Simulation::run()` descobre zero
componentes (não é bug de `inventory`; é assim que Rust sempre linkou binário↔biblioteca — só ficou
visível agora que main() não chama mais nada da lib por nome).
*/
extern crate tennessee_eastman_process;

use monjolo::numerical_method::NumericalMethod;
use monjolo::simulation::Simulation;

const CONFIG_PATH: &str = "config/application.toml";
/* Porta IANA padrão de OPC-UA. Host é `127.0.0.1`, não `0.0.0.0` — de propósito: async-opcua-server
não distingue "endereço de bind" de "endereço anunciado" (`ServerInfo::base_endpoint()`, em
async-opcua-server/src/info.rs, monta o EndpointUrl que o servidor devolve em GetEndpoints/
FindServers a partir do MESMO `tcp_config.host` do bind). Com `0.0.0.0`, o servidor aceita conexão
em qualquer interface, mas anuncia a si mesmo como "opc.tcp://0.0.0.0:...", um endereço que nenhum
cliente consegue discar de verdade — quebra qualquer client que confie no EndpointUrl reportado pra
reconectar (ex.: UaExpert), mesmo que a conexão inicial/manual funcione (ex.: opcua-commander,
conexão direta sem essa segunda etapa). Se um dia isso precisar ser alcançável de outra máquina na
rede, precisa virar configurável (bind em 0.0.0.0, anunciar o IP real da máquina) — não dá pra ter
os dois com essa constante sozinha hoje.
*/
#[cfg(feature = "opcua")]
const OPCUA_ENDPOINT: &str = "opc.tcp://127.0.0.1:4840/tep/server/";

fn main() {
    let mut simulation = Simulation::new();

    simulation.set_config_path(CONFIG_PATH);
    simulation.set_numerical_method(NumericalMethod::RK4);

    #[cfg(feature = "opcua")]
    simulation.set_adapter(monjolo::adapter::AdapterConfig::OpcUa {
        endpoint: OPCUA_ENDPOINT.to_string(),
    });

    simulation.run().expect("run encerrou com erro");
}
