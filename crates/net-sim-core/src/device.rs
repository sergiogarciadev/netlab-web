use std::collections::VecDeque;

use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;

/// A virtual Ethernet NIC backed by in-memory RX/TX frame queues.
#[derive(Debug, Clone)]
pub struct VirtualDevice {
    pub rx_queue: VecDeque<Vec<u8>>,
    pub tx_queue: VecDeque<Vec<u8>>,
    pub max_transmission_unit: usize,
}

impl Default for VirtualDevice {
    fn default() -> Self {
        Self::new(1514)
    }
}

impl VirtualDevice {
    pub fn new(max_transmission_unit: usize) -> Self {
        Self {
            rx_queue: VecDeque::new(),
            tx_queue: VecDeque::new(),
            max_transmission_unit,
        }
    }

    /// Enqueue an incoming Ethernet frame to be processed by the host stack.
    pub fn push_rx(&mut self, frame: Vec<u8>) {
        self.rx_queue.push_back(frame);
    }

    /// Pop an outgoing Ethernet frame transmitted by the host stack.
    pub fn pop_tx(&mut self) -> Option<Vec<u8>> {
        self.tx_queue.pop_front()
    }
}

pub struct VirtualRxToken {
    frame: Vec<u8>,
}

impl RxToken for VirtualRxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        f(&self.frame)
    }
}

pub struct VirtualTxToken<'a> {
    tx_queue: &'a mut VecDeque<Vec<u8>>,
}

impl<'a> TxToken for VirtualTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = vec![0u8; len];
        let result = f(&mut buffer);
        self.tx_queue.push_back(buffer);
        result
    }
}

impl Device for VirtualDevice {
    type RxToken<'a> = VirtualRxToken where Self: 'a;
    type TxToken<'a> = VirtualTxToken<'a> where Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let frame = self.rx_queue.pop_front()?;
        let rx = VirtualRxToken { frame };
        let tx = VirtualTxToken {
            tx_queue: &mut self.tx_queue,
        };
        Some((rx, tx))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(VirtualTxToken {
            tx_queue: &mut self.tx_queue,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = self.max_transmission_unit;
        caps
    }
}
