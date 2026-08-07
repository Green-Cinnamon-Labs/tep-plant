/* bin/tep_plant.rs */

/** `tep-plant` — a aplicação: monta a Simulation da Tennessee Eastman Plant. `Simulation` cuida de
StateRegistry/thread por dentro — esse binário não monta isso manualmente.

NOTA (2026-07-30): não há mais adaptador de rede configurado aqui — a camada de exposição externa
(OPC-UA, discovery de Sensor/Actuator/Controller) foi retirada do `monjolo` por enquanto, pendente
de redesenho. Rodar este binário hoje só integra a planta no tempo, sem expor nada pra fora.

A condição inicial (`Snapshot`) é decisão deste binário — de onde o arquivo vem é problema da
aplicação, não do modelo. `model::new` não cabe direto em `set_model()` como ponteiro de função
(recebe `&Snapshot` também), por isso a closure abaixo.

Roda com: cargo run --bin tep-plant
*/

use monjolo::numerical_method::NumericalMethod;
use monjolo::simulation::Simulation;
use monjolo::snapshot::Snapshot;
use tennessee_eastman_process::model;

const INITIAL_STATE_PATH: &str = "src/snapshots/te_exp3_snapshot.toml";

fn main() {
    let initial = Snapshot::from_file(INITIAL_STATE_PATH)
        .unwrap_or_else(|e| panic!("falha ao carregar condição inicial de '{INITIAL_STATE_PATH}': {e}"));

    let mut simulation = Simulation::new();

    simulation.set_model(move |registry| model::new(registry, &initial));
    simulation.set_numerical_method(NumericalMethod::RK4);

    simulation.run().expect("run encerrou com erro");
}
