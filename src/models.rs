#[derive(Debug, Clone, PartialEq)]
pub enum NeuronType {
    Sensor,
    Motor,
    Hidden,
}

#[derive(Debug, Clone)]
pub struct NeuronState {
    pub potential: f32,
    pub last_updated_tick: u64,
    pub cooldown_until: u64,
    pub neuron_type: NeuronType,
}

#[derive(Debug, Clone)]
pub struct Synapse {
    pub target_id: u64,    // ID целевого нейрона, куда летит импульс
    pub weight: f32,       // Сила связи
    pub tag_trace: f32,    // Химический след активности (Tag)
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

    pub fn receive_impulse(&mut self, incoming_charge: f32, current_tick: u64, leak_tau: f32, spike_threshold: f32, cooldown_ticks: u64) -> bool {
        if current_tick < self.cooldown_until {
            return false;
        }

        if current_tick > self.last_updated_tick {
            let delta_t = (current_tick - self.last_updated_tick) as f32;
            self.potential *= (-delta_t / leak_tau).exp();
            self.last_updated_tick = current_tick;
        }

        self.potential += incoming_charge;

        if self.potential < 0.0 {
            self.potential = 0.0;
        }

        if self.potential >= spike_threshold {
            self.potential = 0.0;
            self.cooldown_until = current_tick + cooldown_ticks;
            true
        } else {
            false
        }
    }
}

impl Synapse {
    pub fn trigger(&mut self, current_tick: u64, tag_tau: f32) {
        self.decay_tag_lazy(current_tick, tag_tau);
        self.tag_trace = (self.tag_trace + 0.5).min(1.0);
    }

    pub fn decay_tag_lazy(&mut self, current_tick: u64, tag_tau: f32) {
        if current_tick > self.last_used_tick {
            let delta_t = (current_tick - self.last_used_tick) as f32;
            self.tag_trace *= (-delta_t / tag_tau).exp();
            self.last_used_tick = current_tick;
        }
    }

    pub fn calculate_resonance_score(&self) -> f32 {
        self.weight + self.tag_trace
    }
}
