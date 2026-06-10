use serde::{Serialize, Deserialize};
use bincode_next::{Encode, Decode};
use crate::config::CONFIG; // Импортируем наш глобальный конфиг

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq)]
pub enum NeuronType {
    Sensor,
    Motor,
    Hidden,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone)]
pub struct NeuronState {
    pub potential: f32,
    pub last_updated_tick: u64,
    pub cooldown_until: u64,
    pub neuron_type: NeuronType,
}


impl Synapse {
    pub fn trigger(&mut self, current_tick: u64) {
        self.decay_tag_lazy(current_tick);
        self.tag_trace = (self.tag_trace + 0.5).min(1.0);
    }

    pub fn decay_tag_lazy(&mut self, current_tick: u64) {
        let cfg = CONFIG.get().expect("Конфигурация SpikeDB не инициализирована");
        
        if current_tick > self.last_used_tick {
            let delta_t = (current_tick - self.last_used_tick) as f32;
            self.tag_trace *= (-delta_t / cfg.tag_tau).exp();
            self.last_used_tick = current_tick;
        }
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone)]
pub struct Synapse {
    pub weight: f32,
    pub tag_trace: f32,
    pub last_used_tick: u64,
}

impl NeuronState {
    pub fn new(neuron_type: NeuronType) -> Self {
        Self {
            potential: 0.0,
            last_updated_tick: 0,
            cooldown_until: 0,
            neuron_type,
        }
    }

    /// Принимает импульс, используя глобальный CONFIG
    pub fn receive_impulse(&mut self, incoming_charge: f32, current_tick: u64) -> bool {
        if current_tick < self.cooldown_until {
            return false;
        }

        let cfg = CONFIG.get().expect("Конфигурация SpikeDB не инициализирована");

        if current_tick > self.last_updated_tick {
            let delta_t = (current_tick - self.last_updated_tick) as f32;
            self.potential *= (-delta_t / cfg.leak_tau).exp();
            self.last_updated_tick = current_tick;
        }

        self.potential += incoming_charge;

        if self.potential < 0.0 {
            self.potential = 0.0;
        }

        if self.potential >= cfg.spike_threshold {
            self.potential = 0.0;
            self.cooldown_until = current_tick + cfg.cooldown_ticks;
            true
        } else {
            false
        }
    }
}

