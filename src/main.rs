pub mod simulation;

use simulation::*;

#[allow(dead_code)]
mod util;

use std::io;

use kernel_density::ecdf::Ecdf;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, Marker, Widget};
use tui::Terminal;

use crate::util::event::{Event, Events};

struct App {
    sys_addr: SystemAddrs,
    density: Vec<(f64, f64)>,
    window: [f64; 2],
}

impl App {
    fn new(sys_addr: SystemAddrs) -> App {
        let n_nodes = sys_addr.node_addrs.len();

        let density = (0..32).map(move |i: u32| (i as f64, 0.0)).collect();
        App {
            sys_addr,
            density,
            window: [0.0, n_nodes as f64],
        }
    }

    async fn update(&mut self) {
        if let Ok(distances) = self.sys_addr.get_all_distances().await {
            let ecdf = Ecdf::new(&distances);
            self.window = [0.0, ecdf.max() as f64];
            self.density = (ecdf.min()..ecdf.max())
                .map(move |i| (i as f64, ecdf.value(i as u32) as f64))
                .collect();
        }
    }
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start nodes and collect addresses
    let n_nodes: usize = 300;
    let heartbeat_ms = 1_000;

    let nodes: Vec<_> = (0..n_nodes)
        .map(|_| {
            let sample_size = 16;
            Node::new(1, heartbeat_ms, 1, sample_size)
        })
        .collect();

    let wallet_broadcast_interval = 1;
    let sys_addrs = start_simulation(nodes, 10, wallet_broadcast_interval);

    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Setup event handlers
    let events = Events::new();

    // App
    let mut app = App::new(sys_addrs);

    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            Chart::default()
                .block(
                    Block::default()
                        .title("Chart")
                        .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD))
                        .borders(Borders::ALL),
                )
                .x_axis(
                    Axis::default()
                        .title("Distance")
                        .style(Style::default().fg(Color::Gray))
                        .labels_style(Style::default().modifier(Modifier::ITALIC))
                        .bounds(app.window)
                        .labels(&[
                            &format!("{}", app.window[0]),
                            &format!("{}", (app.window[0] + app.window[1]) / 2.0),
                            &format!("{}", app.window[1]),
                        ]),
                )
                .y_axis(
                    Axis::default()
                        .title("Density")
                        .style(Style::default().fg(Color::Gray))
                        .labels_style(Style::default().modifier(Modifier::ITALIC))
                        .bounds([0., 1.0])
                        .labels(&["0", "1"]),
                )
                .datasets(&[Dataset::default()
                    .marker(Marker::Dot)
                    .style(Style::default().fg(Color::Cyan))
                    .data(&app.density)])
                .render(&mut f, size);
        })?;

        match events.next()? {
            Event::Input(input) => {
                if input == Key::Char('q') {
                    break;
                }
            }
            Event::Tick => {
                app.update().await;
            }
        }
    }

    Ok(())
}
