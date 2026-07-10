// src/bin/tep_plant.rs
//
// `tep-plant` — a aplicação: monta a Simulation da Tennessee Eastman Plant
// e configura o adaptador OPC-UA (interface externa de hoje — não é a
// identidade do binário, só uma forma de expor a simulação). `Simulation`
// cuida de StateRegistry/thread/canal por dentro (ver
// docs/issue55_opcua_refactor/plan_refactor.md, seção 10-11) — esse binário
// não monta isso manualmente.
//
// Os sensores/atuadores em si NÃO são declarados aqui — são declarados por
// `TennesseeEastmanModel::new()` (seção 11.8 do plano), porque é o modelo
// quem sabe quais dos seus próprios slots fazem sentido expor. Este binário
// só monta a `Simulation` e decide se/onde o OPC-UA sobe.
//
// A condição inicial (`Snapshot`, seção 11.9 do plano) também é decisão
// deste binário — de onde o arquivo vem é problema da aplicação, não do
// modelo. `TennesseeEastmanModel::new` deixou de caber direto em
// `set_model()` como ponteiro de função (agora recebe `&Snapshot` também),
// por isso a closure abaixo.
//
// Roda com: cargo run --bin tep-plant

use simulation_framework::simulation::Simulation;
use simulation_framework::snapshot::Snapshot;
use tennessee_eastman_process::model::TennesseeEastmanModel;

const OPCUA_ENDPOINT: &str = "opc.tcp://0.0.0.0:4840/tep/server/";
const INITIAL_STATE_PATH: &str = "src/snapshots/te_exp3_snapshot.toml";

fn main() {
    let initial = Snapshot::from_file(INITIAL_STATE_PATH)
        .unwrap_or_else(|e| panic!("falha ao carregar condição inicial de '{INITIAL_STATE_PATH}': {e}"));

    let mut simulation = Simulation::new();
    simulation.set_model(move |registry| TennesseeEastmanModel::new(registry, &initial));

    println!("tep-plant: OPC-UA em {OPCUA_ENDPOINT}");
    simulation.start_opcua_server(OPCUA_ENDPOINT);

    simulation.run_model().expect("run_model encerrou com erro");
}
