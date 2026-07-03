// dashboard.rs

// TODO(refactor-core): `te_core::snapshot::SimulationSnapshot` foi removido durante a
// refatoração do core (issue #58 / #55). Todo o dashboard dependia dele e foi
// temporariamente comentado abaixo até a nova API do core (snapshot/alarms) ser
// reescrita. Isso está ERRADO/incompleto — o TUI não renderiza nada.
// use crossterm::{
//     event::{self, Event, KeyCode, KeyModifiers},
//     execute,
//     terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
// };
// use ratatui::{
//     backend::CrosstermBackend,
//     layout::{Constraint, Direction, Layout, Rect},
//     style::{Color, Modifier, Style},
//     text::{Line, Span},
//     widgets::{Block, Borders, Cell, Paragraph, Row, Table},
//     Frame, Terminal,
// };
// use std::io;
// use te_core::snapshot::SimulationSnapshot;

// TODO(refactor-core): stub temporário para manter o crate compilando enquanto o core
// é reescrito. Nenhum método real está implementado.
pub struct Dashboard;

/*
TODO(refactor-core): implementação original, preservada para referência.

const XMEAS_META: &[(usize, &str, &str, &str)] = &[
    (0,  "XMEAS(1)",  "A Feed",                "kscmh"),
    (1,  "XMEAS(2)",  "D Feed",                "kg/hr"),
    (2,  "XMEAS(3)",  "E Feed",                "kg/hr"),
    (3,  "XMEAS(4)",  "A&C Feed",              "kscmh"),
    (4,  "XMEAS(5)",  "Recycle Flow",           "kscmh"),
    (5,  "XMEAS(6)",  "Reactor Feed Rate",      "kscmh"),
    (6,  "XMEAS(7)",  "Reactor Pressure",       "kPa"),
    (7,  "XMEAS(8)",  "Reactor Level",          "%"),
    (8,  "XMEAS(9)",  "Reactor Temperature",    "\u{00b0}C"),
    (9,  "XMEAS(10)", "Purge Rate",             "kscmh"),
    (10, "XMEAS(11)", "Sep Temperature",        "\u{00b0}C"),
    (11, "XMEAS(12)", "Sep Level",              "%"),
    (12, "XMEAS(13)", "Sep Pressure",           "kPa"),
    (13, "XMEAS(14)", "Sep Underflow",          "m3/hr"),
    (14, "XMEAS(15)", "Stripper Level",         "%"),
    (15, "XMEAS(16)", "Stripper Pressure",      "kPa"),
    (16, "XMEAS(17)", "Stripper Underflow",     "m3/hr"),
    (17, "XMEAS(18)", "Stripper Temperature",   "\u{00b0}C"),
    (18, "XMEAS(19)", "Stripper Steam Flow",    "kg/hr"),
    (19, "XMEAS(20)", "Compressor Work",        "kW"),
    (20, "XMEAS(21)", "Reactor CW Outlet Temp", "\u{00b0}C"),
    (21, "XMEAS(22)", "Sep CW Outlet Temp",     "\u{00b0}C"),
];

const XMV_META: &[(usize, &str, &str, &str)] = &[
    (0,  "XMV(1)",  "D Feed Flow",             "%"),
    (1,  "XMV(2)",  "E Feed Flow",             "%"),
    (2,  "XMV(3)",  "A Feed Flow",             "%"),
    (3,  "XMV(4)",  "A&C Feed Flow",           "%"),
    (4,  "XMV(5)",  "Compressor Recycle",      "%"),
    (5,  "XMV(6)",  "Purge Valve",             "%"),
    (6,  "XMV(7)",  "Sep Pot Liquid Flow",     "%"),
    (7,  "XMV(8)",  "Stripper Liquid Product", "%"),
    (8,  "XMV(9)",  "Stripper Steam Valve",    "%"),
    (9,  "XMV(10)", "Reactor CW Flow",         "%"),
    (10, "XMV(11)", "Condenser CW Flow",       "%"),
    (11, "XMV(12)", "Agitator Speed",          "%"),
];

pub struct Dashboard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Dashboard {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        // Drain any bytes buffered in stdin before raw mode was enabled
        // (terminal initialization sequences sent by the shell/IDE on session start
        // can be misread as keypresses — e.g. causing an immediate 'q' quit).
        while event::poll(std::time::Duration::from_millis(0))? {
            let _ = event::read()?;
        }
        Ok(Self { terminal })
    }

    pub fn render(&mut self, snap: &SimulationSnapshot) -> io::Result<bool> {
        if event::poll(std::time::Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(false),
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        return Ok(false);
                    }
                    _ => {}
                }
            }
        }
        self.terminal.draw(|f| render_frame(f, snap))?;
        Ok(true)
    }
}

impl Drop for Dashboard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}

fn render_frame(f: &mut Frame, snap: &SimulationSnapshot) {
    let area = f.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // header
            Constraint::Length(6),   // solver + alarms
            Constraint::Min(10),     // XMEAS | XMV | IDV
            Constraint::Length(5),   // internal state YY
        ])
        .split(area);

    render_header(f, rows[0], snap);
    render_diagnostics(f, rows[1], snap);
    render_signals(f, rows[2], snap);
    render_state(f, rows[3], snap);
}

fn render_header(f: &mut Frame, area: Rect, snap: &SimulationSnapshot) {
    let any_alarm = snap.alarms.iter().any(|a| a.active);
    let (status_label, status_color) = if any_alarm {
        ("ALARM", Color::Red)
    } else {
        ("OK", Color::Green)
    };
    let title = format!(
        " Tennessee Eastman Process  \u{b7}  t = {:.2} h  \u{b7}  [{status_label}]  \u{b7}  [q] quit ",
        snap.time
    );
    f.render_widget(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(status_color)),
        area,
    );
}

fn render_diagnostics(f: &mut Frame, area: Rect, snap: &SimulationSnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let s = &snap.solver;
    let (stability, stab_color) = if s.deriv_norm < 1e-6 {
        ("Steady-state",   Color::Green)
    } else if s.deriv_norm < 1.0 {
        ("Slow transient", Color::Yellow)
    } else {
        ("Fast transient", Color::Red)
    };

    let solver_text = vec![
        Line::from(vec![
            Span::styled("Algorithm : ", Style::default().fg(Color::DarkGray)),
            Span::styled(s.algorithm, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Step size : ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.3} s", s.dt)),
        ]),
        Line::from(vec![
            Span::styled("States    : ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", s.n_states)),
        ]),
        Line::from(vec![
            Span::styled("|dy/dt|   : ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.2e}", s.deriv_norm)),
        ]),
        Line::from(vec![
            Span::styled("Status    : ", Style::default().fg(Color::DarkGray)),
            Span::styled(stability, Style::default().fg(stab_color)),
        ]),
    ];
    f.render_widget(
        Paragraph::new(solver_text)
            .block(Block::default().title(" Solver ").borders(Borders::ALL)),
        cols[0],
    );

    let alarm_block = Block::default().title(" Alarms ").borders(Borders::ALL);
    let inner = alarm_block.inner(cols[1]);
    f.render_widget(alarm_block, cols[1]);

    let alarm_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let alarms = &snap.alarms;
    let mid = (alarms.len() + 1) / 2;

    let make_lines = |slice: &[te_core::snapshot::Alarm]| -> Vec<Line<'static>> {
        slice.iter().map(|a| {
            let name = a.name.to_string();
            let (icon, color) = if a.active {
                ("\u{25cf} ALARM  ", Color::Red)
            } else {
                ("\u{25cb} ok     ", Color::Green)
            };
            Line::from(vec![
                Span::styled(icon, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {}", name), Style::default().fg(Color::Gray)),
            ])
        }).collect()
    };

    f.render_widget(Paragraph::new(make_lines(&alarms[..mid.min(alarms.len())])),  alarm_cols[0]);
    f.render_widget(Paragraph::new(make_lines(&alarms[mid.min(alarms.len())..])), alarm_cols[1]);
}

fn render_signals(f: &mut Frame, area: Rect, snap: &SimulationSnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(30),
            Constraint::Percentage(25),
        ])
        .split(area);

    let hdr = Style::default().add_modifier(Modifier::BOLD).fg(Color::White);

    // XMEAS
    let xmeas_rows: Vec<Row> = XMEAS_META.iter().map(|(idx, tag, name, unit)| {
        let val = snap.xmeas.get(*idx).copied().unwrap_or(f64::NAN);
        Row::new(vec![
            Cell::from(*tag).style(Style::default().fg(Color::Yellow)),
            Cell::from(format!("{:>10.3}", val)),
            Cell::from(*unit).style(Style::default().fg(Color::DarkGray)),
            Cell::from(*name).style(Style::default().fg(Color::Gray)),
        ])
    }).collect();

    f.render_widget(
        Table::new(xmeas_rows, [
            Constraint::Length(10), Constraint::Length(11),
            Constraint::Length(7),  Constraint::Fill(1),
        ])
        .header(Row::new(["Tag", "Value", "Unit", "Name"]).style(hdr))
        .block(Block::default().title(" Measurements (XMEAS) ").borders(Borders::ALL)),
        cols[0],
    );

    // XMV
    let xmv_rows: Vec<Row> = XMV_META.iter().map(|(idx, tag, name, unit)| {
        let val = snap.xmv.get(*idx).copied().unwrap_or(f64::NAN);
        Row::new(vec![
            Cell::from(*tag).style(Style::default().fg(Color::Green)),
            Cell::from(format!("{:>7.2}", val)),
            Cell::from(*unit).style(Style::default().fg(Color::DarkGray)),
            Cell::from(*name).style(Style::default().fg(Color::Gray)),
        ])
    }).collect();

    f.render_widget(
        Table::new(xmv_rows, [
            Constraint::Length(8), Constraint::Length(8),
            Constraint::Length(3), Constraint::Fill(1),
        ])
        .header(Row::new(["Tag", "Value", "Unit", "Name"]).style(hdr))
        .block(Block::default().title(" Manipulated (XMV) ").borders(Borders::ALL)),
        cols[1],
    );

    // IDV
    let idv_rows: Vec<Row> = snap.dv.iter().enumerate().map(|(i, &val)| {
        let active = val != 0.0;
        let (label, color) = if active { ("ON ", Color::Red) } else { ("off", Color::DarkGray) };
        Row::new(vec![
            Cell::from(format!("IDV({:2})", i + 1)).style(Style::default().fg(Color::Magenta)),
            Cell::from(format!("{:>6.2}", val)),
            Cell::from(label).style(Style::default().fg(color)),
        ])
    }).collect();

    f.render_widget(
        Table::new(idv_rows, [
            Constraint::Length(8), Constraint::Length(7), Constraint::Length(4),
        ])
        .header(Row::new(["Tag", "Value", "St."]).style(hdr))
        .block(Block::default().title(" Disturbances (IDV) ").borders(Borders::ALL)),
        cols[2],
    );
}

fn render_state(f: &mut Frame, area: Rect, snap: &SimulationSnapshot) {
    let per_row = ((area.width.saturating_sub(2)) / 14).max(1) as usize;

    let lines: Vec<Line> = snap.state
        .chunks(per_row)
        .enumerate()
        .map(|(row_idx, chunk)| {
            let mut spans = vec![];
            for (col, val) in chunk.iter().enumerate() {
                let i = row_idx * per_row + col;
                spans.push(Span::styled(
                    format!("[{:2}]", i),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::raw(format!("{:>9.3} ", val)));
            }
            Line::from(spans)
        })
        .collect();

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title(" Internal State (YY) ").borders(Borders::ALL)),
        area,
    );
}
*/
