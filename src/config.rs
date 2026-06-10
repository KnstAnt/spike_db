use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

pub static CONFIG: OnceLock<BrainConfig> = OnceLock::new();

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrainConfig {
    pub leak_tau: f32,
    pub spike_threshold: f32,
    pub cooldown_ticks: u64,
    pub tag_tau: f32,
    pub coincidence_threshold: u32,
    pub learning_rate: f32,
    pub punish_rate: f32,

    // НОВЫЕ ПАРАМЕТРЫ ДЛЯ КОНТРАСТНОГО СНА
    pub sleep_death_threshold: f32,   // Порог смерти синапса (было 0.2)
    pub sleep_strong_decay: f32,     // Угасание сильных связей (было 0.98)
    pub sleep_medium_decay: f32,     // Угасание средних связей (было 0.85)
    pub sleep_weak_shredder: f32,    // Жесткий штраф вычитания для шума (было 0.25)
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            leak_tau: 20.0,
            spike_threshold: 1.0,
            cooldown_ticks: 5,
            tag_tau: 15.0,
            coincidence_threshold: 3,
            learning_rate: 0.3,
            punish_rate: 0.2,
            
            // Дефолтные настройки ночного гомеостаза
            sleep_death_threshold: 0.2,
            sleep_strong_decay: 0.98,
            sleep_medium_decay: 0.85,
            sleep_weak_shredder: 0.25,
        }
    }
}

impl BrainConfig {
    pub fn load_from_file() -> Self {
        let path = "brain_config.toml";
        if Path::new(path).exists() {
            let content = fs::read_to_string(path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_else(|_| BrainConfig::default())
        } else {
            let default_config = BrainConfig::default();
            let toml_string = toml::to_string_pretty(&default_config).unwrap_or_default();
            let _ = fs::write(path, toml_string);
            default_config
        }
    }
}
