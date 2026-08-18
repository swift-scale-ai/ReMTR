use std::collections::VecDeque;
use std::net::Ipv4Addr;
use std::time::Duration;

const HISTORY_LENGTH: usize = 80;

#[derive(Clone, Debug)]
pub struct ProbeResult {
    pub ttl: u8,
    pub address: Option<Ipv4Addr>,
    pub rtt: Option<Duration>,
    pub reached: bool,
}

#[derive(Clone, Debug)]
pub struct HopStats {
    pub ttl: u8,
    pub address: Option<Ipv4Addr>,
    pub alternate_addresses: Vec<Ipv4Addr>,
    pub sent: u64,
    pub received: u64,
    pub last_ms: Option<f64>,
    pub best_ms: Option<f64>,
    pub worst_ms: Option<f64>,
    pub average_ms: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub history: VecDeque<Option<f64>>,
    sum_ms: f64,
}

impl HopStats {
    fn new(ttl: u8) -> Self {
        Self {
            ttl,
            address: None,
            alternate_addresses: Vec::new(),
            sent: 0,
            received: 0,
            last_ms: None,
            best_ms: None,
            worst_ms: None,
            average_ms: None,
            jitter_ms: None,
            history: VecDeque::with_capacity(HISTORY_LENGTH),
            sum_ms: 0.0,
        }
    }

    pub fn loss_percent(&self) -> f64 {
        if self.sent == 0 {
            0.0
        } else {
            (self.sent - self.received) as f64 * 100.0 / self.sent as f64
        }
    }

    fn record(&mut self, result: &ProbeResult) {
        self.sent += 1;
        if let Some(address) = result.address {
            if self.address != Some(address) {
                if let Some(previous) = self.address.replace(address) {
                    if !self.alternate_addresses.contains(&previous) {
                        self.alternate_addresses.push(previous);
                    }
                }
            }
        }

        let milliseconds = result.rtt.map(|duration| duration.as_secs_f64() * 1000.0);
        if let Some(milliseconds) = milliseconds {
            self.received += 1;
            self.sum_ms += milliseconds;
            self.best_ms = Some(
                self.best_ms
                    .map_or(milliseconds, |value| value.min(milliseconds)),
            );
            self.worst_ms = Some(
                self.worst_ms
                    .map_or(milliseconds, |value| value.max(milliseconds)),
            );
            self.average_ms = Some(self.sum_ms / self.received as f64);
            self.jitter_ms = self.last_ms.map(|last| (milliseconds - last).abs());
            self.last_ms = Some(milliseconds);
        }
        if self.history.len() == HISTORY_LENGTH {
            self.history.pop_front();
        }
        self.history.push_back(milliseconds);
    }
}

#[derive(Clone, Debug)]
pub struct PathModel {
    pub target: String,
    pub target_address: Ipv4Addr,
    pub hops: Vec<HopStats>,
    pub cycles: u64,
    pub reached_hop: Option<u8>,
}

impl PathModel {
    pub fn new(target: String, target_address: Ipv4Addr, max_hops: u8) -> Self {
        Self {
            target,
            target_address,
            hops: (1..=max_hops).map(HopStats::new).collect(),
            cycles: 0,
            reached_hop: None,
        }
    }

    pub fn record(&mut self, result: ProbeResult) {
        if let Some(hop) = self.hops.get_mut(result.ttl.saturating_sub(1) as usize) {
            hop.record(&result);
        }
        if result.reached {
            self.reached_hop = Some(result.ttl);
            self.cycles += 1;
        } else if result.ttl == self.hops.len() as u8 {
            self.cycles += 1;
        }
    }

    pub fn visible_hops(&self) -> &[HopStats] {
        let length = self
            .reached_hop
            .map_or(self.hops.len(), |hop| hop as usize)
            .max(1);
        &self.hops[..length]
    }

    pub fn reset(&mut self) {
        let max_hops = self.hops.len() as u8;
        *self = Self::new(self.target.clone(), self.target_address, max_hops);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_loss_and_latency() {
        let mut hop = HopStats::new(1);
        hop.record(&ProbeResult {
            ttl: 1,
            address: Some(Ipv4Addr::LOCALHOST),
            rtt: Some(Duration::from_millis(10)),
            reached: false,
        });
        hop.record(&ProbeResult {
            ttl: 1,
            address: None,
            rtt: None,
            reached: false,
        });
        assert_eq!(hop.loss_percent(), 50.0);
        assert_eq!(hop.average_ms, Some(10.0));
    }

    #[test]
    fn tracks_route_changes() {
        let mut hop = HopStats::new(2);
        for address in [Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 2)] {
            hop.record(&ProbeResult {
                ttl: 2,
                address: Some(address),
                rtt: Some(Duration::from_millis(1)),
                reached: false,
            });
        }
        assert_eq!(hop.address, Some(Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(hop.alternate_addresses, vec![Ipv4Addr::new(10, 0, 0, 1)]);
    }
}
