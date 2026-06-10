use serde::{Serialize, Deserialize};
use std::fs;
use std::sync::OnceLock;

pub static CONFIG: OnceLock<BrainConfig> = OnceLock::new();

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrainConfig {
    pub leak_tau: f32,
    pub tag_tau: f32,
    pub spike_threshold: f32,
    pub learning_rate: f32,
    pub punish_rate: f32,
    pub cooldown_ticks: u64,
    pub coincidence_threshold: u32,
    pub sleep_strong_decay: f32,
    pub sleep_medium_decay: f32,
    pub sleep_weak_shredder: f32,
    pub sleep_death_threshold: f32,
    // НОВЫЕ СИСТЕМНЫЕ КОНФИГУРАЦИОННЫЕ ПОЛЯ:
    pub max_synapse_weight: f32,
    pub max_synaptic_fanout: usize,
    pub base_hebbian_weight: f32,
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            leak_tau: 10.0,
            tag_tau: 15.0,
            spike_threshold: 1.0,
            learning_rate: 0.4,
            punish_rate: 0.5,
            cooldown_ticks: 15,
            coincidence_threshold: 8,
            sleep_strong_decay: 0.80,
            sleep_medium_decay: 0.85,
            sleep_weak_shredder: 0.25,
            sleep_death_threshold: 0.20,
            max_synapse_weight: 3.0,
            max_synaptic_fanout: 8,
            base_hebbian_weight: 0.5,
        }
    }
}

impl BrainConfig {
    pub fn load_from_file() -> Self {
        let path = "brain_config.toml";
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(cfg) = toml::from_str(&content) {
                return cfg;
            }
        }
        let default_cfg = Self::default();
        let _ = fs::write(path, toml::to_string_pretty(&default_cfg).unwrap());
        default_cfg
    }
}
