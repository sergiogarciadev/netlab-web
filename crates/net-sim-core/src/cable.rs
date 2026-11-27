use std::cmp::Ordering;
use std::collections::BinaryHeap;

use serde::{Deserialize, Serialize};

/// Deterministic xorshift64* PRNG for reproducible jitter and packet loss in WASM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Returns a float in [0.0, 1.0).
    pub fn next_f64(&mut self) -> f64 {
        let val = self.next_u64() >> 11;
        (val as f64) / ((1u64 << 53) as f64)
    }

    /// Returns an integer in [0, bound).
    pub fn next_range(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        self.next_u64() % bound
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Endpoint {
    pub node_id: String,
    pub port_id: String,
}

impl Endpoint {
    pub fn new(node_id: impl Into<String>, port_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            port_id: port_id.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CableDirection {
    AToB,
    BToA,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CableConfig {
    pub latency_ms: u64,
    pub jitter_ms: u64,
    /// Packet loss percentage in [0.0, 100.0].
    pub loss_percent: f64,
    pub up: bool,
}

impl Default for CableConfig {
    fn default() -> Self {
        Self {
            latency_ms: 5,
            jitter_ms: 0,
            loss_percent: 0.0,
            up: true,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct InFlightFrame {
    deliver_at_ms: u64,
    seq: u64,
    direction: CableDirection,
    frame: Vec<u8>,
}

impl Ord for InFlightFrame {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior
        other
            .deliver_at_ms
            .cmp(&self.deliver_at_ms)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

impl PartialOrd for InFlightFrame {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Point-to-point bidirectional virtual Ethernet cable with discrete-event delivery.
#[derive(Debug, Clone)]
pub struct VirtualCable {
    pub id: String,
    pub end_a: Endpoint,
    pub end_b: Endpoint,
    pub config: CableConfig,
    queue: BinaryHeap<InFlightFrame>,
    next_seq: u64,
    rng: DeterministicRng,
    pub packets_sent: u64,
    pub packets_dropped: u64,
}

impl VirtualCable {
    pub fn new(id: impl Into<String>, end_a: Endpoint, end_b: Endpoint, seed: u64) -> Self {
        Self {
            id: id.into(),
            end_a,
            end_b,
            config: CableConfig::default(),
            queue: BinaryHeap::new(),
            next_seq: 0,
            rng: DeterministicRng::new(seed),
            packets_sent: 0,
            packets_dropped: 0,
        }
    }

    /// Enqueue a frame from `from_endpoint` toward the other end.
    /// Returns `Some((direction, deliver_at_ms))` if accepted, or `None` if dropped/link down.
    pub fn transmit(
        &mut self,
        from_node_id: &str,
        from_port_id: &str,
        frame: Vec<u8>,
        now_ms: u64,
    ) -> Option<(CableDirection, u64)> {
        let direction = if self.end_a.node_id == from_node_id && self.end_a.port_id == from_port_id
        {
            CableDirection::AToB
        } else if self.end_b.node_id == from_node_id && self.end_b.port_id == from_port_id {
            CableDirection::BToA
        } else {
            return None;
        };

        if !self.config.up {
            self.packets_dropped += 1;
            return None;
        }

        if self.config.loss_percent > 0.0 {
            let roll = self.rng.next_f64() * 100.0;
            if roll < self.config.loss_percent {
                self.packets_dropped += 1;
                return None;
            }
        }

        let jitter = if self.config.jitter_ms > 0 {
            self.rng.next_range(self.config.jitter_ms + 1)
        } else {
            0
        };
        let deliver_at_ms = now_ms + self.config.latency_ms + jitter;
        let seq = self.next_seq;
        self.next_seq += 1;
        self.packets_sent += 1;

        self.queue.push(InFlightFrame {
            deliver_at_ms,
            seq,
            direction,
            frame,
        });

        Some((direction, deliver_at_ms))
    }

    /// Pop all frames whose delivery timestamp <= `now_ms`.
    pub fn pop_due(&mut self, now_ms: u64) -> Vec<(Endpoint, Vec<u8>)> {
        let mut delivered = Vec::new();
        while let Some(top) = self.queue.peek() {
            if top.deliver_at_ms > now_ms {
                break;
            }
            let item = self.queue.pop().unwrap();
            let target = match item.direction {
                CableDirection::AToB => self.end_b.clone(),
                CableDirection::BToA => self.end_a.clone(),
            };
            delivered.push((target, item.frame));
        }
        delivered
    }

    pub fn in_flight_count(&self) -> usize {
        self.queue.len()
    }

    pub fn next_delivery_ms(&self) -> Option<u64> {
        self.queue.peek().map(|f| f.deliver_at_ms)
    }
}
