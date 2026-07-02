// dynamics/model_v2.rs
//
// Composite pattern for dynamic systems.
// Each subsystem (plant core, actuator, etc.) implements DynamicModelV2
// and contributes its own state slice and derivatives to a single vector
// integrated by one RK4 pass.

pub trait DynamicModelV2 {
    fn state_size(&self) -> usize;
    fn derivatives(&mut self, state: &[f64]) -> Vec<f64>;
    fn name(&self) -> &'static str { "unnamed" }
}

// ── CompositeDynamicModel ─────────────────────────────────────────────────────

pub struct CompositeDynamicModel {
    children: Vec<Box<dyn DynamicModelV2>>,
}

impl CompositeDynamicModel {
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    pub fn add(mut self, child: Box<dyn DynamicModelV2>) -> Self {
        self.children.push(child);
        self
    }

    pub fn state_offsets(&self) -> Vec<usize> {
        let mut offsets = vec![0usize];
        for child in &self.children {
            let last = *offsets.last().unwrap();
            offsets.push(last + child.state_size());
        }
        offsets
    }
}

impl DynamicModelV2 for CompositeDynamicModel {
    fn state_size(&self) -> usize {
        self.children.iter().map(|c| c.state_size()).sum()
    }

    fn derivatives(&mut self, state: &[f64]) -> Vec<f64> {
        let offsets = self.state_offsets();
        let mut yp = Vec::with_capacity(self.state_size());
        for (i, child) in self.children.iter_mut().enumerate() {
            let slice = &state[offsets[i]..offsets[i + 1]];
            yp.extend(child.derivatives(slice));
        }
        yp
    }

    fn name(&self) -> &'static str { "composite" }
}
