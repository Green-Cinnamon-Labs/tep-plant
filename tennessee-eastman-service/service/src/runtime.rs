// runtime.rs

use std::fs::File;
use std::io::{BufWriter, Write};

use te_core::plant::Plant;
use te_core::params::Params;

use crate::config::Config;
use crate::shared::{SharedPlant, MetricsSnapshot, AlarmSnapshot};
use crate::dashboard::Dashboard;
use crate::resolver::resolve;

pub fn run(config: Config, shared: SharedPlant) {

    let resolved = resolve(&config);

    let params = Params::default();
    let mut plant = Plant::with_state_values(
        &resolved.initial_state,
        resolved.model,
        params,
        resolved.integrator,
    );

    for &idv in &config.active_idv {
        if idv >= 1 && idv <= plant.bus.inputs.dv.len() {
            plant.bus.inputs.dv[idv - 1] = 1.0;
        }
    }

    // ── Cold start ─────────────────────────────────────────────────────────────
    let nominal_mv: Vec<f64> = plant.bus.inputs.mv.clone();
    let ramp_duration = config.ramp_duration;

    // Disable disturbances during ramp
    let saved_dv = plant.bus.inputs.dv.clone();
    for v in plant.bus.inputs.dv.iter_mut() {
        *v = 0.0;
    }

    // Close feed valves at t=0
    for i in 0..4 {
        plant.bus.inputs.mv[i] = 0.0;
    }

    let mut disturbances_restored = false;

    // ── CSV logger (disabled in headless mode) ───────────────────────────────
    let mut csv: Option<BufWriter<File>> = if !config.headless {
        let csv_file = File::create("simulation_log.csv").expect("Failed to create CSV file");
        let mut w = BufWriter::new(csv_file);

        let mut header = String::from("t_h");
        for i in 1..=22 { header.push_str(&format!(",XMEAS({})", i)); }
        for i in 1..=12 { header.push_str(&format!(",XMV({})", i)); }
        header.push_str(",deriv_norm");
        for i in 0..10  { header.push_str(&format!(",YY[{}]", i)); }
        for i in 27..35 { header.push_str(&format!(",YY[{}]", i)); }
        writeln!(w, "{}", header).unwrap();

        Some(w)
    } else {
        None
    };

    // ── Dashboard (disabled in headless mode) ────────────────────────────────
    let mut dashboard: Option<Dashboard> = if !config.headless {
        Some(Dashboard::new().expect("Failed to initialize terminal dashboard"))
    } else {
        eprintln!("Headless mode: no TUI, no CSV. gRPC only.");
        None
    };

    let mut t_simulation  = 0.0_f64;
    let mut t_operational = 0.0_f64;
    let mut isd_active    = false;
    let mut last_state: Option<Vec<f64>> = None;
    let mut clean_exit    = false;

    loop {
        // Read control state (fast lock): pause + speed factor
        let (is_paused, speed_factor) = {
            let s = shared.lock().unwrap();
            (s.paused, s.speed_factor)
        };

        if is_paused {
            // While paused: idle at the configured rate to keep gRPC responsive
            let pause_delay = if speed_factor > 0.0 { (3.6_f64 / speed_factor).min(0.1) } else { 0.05 };
            std::thread::sleep(std::time::Duration::from_secs_f64(pause_delay));
            continue;
        }

        // Sync disturbances from shared state (allows runtime toggling)
        {
            let state = shared.lock().unwrap();

            if disturbances_restored {
                for i in 0..plant.bus.inputs.dv.len() {
                    plant.bus.inputs.dv[i] = 0.0;
                }
                for &idv in &state.active_idv {
                    if idv >= 1 && idv <= plant.bus.inputs.dv.len() {
                        plant.bus.inputs.dv[idv - 1] = 1.0;
                    }
                }
                // Sync configurable step magnitudes (IDV 3, 4, 5)
                for &idx in &[2usize, 3, 4] {
                    let idv_num = idx + 1;
                    if let Some(&mag) = state.idv_magnitudes.get(&idv_num) {
                        plant.model.set_idv_step_mag(idx, mag);
                    }
                }
            }
        }

        if !isd_active {
            plant.step(config.dt);
            t_simulation   += config.dt;
            plant.bus.time += config.dt;

            // ── Cold start ramp ────────────────────────────────────────────────
            let progress = (t_simulation / ramp_duration).clamp(0.0, 1.0);
            for i in 0..4 {
                plant.bus.inputs.mv[i] = nominal_mv[i] * progress;
            }

            // Restore disturbances once ramp completes
            if progress >= 1.0 && !disturbances_restored {
                plant.bus.inputs.dv = saved_dv.clone();
                disturbances_restored = true;
            }

            // ── Shared state: run controllers, write metrics ─────────────────
            {
                let mut state = shared.lock().unwrap();

                // Controllers (injected)
                state.bank.step(&plant.bus.outputs.xmeas, &mut plant.bus.inputs.mv);

                // Write metrics snapshot for gRPC readers
                state.metrics = MetricsSnapshot {
                    t_h: t_simulation,
                    xmeas: plant.bus.outputs.xmeas.to_vec(),
                    xmv: plant.bus.inputs.mv.to_vec(),
                    alarms: Vec::new(), // filled below after snap
                    deriv_norm: 0.0,
                    isd_active: false,
                };
            }

            let snap = plant.snapshot();

            // Update alarm info in shared state
            {
                let mut state = shared.lock().unwrap();
                state.metrics.deriv_norm = snap.solver.deriv_norm;
                state.metrics.alarms = snap.alarms.iter().map(|a| AlarmSnapshot {
                    variable: a.name.to_string(),
                    condition: String::new(),
                    active: a.active,
                }).collect();
            }

            // ── CSV row ──────────────────────────────────────────────────────
            if let Some(ref mut w) = csv {
                let mut row = format!("{:.6}", t_simulation);
                for i in 0..22 { row.push_str(&format!(",{:.6}", snap.xmeas[i])); }
                for i in 0..12 { row.push_str(&format!(",{:.6}", snap.xmv[i])); }
                row.push_str(&format!(",{:.6e}", snap.solver.deriv_norm));
                for i in 0..10  { row.push_str(&format!(",{:.6e}", snap.state.get(i).copied().unwrap_or(f64::NAN))); }
                for i in 27..35 { row.push_str(&format!(",{:.6e}", snap.state.get(i).copied().unwrap_or(f64::NAN))); }
                writeln!(w, "{}", row).unwrap();
            }

            // Advance t_operational only while no alarms are active.
            let any_alarm = snap.alarms.iter().any(|a| a.active);
            if !any_alarm {
                t_operational += config.dt;
            }

            // Shutdown detection
            if snap.solver.deriv_norm == 0.0 && any_alarm {
                isd_active = true;
                {
                    let mut state = shared.lock().unwrap();
                    state.metrics.isd_active = true;
                }
                eprintln!("SIMULATION STOPPED: plant shutdown condition reached");
                eprintln!("  t_simulation  = {:.3} h", t_simulation);
                eprintln!("  t_operational = {:.3} h", t_operational);
                if let Some(ref mut w) = csv { let _ = w.flush(); }
            }

            // ── Time limit ────────────────────────────────────────────────────
            last_state = Some(snap.state.clone());
            if let Some(max_t) = config.max_sim_time_h {
                if t_simulation >= max_t {
                    eprintln!("SIMULATION TIME LIMIT: {:.3} h reached", t_simulation);
                    if let Some(ref mut w) = csv { let _ = w.flush(); }
                    clean_exit = true;
                    break;
                }
            }

            // ── Dashboard render ──────────────────────────────────────────────
            if let Some(ref mut d) = dashboard {
                let running = d.render(&snap).expect("Failed to render dashboard");
                if !running {
                    clean_exit = true;
                    break;
                }
            }
        } else {
            // ISD active — in headless mode just sleep, in TUI mode render
            if let Some(ref mut d) = dashboard {
                let snap = plant.snapshot();
                let running = d.render(&snap).expect("Failed to render dashboard");
                if !running { break; }
            } else {
                // Headless: sleep briefly to avoid busy-wait, gRPC still serves metrics
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // Speed-controlled delay in ISD state
            let isd_delay = if speed_factor > 0.0 { 3.6_f64 / speed_factor } else { 0.0 };
            if isd_delay > 0.0 {
                std::thread::sleep(std::time::Duration::from_secs_f64(isd_delay));
            }
        }

        if !isd_active {
            let step_delay = if speed_factor > 0.0 { 3.6_f64 / speed_factor } else { 0.0 };
            if step_delay > 0.0 {
                std::thread::sleep(std::time::Duration::from_secs_f64(step_delay));
            }
        }
    }

    if let Some(ref mut w) = csv { let _ = w.flush(); }

    // ── Snapshot on clean exit ─────────────────────────────────────────────────
    if clean_exit {
        if let (Some(ref path), Some(ref state)) = (&config.snapshot_path, &last_state) {
            write_snapshot_toml(path, state, t_simulation);
        }
    }
}

fn write_snapshot_toml(path: &str, state: &[f64], t_h: f64) {
    let file = File::create(path).expect("Failed to create snapshot file");
    let mut w = BufWriter::new(file);

    let comps = ["A", "B", "C", "D", "E", "F", "G", "H"];
    let valve_names = [
        "d_feed", "e_feed", "a_feed", "a_c_feed",
        "compressor_recycle_valve", "purge_valve",
        "separator_underflow", "stripper_product", "stripper_steam_valve",
        "reactor_cooling_water", "condenser_cooling_water", "agitator_speed",
    ];

    writeln!(w, "[meta]").unwrap();
    writeln!(w, "mode = 1").unwrap();
    writeln!(w, "description = \"TEP snapshot at t = {:.4} h\"", t_h).unwrap();
    writeln!(w, "source = \"auto-snapshot from simulation\"").unwrap();
    writeln!(w).unwrap();

    writeln!(w, "[state.reactor_vapor]").unwrap();
    for (i, c) in comps.iter().enumerate() {
        writeln!(w, "{} = {}", c, state[i]).unwrap();
    }
    writeln!(w).unwrap();

    writeln!(w, "[state.reactor]").unwrap();
    writeln!(w, "energy = {}", state[8]).unwrap();
    writeln!(w).unwrap();

    writeln!(w, "[state.separator_vapor]").unwrap();
    for (i, c) in comps.iter().enumerate() {
        writeln!(w, "{} = {}", c, state[9 + i]).unwrap();
    }
    writeln!(w).unwrap();

    writeln!(w, "[state.separator]").unwrap();
    writeln!(w, "energy = {}", state[17]).unwrap();
    writeln!(w).unwrap();

    writeln!(w, "[state.stripper_liquid]").unwrap();
    for (i, c) in comps.iter().enumerate() {
        writeln!(w, "{} = {}", c, state[18 + i]).unwrap();
    }
    writeln!(w).unwrap();

    writeln!(w, "[state.stripper]").unwrap();
    writeln!(w, "energy = {}", state[26]).unwrap();
    writeln!(w).unwrap();

    writeln!(w, "[state.compressor_vapor]").unwrap();
    for (i, c) in comps.iter().enumerate() {
        writeln!(w, "{} = {}", c, state[27 + i]).unwrap();
    }
    writeln!(w).unwrap();

    writeln!(w, "[state.compressor]").unwrap();
    writeln!(w, "energy = {}", state[35]).unwrap();
    writeln!(w).unwrap();

    writeln!(w, "[state.cooling]").unwrap();
    writeln!(w, "reactor_water_temp   = {}", state[36]).unwrap();
    writeln!(w, "separator_water_temp = {}", state[37]).unwrap();
    writeln!(w).unwrap();

    writeln!(w, "[state.valves]").unwrap();
    for (i, name) in valve_names.iter().enumerate() {
        writeln!(w, "{:<25} = {}", name, state[38 + i]).unwrap();
    }

    w.flush().unwrap();
    eprintln!("Snapshot written → {}", path);
}
