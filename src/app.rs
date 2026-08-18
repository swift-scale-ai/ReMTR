use crate::args::Config;
use crate::model::{HopStats, PathModel};
use crate::probe::{self, ProbeControl, ProbeEvent};
use crossterm::event::KeyCode;
use std::io;
use std::net::Ipv4Addr;
use std::sync::mpsc::Receiver;

pub struct App {
    pub model: PathModel,
    pub selected: usize,
    pub paused: bool,
    pub error: Option<String>,
    control: ProbeControl,
    events: Receiver<ProbeEvent>,
}

impl App {
    pub fn new(config: Config, address: Ipv4Addr) -> Self {
        let model = PathModel::new(config.target.clone(), address, config.max_hops);
        let (control, events) = probe::spawn(config.clone(), address);
        Self {
            model,
            selected: 0,
            paused: false,
            error: None,
            control,
            events,
        }
    }

    pub fn refresh(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                ProbeEvent::Result(result) => {
                    self.model.record(result);
                    self.error = None;
                }
                ProbeEvent::Error(error) => self.error = Some(error),
                ProbeEvent::Stopped => break,
            }
        }
        self.selected = self
            .selected
            .min(self.model.visible_hops().len().saturating_sub(1));
    }

    pub fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected =
                    (self.selected + 1).min(self.model.visible_hops().len().saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = self.model.visible_hops().len().saturating_sub(1),
            KeyCode::Char('p') | KeyCode::Char(' ') => {
                self.paused = self.control.toggle_pause();
            }
            KeyCode::Char('r') => {
                self.model.reset();
                self.selected = 0;
            }
            _ => {}
        }
    }

    pub fn selected_hop(&self) -> Option<&HopStats> {
        self.model.visible_hops().get(self.selected)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.control.stop();
    }
}

pub fn collect_snapshot(config: &Config, address: Ipv4Addr) -> io::Result<PathModel> {
    let mut model = PathModel::new(config.target.clone(), address, config.max_hops);
    let (control, events) = probe::spawn(config.clone(), address);
    while model.cycles < u64::from(config.cycles) {
        match events.recv() {
            Ok(ProbeEvent::Result(result)) => model.record(result),
            Ok(ProbeEvent::Error(error)) => {
                control.stop();
                return Err(io::Error::other(error));
            }
            Ok(ProbeEvent::Stopped) | Err(_) => break,
        }
    }
    control.stop();
    Ok(model)
}
