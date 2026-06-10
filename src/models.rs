#[derive(Debug, Clone, PartialEq)]
pub enum NeuronType {
    Sensor,
    Motor,
    Hidden,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NeuronOrigin {
    // Указывает на имя базового токена
    BaseToken(String),
    // Ссылка на базовые нейроны, на основе которых рождено новое мета-понятие
    ChunkSequence(u64, u64),
}

#[derive(Debug, Clone)]
pub struct NeuronState {
    pub potential: f32,
    pub last_updated_tick: u64,
    pub cooldown_until: u64,
    pub neuron_type: NeuronType,
    pub origin: NeuronOrigin,
}

#[derive(Debug, Clone)]
pub struct Synapse {
    pub target_id: u64,
    pub weight: f32,
    pub tag_trace: f32,
    pub last_used_tick: u64,
    pub cooldown_until: u64,
}

impl NeuronState {
    pub fn new(neuron_type: NeuronType, origin: NeuronOrigin) -> Self {
        Self {
            potential: 0.0,
            last_updated_tick: 0,
            cooldown_until: 0,
            neuron_type,
            origin,
        }
    }

    pub fn receive_impulse(
        &mut self,
        incoming_charge: f32,
        current_tick: u64,
        leak_tau: f32,
        spike_threshold: f32,
        cooldown_ticks: u64,
    ) -> bool {
        // ИСПРАВЛЕНИЕ: Если нейрон отдыхает в кулдауне, мы продвигаем его часы last_updated_tick,
        // но мембранный потенциал жестко удерживаем в нуле (активное торможение релаксации)!
        if current_tick < self.cooldown_until {
            self.potential = 0.0;
            self.last_updated_tick = current_tick;
            return false;
        }

        if current_tick > self.last_updated_tick {
            let delta_t = (current_tick - self.last_updated_tick) as f32;
            let leak_factor = (-delta_t / leak_tau).exp();
            self.potential *= leak_factor;
            self.last_updated_tick = current_tick;
        }

        self.potential += incoming_charge;

        if self.potential < 0.0 {
            self.potential = 0.0;
        }

        if self.potential >= spike_threshold {
            self.potential = 0.0;
            // Взводим глубокий рефрактерный период отдыха
            self.cooldown_until = current_tick + cooldown_ticks;
            true
        } else {
            false
        }
    }
}

impl Synapse {
    pub fn trigger(&mut self, current_tick: u64, tag_tau: f32) {
        if current_tick < self.cooldown_until {
            return;
        }
        self.decay_tag_lazy(current_tick, tag_tau);
        self.tag_trace = 1.0;
        self.last_used_tick = current_tick;
    }

    pub fn decay_tag_lazy(&mut self, current_tick: u64, tag_tau: f32) {
        if current_tick > self.last_used_tick {
            let delta_t = (current_tick - self.last_used_tick) as f32;
            let decay_factor = (-delta_t / tag_tau).exp();
            self.tag_trace *= decay_factor;
            self.last_used_tick = current_tick;
        }
    }

    pub fn calculate_resonance_score(&self) -> f32 {
        self.weight + self.tag_trace
    }
}
