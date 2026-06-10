use crate::config::{BrainConfig, CONFIG};
use crate::memory::SpikeMemory;
use std::collections::HashMap;

// ИСПРАВЛЕНИЕ: Импортируем компоненты изолированных подсистем через абсолютный корень crate::kernel
use crate::kernel::chunking::process_coincidences_kernel;
use crate::kernel::event_queue::KernelEventQueue;
pub use crate::kernel::event_queue::SpikeEvent;
use crate::kernel::evolution::{apply_reinforcement_kernel, sleep_and_prune_kernel};
use crate::kernel::generator::generate_trail_kernel;

pub struct SpikingNetwork {
    pub memory: SpikeMemory,
    pub current_tick: u64,
    pub event_queue: KernelEventQueue,
    pub previous_tick_spikes: Vec<u64>,
    pub coincidence_tracker: HashMap<(u64, u64), u32>,
    pub is_learning_mode: bool,
}

impl SpikingNetwork {
    pub fn new(config: BrainConfig) -> Self {
        let _ = CONFIG.set(config);
        Self {
            memory: SpikeMemory::new(),
            current_tick: 0,
            event_queue: KernelEventQueue::new(),
            previous_tick_spikes: Vec::new(),
            coincidence_tracker: HashMap::new(),
            is_learning_mode: false,
        }
    }

    pub fn inject_stimulus(&mut self, neuron_id: u64, charge: f32) {
        self.event_queue.push(neuron_id, self.current_tick);
        if self.process_impulse_to_neuron(neuron_id, charge) {
            self.previous_tick_spikes.push(neuron_id);
        }
    }

    pub fn set_learning_mode(&mut self, is_learning: bool) {
        self.is_learning_mode = is_learning;
    }

    pub fn process_impulse_to_neuron(&mut self, neuron_id: u64, charge: f32) -> bool {
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        if let Some(neuron) = self.memory.neurons.get_mut(neuron_id as usize) {
            return neuron.receive_impulse(
                charge,
                self.current_tick,
                cfg.leak_tau,
                cfg.spike_threshold,
                cfg.cooldown_ticks,
            );
        }
        false
    }

    /// Полностью очищает кратковременные буферы спайков, обнуляет tag_trace
    /// и принудительно сбрасывает мембранный потенциал всех нейронов в чистый ноль,
    /// гарантируя стопроцентную релаксацию и изоляцию контекста строк!
    pub fn clear_runtime_attention_buffers(&mut self) {
        self.previous_tick_spikes.clear();
        self.event_queue.clear();

        // 1. Сброс синаптических тегов активности
        for links in self.memory.adj_list.iter_mut() {
            for synapse in links.iter_mut() {
                synapse.tag_trace = 0.0;
            }
        }

        // =================================================================
        // ТОТАЛЬНЫЙ ГОМЕОСТАЗ МЕМБРАН (АКТИВНАЯ РЕЛАКСАЦИЯ ВНИМАНИЯ)
        // ИСПРАВЛЕНИЕ: Перед началом новой строки мы полностью выжигаем
        // все фоновые подпороговые заряды в ОЗУ! Нейроны начинают восприятие
        // новой строки с абсолютного нуля. Лавинный шторм полностью заблокирован.
        // =================================================================
        for neuron in self.memory.neurons.iter_mut() {
            neuron.potential = 0.0;
            neuron.last_updated_tick = self.current_tick;
        }
    }

    pub fn active_spikes_count(&self) -> usize {
        self.event_queue.len()
    }

    pub fn get_synapse_weight(&self, source_id: u64, target_id: u64) -> f32 {
        self.memory.get_synapse_weight(source_id, target_id)
    }

    pub fn apply_reinforcement(&mut self, is_success: bool) {
        apply_reinforcement_kernel(
            &mut self.memory,
            self.current_tick,
            is_success,
            CONFIG.get().unwrap(),
        );
    }

    pub fn sleep_and_prune(&mut self) {
        sleep_and_prune_kernel(&mut self.memory);
        self.coincidence_tracker.clear();
    }

    pub fn generate_autonomous_mutation(
        &self,
        start_token: &str,
        context_strings: &[String],
    ) -> Vec<String> {
        generate_trail_kernel(
            &self.memory,
            start_token,
            context_strings,
            CONFIG.get().unwrap(),
        )
    }

    pub fn tick(&mut self) {
        // ДИАГНОСТИКА ШАГА 0: Проверяем факт входа в метод
        println!("  ⚙️ [ЯДРО]: Вызван метод tick(). Текущий тик ДО инкремента: {}, Всего нейронов в памяти: {}", 
            self.current_tick, self.memory.neurons.len());

        let cfg = CONFIG.get().expect("Конфиг не инициализирован");

        self.current_tick += 1;

        let mut raw_tick_spikes = self.event_queue.extract_current_spikes(self.current_tick);
        raw_tick_spikes.sort_unstable();
        raw_tick_spikes.dedup();
        let current_tick_spikes = raw_tick_spikes;

        let mut next_spikes = Vec::new();

        for &neuron_id in &current_tick_spikes {
            let source_idx = neuron_id as usize;
            if source_idx < self.memory.adj_list.len() {
                let mut links = std::mem::take(&mut self.memory.adj_list[source_idx]);
                for synapse in links.iter_mut() {
                    if self.current_tick < synapse.cooldown_until {
                        continue;
                    }
                    synapse.trigger(self.current_tick, cfg.tag_tau);
                    let target_id = synapse.target_id;
                    let std_factor = (1.0 - (synapse.tag_trace * 0.45)).max(0.1);
                    let effective_weight = synapse.weight * std_factor;
                    if self.process_impulse_to_neuron(target_id, effective_weight) {
                        next_spikes.push(target_id);
                        synapse.cooldown_until = self.current_tick + 3;
                    }
                }
                self.memory.adj_list[source_idx] = links;
            }
        }

        // ДИАГНОСТИКА ШАГА 1: Проверяем, заходит ли выполнение в глобальную релаксацию
        let mut relaxation_executed_count = 0;
        for neuron in self.memory.neurons.iter_mut() {
            if self.current_tick > neuron.last_updated_tick {
                let delta_t = (self.current_tick - neuron.last_updated_tick) as f32;
                let leak_factor = (-delta_t / cfg.leak_tau).exp();

                // Печатаем трассировку для нашего подопытного нейрона
                if neuron.last_updated_tick == 0 && neuron.potential > 0.0 {
                    println!("    🔍 [РЕЛАКСАЦИЯ НЕЙРОНА]: Наден подопытный! Потенциал: {} -> {}, Delta_T: {}", 
                        neuron.potential, neuron.potential * leak_factor, delta_t);
                }

                neuron.potential *= leak_factor;
                neuron.last_updated_tick = self.current_tick;
                relaxation_executed_count += 1;
            }
        }

        if relaxation_executed_count > 0 {
            println!(
                "  ⚙️ [ЯДРО]: Релаксация выполнена для {} нейронов. Тик ПОСЛЕ инкремента: {}",
                relaxation_executed_count, self.current_tick
            );
        }

        self.previous_tick_spikes = current_tick_spikes;
        for target_id in next_spikes {
            self.event_queue.push(target_id, self.current_tick + 1);
        }
    }
}
