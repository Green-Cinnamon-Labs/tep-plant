// tep/model.rs
//
// TennesseeEastmanModel — implementação concreta de DynamicModel para a planta
// TEP. Reconstrução do zero sobre o contrato atual (ver
// docs/issue55_opcua_refactor/plan_refactor.md, seções 1-2). A versão
// anterior, com toda a química/distúrbio já resolvidos por dynamics(), está
// preservada e comentada em _deprecated.rs.
//
// new() é uma sequência de add_dynamic (seção 2.6) — só os 4 componentes que
// já têm evaluate() real (Flows/Heat/Measurements/Valve/Agitator ainda não
// migrados, ver flows.rs). Cada um se inscreve no StateRegistry recebido
// durante sua própria construção; quem chama new() ainda precisa chamar
// `registry.resolve()` depois — TennesseeEastmanModel não decide isso.
//
// new() também é quem declara os sensores observáveis (add_sensor, seção
// 11.8 do plano) — mesma relação que add_dynamic tem com composição:
// TennesseeEastmanModel é quem sabe quais dos seus próprios slots fazem
// sentido expor pra fora (drawio, aba "arquitetura": "TennesseeEastmanModel
// --DECLARA--> Sensor"). `Simulation::set_model()` lê isso via
// `DynamicModel::sensors()` — quem monta a `Simulation` (ex.: tep_plant.rs)
// não precisa mais saber esses nomes/chaves.
//
// new() recebe um `&Snapshot` (condição inicial já carregada por quem
// chama — não é responsabilidade do modelo saber de onde vem o arquivo) e
// repassa pra cada subsistema buscar só as chaves que interessam pra ele
// (ver Reactor::new). Só o Reactor usa isso por enquanto — Separator/
// Stripper/Compressor ainda nascem sem condição inicial.

use simulation_framework::dynamic_model::{CompositeDynamicModel, DynamicModel};
use simulation_framework::snapshot::Snapshot;
use simulation_framework::state_registry::StateRegistry;

use crate::subsystems::compressor::Compressor;
use crate::subsystems::reactor::Reactor;
use crate::subsystems::separator::Separator;
use crate::subsystems::stripper::Stripper;

pub struct TennesseeEastmanModel {
    models: Vec<Box<dyn DynamicModel>>,
    sensors: Vec<(String, String)>,
}

impl TennesseeEastmanModel {
    pub fn new(registry: &mut StateRegistry, initial: &Snapshot) -> Self {
        let mut tep = Self { models: Vec::new(), sensors: Vec::new() };

        tep.add_dynamic(Box::new(Reactor::new(registry, initial)));
        tep.add_dynamic(Box::new(Separator::new(registry)));
        tep.add_dynamic(Box::new(Stripper::new(registry)));
        tep.add_dynamic(Box::new(Compressor::new(registry)));

        // Só os valores termodinâmicos que já são reais hoje (Reactor/
        // Separator já computam; Flows/Heat/Measurements — o resto do
        // XMEAS — ainda são `todo!()`, ver
        // docs/issue55_opcua_refactor/eval_signal_exposure.md).
        tep.add_sensor("TEP/Reactor/Temperature", "reactor.temperature");
        tep.add_sensor("TEP/Reactor/Pressure", "reactor.pressure");
        tep.add_sensor("TEP/Separator/Temperature", "separator.temperature");
        tep.add_sensor("TEP/Separator/Pressure", "separator.pressure");

        tep
    }

    fn add_sensor(&mut self, name: &str, key: &str) {
        self.sensors.push((name.to_string(), key.to_string()));
    }
}

impl DynamicModel for TennesseeEastmanModel {
    fn name(&self) -> &'static str {
        "TennesseeEastmanModel"
    }

    fn evaluate(&self) {
        self.evaluate_children();
    }

    fn sensors(&self) -> Vec<(String, String)> {
        self.sensors.clone()
    }
}

impl CompositeDynamicModel for TennesseeEastmanModel {
    fn models(&self) -> &[Box<dyn DynamicModel>] {
        &self.models
    }

    fn models_mut(&mut self) -> &mut Vec<Box<dyn DynamicModel>> {
        &mut self.models
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prova que a cadeia inteira funciona: construção + inscrição (add_dynamic
    /// já dispara subscribe() dentro de cada Reactor/Separator/Stripper/
    /// Compressor::new()) + resolve() (sem erro — reactor.temperature e
    /// separator.temperature têm provedor) + evaluate() sem panic (nenhum
    /// Proxy é lido antes de ser resolvido).
    #[test]
    fn wires_and_evaluates_without_panicking() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);
        let tep = TennesseeEastmanModel::new(&mut registry.borrow_mut(), &initial);

        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        tep.evaluate();
    }
}
