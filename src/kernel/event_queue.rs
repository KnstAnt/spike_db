use std::collections::VecDeque;

pub struct SpikeEvent {
    pub neuron_id: u64,
    pub target_tick: u64,
}

pub struct KernelEventQueue {
    pub queue: VecDeque<SpikeEvent>,
}

impl KernelEventQueue {
    pub fn new() -> Self {
        Self { queue: VecDeque::new() }
    }

    pub fn push(&mut self, neuron_id: u64, target_tick: u64) {
        self.queue.push_back(SpikeEvent { neuron_id, target_tick });
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn extract_current_spikes(&mut self, current_tick: u64) -> Vec<u64> {
        let mut current_spikes = Vec::new();
        while let Some(pos) = self.queue.iter().position(|e| e.target_tick <= current_tick) {
            if let Some(event) = self.queue.remove(pos) {
                current_spikes.push(event.neuron_id);
            }
        }
        current_spikes
    }
}