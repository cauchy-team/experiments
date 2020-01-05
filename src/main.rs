pub mod simulation;

use simulation::*;

#[allow(dead_code)]
mod util;

use std::io;

use criterion_stats::univariate::{kde::{Kde, Bandwidth, kernel::Gaussian}, Sample};
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, Marker, Widget};
use tui::Terminal;
use rayon::prelude::*;

use crate::util::event::{Event, Events};

struct App {
    sys_addr: SystemAddrs,
    density: Vec<(f64, f64)>,
    window: [f64; 2],
    range: [f64; 2],
    resolution: u32
}

impl App {
    fn new(sys_addr: SystemAddrs, x_range: u32, resolution: u32) -> App {
        let density = (0..32).map(move |i: u32| (i as f64, 0.0)).collect();
        App {
            sys_addr,
            density,
            window: [0.0, x_range as f64],
            range: [0.0, 1.],
            resolution
        }
    }

    async fn update(&mut self) {
        if let Ok(distances) = self.sys_addr.get_all_distances().await {
            let max = *distances.iter().max().unwrap();
            let min = *distances.iter().min().unwrap();

            let float_distances: Vec<f64> = distances.into_iter().map(|i| i as f64).collect();
            let sample = Sample::new(&float_distances);
            let bandwidth = Bandwidth::Silverman;
            let kde = Kde::new(sample, Gaussian, bandwidth);

            let resolution = self.resolution;
            self.density = (0..(max - min) * resolution).into_par_iter()
                .map(move |i| {
                    let point = min as f64 + i as f64 / (resolution.clone() as f64);
                    (point, kde.estimate(point))
                })
                .collect();
            self.range = [0., 0.1]
        }
    }
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Network settings
    let n_nodes: usize = 256;
    let heartbeat_ms = 1_000;
    let sample_size = 3;

    // Wallet settings
    let wallet_broadcast_ms = 10;
    let wallet_fan_size = 32;

    // App settings
    let resolution = 10;
    let x_range = 2048;
    let refresh_rate = 300;

    // Network initialization
    let nodes: Vec<_> = (0..n_nodes)
        .map(|_| {
            Node::new(1, heartbeat_ms, 1, sample_size)
        })
        .collect();
    let sys_addrs = start_simulation(nodes, wallet_fan_size, wallet_broadcast_ms);

    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Setup event handlers
    let events = Events::new();
    let mut app = App::new(sys_addrs, x_range, resolution);

    let mut last_refresh = std::time::Instant::now();
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
                        .bounds(app.range)
                        .labels(&["0", ""]),
                )
                .datasets(&[Dataset::default()
                    .marker(Marker::Braille)
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
                let now = std::time::Instant::now();
                if now.duration_since(last_refresh).as_millis() > refresh_rate {
                    app.update().await;
                    last_refresh = now;
                }
            }
        }
    }

    Ok(())
}
