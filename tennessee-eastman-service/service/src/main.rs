// main.rs
//
// Ponto de entrada: instancia a Simulation e roda. O servidor OPC-UA entra
// aqui depois — por enquanto só prova que a cadeia funciona de ponta a ponta.

use simulation_framework::simulation::Simulation;
use te_core::model::TennesseeEastmanModel;

fn main() {
    let simulation = Simulation::new(TennesseeEastmanModel::new)
        .expect("falha ao resolver o StateRegistry — algum `need` não tem provedor");

    simulation.run();
}
