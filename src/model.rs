// tep/model.rs
//
// TennesseeEastmanModel — implementação concreta de DynamicModel para a
// planta TEP.
//
// new() é uma sequência de add_dynamic — os 4 subsistemas químicos
// (Flows/Heat/Measurements ainda não migrados, ver flows.rs) mais os dois
// atuadores físicos: a válvula de água de resfriamento do reator e o
// agitador. Cada um se inscreve no StateRegistry recebido durante sua
// própria construção; quem chama new() ainda precisa chamar
// `registry.resolve()` depois — TennesseeEastmanModel não decide isso.
//
// NOTA (2026-07-30): `Sensor`/`Valve`/`Agitator` agora moram aqui (não mais
// dentro do `monjolo`) — implementam os traits mínimos `monjolo::sensor::
// Sensor`/`monjolo::actuator::Actuator`. `Valve`/`Agitator` são as duas
// coisas ao mesmo tempo: `DynamicModel` (física real, `add_dynamic`'d) *e*
// `Actuator` (aceitam comando via `write()`) — não há caixa de correio
// separada. A camada de descoberta externa (quais sensores/atuadores ficam
// visíveis pra um adaptador de rede) foi retirada por enquanto, pendente de
// redesenho — `new()` constrói os `Sensor`s e os mantém vivos
// (`self.sensors`), mas não expõe nada ainda.
//
// new() recebe um `&Snapshot` (condição inicial já carregada por quem
// chama — não é responsabilidade do modelo saber de onde vem o arquivo) e
// repassa pra cada subsistema buscar só as chaves que interessam pra ele
// (ver Reactor::new/Separator::new/Stripper::new/Compressor::new).

use monjolo::dynamic_model::{CompositeDynamicModel, DynamicModel};
use monjolo::snapshot::Snapshot;
use monjolo::state_registry::StateRegistry;

use monjolo::sensor::model::{Ideal, Sensor};

use crate::subsystems::actuators::{Agitator, Valve};
use crate::subsystems::compressor::Compressor;
use crate::subsystems::reactor::Reactor;
use crate::subsystems::separator::Separator;
use crate::subsystems::stripper::Stripper;

/// `VTAU(10)` em `teprob.f`: constante de tempo da válvula de água de
/// resfriamento do reator — 5 segundos, convertidos pra horas (unidade que
/// o resto da física do TEP usa, ver `Simulation::set_dt_hours`).
const COOLING_WATER_VALVE_TAU_HOURS: f64 = 5.0 / 3600.0;

/// `VTAU(12)` em `teprob.f`: constante de tempo do agitador — também 5
/// segundos.
const AGITATOR_TAU_HOURS: f64 = 5.0 / 3600.0;

pub struct TennesseeEastmanModel {
    models: Vec<Box<dyn DynamicModel>>,
    /// Mantidos vivos (o `ReadProxy` de cada um precisa continuar
    /// existindo pra ser útil depois) — nada os expõe ainda, pendente do
    /// redesenho da camada de descoberta externa.
    #[allow(dead_code)]
    sensors: Vec<Sensor>,
}

impl TennesseeEastmanModel {
    pub fn new(registry: &mut StateRegistry, initial: &Snapshot) -> Self {
        
        let mut tep = Self {
            models: Vec::new(),
            sensors: Vec::new(),
        };

        tep.add_dynamic(Box::new(Reactor::new(registry, initial)));
        tep.add_dynamic(Box::new(Separator::new(registry, initial)));
        tep.add_dynamic(Box::new(Stripper::new(registry, initial)));
        tep.add_dynamic(Box::new(Compressor::new(registry, initial)));
        tep.add_dynamic(Box::new(Valve::new(
            registry,
            "cooling_water",
            COOLING_WATER_VALVE_TAU_HOURS,
        )));
        tep.add_dynamic(Box::new(Agitator::new(registry, AGITATOR_TAU_HOURS)));

        // Só os valores termodinâmicos que já são reais hoje (Reactor/
        // Separator já computam; Flows/Heat/Measurements — o resto do
        // XMEAS — ainda são `todo!()`, ver
        // docs/issue55_opcua_refactor/eval_signal_exposure.md). Sensores
        // companheiros dos dois atuadores confirmam a posição/velocidade
        // resultante — evolui via a defasagem de 1ª ordem, não é
        // instantânea.
        for key in [
            "reactor.temperature",
            "reactor.pressure",
            "separator.temperature",
            "separator.pressure",
            "valve.cooling_water.position",
            "agitator.speed",
        ] {
            let sensor = Sensor::new(registry, key, Box::new(Ideal))
                .unwrap_or_else(|e| panic!("TennesseeEastmanModel: sensor '{key}': {e}"));
            tep.sensors.push(sensor);
        }

        tep
    }
}

impl DynamicModel for TennesseeEastmanModel {
    fn name(&self) -> &'static str {
        "TennesseeEastmanModel"
    }

    fn evaluate(&self) {
        self.evaluate_children();
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
    /// Compressor/Valve/Agitator::new()) + resolve() (sem erro) + evaluate()
    /// sem panic (nenhum Proxy é lido antes de ser resolvido).
    #[test]
    fn wires_and_evaluates_without_panicking() {
        let registry = StateRegistry::shared();
        let initial = Snapshot::from_pairs(&[]);
        let tep = TennesseeEastmanModel::new(&mut registry.borrow_mut(), &initial);

        registry.borrow_mut().resolve().expect("todo input deveria ter provedor");

        tep.evaluate();
    }
}
