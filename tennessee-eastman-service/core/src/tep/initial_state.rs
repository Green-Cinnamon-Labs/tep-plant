use serde::Deserialize;
use std::fs;

const N_STATES: usize = 50;

#[derive(Debug, Deserialize)]
pub struct InitialState {
    pub state: StateSections,
}

#[derive(Debug, Deserialize)]
pub struct StateSections {
    pub reactor_vapor:    Components,
    pub reactor:          EnergySection,
    pub separator_vapor:  Components,
    pub separator:        EnergySection,
    pub stripper_liquid:  Components,
    pub stripper:         EnergySection,
    pub compressor_vapor: Components,
    pub compressor:       EnergySection,
    pub cooling:          CoolingSection,
    pub valves:           ValveSection,
}

#[derive(Debug, Deserialize)]
pub struct EnergySection {
    pub energy: f64,
}

#[derive(Debug, Deserialize)]
pub struct CoolingSection {
    pub reactor_water_temp:   f64,
    pub separator_water_temp: f64,
}

#[derive(Debug, Deserialize)]
pub struct ValveSection {
    pub d_feed:                   f64,
    pub e_feed:                   f64,
    pub a_feed:                   f64,
    pub a_c_feed:                 f64,
    pub compressor_recycle_valve: f64,
    pub purge_valve:              f64,
    pub separator_underflow:      f64,
    pub stripper_product:         f64,
    pub stripper_steam_valve:     f64,
    pub reactor_cooling_water:    f64,
    pub condenser_cooling_water:  f64,
    pub agitator_speed:           f64,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Components {
    pub A: f64,
    pub B: f64,
    pub C: f64,
    pub D: f64,
    pub E: f64,
    pub F: f64,
    pub G: f64,
    pub H: f64,
}

impl InitialState {

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Erro lendo arquivo: {}", e))?;

        let parsed: InitialState = toml::from_str(&content)
            .map_err(|e| format!("Erro parseando TOML: {}", e))?;

        parsed.validate()?;
        Ok(parsed)
    }

    fn validate(&self) -> Result<(), String> {
        Ok(()) // validações físicas podem ser adicionadas depois
    }
    
    pub fn flatten(&self) -> [f64; N_STATES] {
        let s = &self.state;
        let mut x = [0.0f64; N_STATES];
        let mut i = 0;

        // YY(1-8): reactor vapor holdup
        push_components(&mut x, &mut i, &s.reactor_vapor);
        // YY(9): reactor energy
        x[i] = s.reactor.energy; i += 1;

        // YY(10-17): separator vapor holdup
        push_components(&mut x, &mut i, &s.separator_vapor);
        // YY(18): separator energy
        x[i] = s.separator.energy; i += 1;

        // YY(19-26): stripper liquid holdup
        push_components(&mut x, &mut i, &s.stripper_liquid);
        // YY(27): stripper energy
        x[i] = s.stripper.energy; i += 1;

        // YY(28-35): compressor vapor holdup
        push_components(&mut x, &mut i, &s.compressor_vapor);
        // YY(36): compressor energy
        x[i] = s.compressor.energy; i += 1;

        // YY(37-38): cooling water temperatures
        x[i] = s.cooling.reactor_water_temp;   i += 1;
        x[i] = s.cooling.separator_water_temp; i += 1;

        // YY(39-50): valve positions
        x[i] = s.valves.d_feed;                   i += 1;
        x[i] = s.valves.e_feed;                   i += 1;
        x[i] = s.valves.a_feed;                   i += 1;
        x[i] = s.valves.a_c_feed;                 i += 1;
        x[i] = s.valves.compressor_recycle_valve; i += 1;
        x[i] = s.valves.purge_valve;              i += 1;
        x[i] = s.valves.separator_underflow;      i += 1;
        x[i] = s.valves.stripper_product;         i += 1;
        x[i] = s.valves.stripper_steam_valve;     i += 1;
        x[i] = s.valves.reactor_cooling_water;    i += 1;
        x[i] = s.valves.condenser_cooling_water;  i += 1;
        x[i] = s.valves.agitator_speed;           i += 1;

        assert!(i == N_STATES, "flatten preencheu {} estados, esperado {}", i, N_STATES);

        x
    }
}

fn push_components(x: &mut [f64], i: &mut usize, c: &Components) {
    x[*i] = c.A; *i += 1;
    x[*i] = c.B; *i += 1;
    x[*i] = c.C; *i += 1;
    x[*i] = c.D; *i += 1;
    x[*i] = c.E; *i += 1;
    x[*i] = c.F; *i += 1;
    x[*i] = c.G; *i += 1;
    x[*i] = c.H; *i += 1;
}