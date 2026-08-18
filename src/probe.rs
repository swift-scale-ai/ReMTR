use crate::args::Config;
use crate::model::ProbeResult;
use crate::platform::darwin::{IcmpSocket, ReplyKind};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum ProbeEvent {
    Result(ProbeResult),
    Error(String),
    Stopped,
}

pub struct ProbeControl {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl ProbeControl {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn toggle_pause(&self) -> bool {
        let paused = !self.paused.load(Ordering::Relaxed);
        self.paused.store(paused, Ordering::Relaxed);
        paused
    }
}

pub fn resolve_ipv4(target: &str) -> io::Result<Ipv4Addr> {
    (target, 0)
        .to_socket_addrs()?
        .find_map(|address| match address {
            SocketAddr::V4(address) => Some(*address.ip()),
            SocketAddr::V6(_) => None,
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "target has no IPv4 address",
            )
        })
}

pub fn spawn(config: Config, target: Ipv4Addr) -> (ProbeControl, Receiver<ProbeEvent>) {
    let (sender, receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let thread_paused = Arc::clone(&paused);
    thread::spawn(move || run_worker(config, target, sender, thread_stop, thread_paused));
    (ProbeControl { stop, paused }, receiver)
}

fn run_worker(
    config: Config,
    target: Ipv4Addr,
    sender: Sender<ProbeEvent>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
) {
    let socket = match IcmpSocket::open() {
        Ok(socket) => socket,
        Err(error) => {
            let message = if error.kind() == io::ErrorKind::PermissionDenied {
                "raw ICMP socket permission denied; run ReMTR with sudo".to_owned()
            } else {
                format!("unable to open raw ICMP socket: {error}")
            };
            let _ = sender.send(ProbeEvent::Error(message));
            return;
        }
    };
    let identifier = std::process::id() as u16;
    let mut sequence = 1_u16;

    while !stop.load(Ordering::Relaxed) {
        for ttl in 1..=config.max_hops {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            while paused.load(Ordering::Relaxed) && !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(50));
            }
            let reply = match socket.probe(target, ttl, identifier, sequence, config.timeout) {
                Ok(reply) => reply,
                Err(error) => {
                    let _ = sender.send(ProbeEvent::Error(format!(
                        "probe failed at hop {ttl}: {error}"
                    )));
                    return;
                }
            };
            sequence = sequence.wrapping_add(1).max(1);
            let result = ProbeResult {
                ttl,
                address: reply.map(|reply| reply.address),
                rtt: reply.map(|reply| reply.elapsed),
                reached: reply.is_some_and(|reply| reply.kind == ReplyKind::EchoReply),
            };
            let reached = result.reached;
            if sender.send(ProbeEvent::Result(result)).is_err() {
                return;
            }
            if reached {
                break;
            }
            thread::sleep(config.interval);
        }
    }
    let _ = sender.send(ProbeEvent::Stopped);
}
