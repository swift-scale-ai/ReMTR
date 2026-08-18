#![deny(unsafe_op_in_unsafe_fn)]

mod app;
mod args;
mod model;
mod platform;
mod probe;
mod ui;

use args::Config;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

fn main() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        eprintln!("ReMTR currently supports native Apple Silicon macOS only.");
        std::process::exit(2);
    }

    let config = match Config::parse(std::env::args()) {
        Ok(config) => config,
        Err(message) => {
            let success = message.starts_with("ReMTR") || message.starts_with("remtr ");
            if success {
                println!("{message}");
            } else {
                eprintln!("{message}");
            }
            std::process::exit(if success { 0 } else { 2 });
        }
    };
    let address = match probe::resolve_ipv4(&config.target) {
        Ok(address) => address,
        Err(error) => {
            eprintln!("unable to resolve {}: {error}", config.target);
            std::process::exit(1);
        }
    };

    let result = if config.snapshot {
        print_snapshot(&config, address)
    } else {
        run_tui(config, address)
    };
    if let Err(error) = result {
        eprintln!("remtr: {error}");
        std::process::exit(1);
    }
}

fn print_snapshot(config: &Config, address: std::net::Ipv4Addr) -> io::Result<()> {
    let model = app::collect_snapshot(config, address)?;
    println!(
        "ReMTR {} ({}) · cycles {}",
        model.target, model.target_address, model.cycles
    );
    println!("Hop  Host              Loss%  Sent    Last     Avg    Best   Worst  Jitter");
    for hop in model.visible_hops() {
        println!(
            "{:>3}  {:<16} {:>5.1} {:>5} {:>7} {:>7} {:>7} {:>7} {:>7}",
            hop.ttl,
            hop.address
                .map(|address| address.to_string())
                .unwrap_or_else(|| "*".to_owned()),
            hop.loss_percent(),
            hop.sent,
            metric(hop.last_ms),
            metric(hop.average_ms),
            metric(hop.best_ms),
            metric(hop.worst_ms),
            metric(hop.jitter_ms),
        );
    }
    Ok(())
}

fn metric(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}"))
}

fn run_tui(config: Config, address: std::net::Ipv4Addr) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let result = run_loop(&mut terminal, config, address);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
    address: std::net::Ipv4Addr,
) -> io::Result<()> {
    let mut app = app::App::new(config, address);
    loop {
        app.refresh();
        terminal.draw(|frame| ui::draw(frame, &app))?;
        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        key => app.on_key(key),
                    }
                }
            }
        }
    }
    Ok(())
}
