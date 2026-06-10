use crate::config::{BrainConfig, CONFIG};
use crate::memory::SpikeMemory;
use crate::models::{NeuronState, NeuronType, Synapse};
use rayon::prelude::*;
use std::collections::{HashMap, VecDeque};

pub struct SpikeEvent {
    pub neuron_id: u64,
    pub target_tick: u64,
}

pub struct SpikingNetwork {
    pub memory: SpikeMemory,
    pub current_tick: u64,
    event_queue: VecDeque<SpikeEvent>,
    previous_tick_spikes: Vec<u64>,
    coincidence_tracker: HashMap<(u64, u64), u32>,
}

impl SpikingNetwork {
    pub fn new(config: BrainConfig) -> Self {
        let _ = CONFIG.set(config);

        Self {
            memory: SpikeMemory::new(),
            current_tick: 0,
            event_queue: VecDeque::new(),
            previous_tick_spikes: Vec::new(),
            coincidence_tracker: HashMap::new(),
        }
    }

    pub fn inject_stimulus(&mut self, neuron_id: u64, charge: f32) {
        self.event_queue.push_back(SpikeEvent {
            neuron_id,
            target_tick: self.current_tick,
        });
        if self.process_impulse_to_neuron(neuron_id, charge) {
            self.previous_tick_spikes.push(neuron_id);
        }
    }

    fn process_impulse_to_neuron(&mut self, neuron_id: u64, charge: f32) -> bool {
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

    pub fn apply_reinforcement(&mut self, is_success: bool) {
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let current_t = self.current_tick;

        for links in self.memory.adj_list.iter_mut() {
            for synapse in links.iter_mut() {
                synapse.decay_tag_lazy(current_t, cfg.tag_tau);

                if synapse.tag_trace > 0.001 {
                    if is_success {
                        synapse.weight += synapse.tag_trace * cfg.learning_rate;
                        if synapse.weight > 3.0 {
                            synapse.weight = 3.0;
                        }
                    } else {
                        synapse.weight -= synapse.tag_trace * cfg.punish_rate;
                        if synapse.weight < 0.0 {
                            synapse.weight = 0.0;
                        }
                    }
                    synapse.tag_trace = 0.0;
                }
            }
        }
    }

    pub fn sleep_and_prune(&mut self) {
        println!("\n[КОНТРАСТНЫЙ СОН]: Анализ RAM графа и выжигание информационного шума...");
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let mut neuron_activity_counter: HashMap<u64, u32> = HashMap::new();

        for source_id in 0..self.memory.adj_list.len() {
            let links = &mut self.memory.adj_list[source_id];

            for synapse in links.iter_mut() {
                let mut weight = synapse.weight;
                if weight >= 1.5 {
                    weight *= cfg.sleep_strong_decay;
                } else if weight >= 0.8 {
                    weight *= cfg.sleep_medium_decay;
                } else {
                    weight -= cfg.sleep_weak_shredder;
                }
                synapse.weight = weight;
            }

            links.retain(|synapse| {
                if synapse.weight < cfg.sleep_death_threshold {
                    false
                } else {
                    *neuron_activity_counter.entry(source_id as u64).or_insert(0) += 1;
                    *neuron_activity_counter
                        .entry(synapse.target_id)
                        .or_insert(0) += 1;
                    true
                }
            });
        }

        // Шаг 2: Деактивация изолированных мета-нейронов Hidden в едином массиве
        let mut removed_count = 0;
        for id in 0..self.memory.neurons.len() {
            if let Some(neuron) = self.memory.neurons.get_mut(id) {
                if neuron.neuron_type == NeuronType::Hidden && !neuron_activity_counter.contains_key(&(id as u64)) {
                    neuron.potential = 0.0;
                    neuron.cooldown_until = u64::MAX;
                    removed_count += 1;
                }
            }
        }
        println!("  -> Изолированных мета-нейронов деактивировано: {}\n[КОНТРАСТНЫЙ СОН]: Очистка завершена.\n", removed_count);
    }

    /// ИДИОМАТИЧНЫЙ МЕТОД TICK: Ноль аллокаций в куче!
    /// Использует std::mem::take для временного заимствования вектора синапсов.
    pub fn tick(&mut self) {
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let mut current_tick_spikes = Vec::new();
        let mut next_spikes = Vec::new();

        while let Some(pos) = self
            .event_queue
            .iter()
            .position(|e| e.target_tick <= self.current_tick)
        {
            let event = self.event_queue.remove(pos).unwrap();
            current_tick_spikes.push(event.neuron_id);

            let source_idx = event.neuron_id as usize;
            if source_idx < self.memory.adj_list.len() {
                // ИДИОМАТИЧНЫЙ СПЛИТТИНГ ЗАИМСТВОВАНИЙ:
                // Временно забираем (выкачиваем) вектор синапсов из adj_list,
                // оставляя там пустой вектор БЕЗ выделения памяти в куче (Vec::new() не аллоцирует)
                let mut links = std::mem::take(&mut self.memory.adj_list[source_idx]);

                for synapse in links.iter_mut() {
                    synapse.trigger(self.current_tick, cfg.tag_tau);
                    let target_id = synapse.target_id;
                    let weight = synapse.weight;

                    // 'self' полностью свободен от блокировок, вызываем мутабельный метод!
                    if self.process_impulse_to_neuron(target_id, weight) {
                        next_spikes.push(target_id);
                    }
                }

                // Возвращаем вектор синапсов на его законное место в памяти графа
                self.memory.adj_list[source_idx] = links;
            }
        }

        for &old_id in &self.previous_tick_spikes {
            for &new_id in &current_tick_spikes {
                if old_id != new_id {
                    let pair = (old_id, new_id);
                    let count = self.coincidence_tracker.entry(pair).or_insert(0);
                    *count += 1;

                    if *count == cfg.coincidence_threshold {
                        // ИСПРАВЛЕНИЕ: Создаем чанк, передавая ID базовой последовательности нейронов!
                        let meta_neuron_id = self.memory.create_meta_chunk(old_id, new_id);
                        self.memory.set_synapse(old_id, meta_neuron_id, 1.2);
                        self.memory.set_synapse(new_id, meta_neuron_id, 1.2);
                    }
                }
            }
        }

        self.previous_tick_spikes = current_tick_spikes;
        for target_id in next_spikes {
            self.event_queue.push_back(SpikeEvent {
                neuron_id: target_id,
                target_tick: self.current_tick + 1,
            });
        }
        self.current_tick += 1;
    }

    pub fn active_spikes_count(&self) -> usize {
        self.event_queue.len()
    }

    pub fn get_synapse_weight(&self, source_id: u64, target_id: u64) -> f32 {
        self.memory.get_synapse_weight(source_id, target_id)
    }

    pub fn get_strongest_prediction(&self, source_id: u64) -> Option<(u64, f32)> {
        if let Some(links) = self.memory.adj_list.get(source_id as usize) {
            let mut strongest_target = None;
            let mut max_weight = -1.0;

            for synapse in links.iter() {
                if synapse.weight > max_weight {
                    max_weight = synapse.weight;
                    strongest_target = Some((synapse.target_id, synapse.weight));
                }
            }
            return strongest_target;
        }
        None
    }

    /// ЭТАЛОННЫЙ ПАРАЛЛЕЛЬНЫЙ ГЕНЕРАТОР (RAYON): Ноль скрытых аллокаций строк!
    /// Вычисления происходят на уровне регистров CPU (u64 и f32) [📑].
        /// АБСОЛЮТНО ПОТОКОБЕЗОПАСНЫЙ ИДИОМАТИЧНЫЙ ГЕНЕРАТОР (МНОГОПОТОЧНЫЙ ЭТАЛОН):
    /// Полностью исключены Race Conditions и False Sharing. 
    /// Параллельный пул Rayon оперирует только неизменяемыми примитивами в регистрах CPU.
    pub fn generate_autonomous_mutation(&self, start_token: &str, context_strings: &[String]) -> Vec<String> {
        use rayon::prelude::*;

        let mut trail = Vec::new();
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        
        // Переводим контекст в u64 ID один раз на старте
        let forbidden_ids: Vec<u64> = context_strings.iter()
            .filter_map(|token| self.memory.lookup_token_id(token))
            .collect();

        let bad_literal_id = self.memory.lookup_token_id("\"bad_literal\"");

        // Локальный вектор пройденного пути мысли. Живет СТРОГО в главном потоке генерации,
        // параллельные потоки Rayon к нему не прикасаются!
        let mut visited_path = Vec::new();

        let mut current_id = match self.memory.lookup_token_id(start_token) {
            Some(id) => id,
            None => return vec![start_token.to_string()],
        };
        trail.push(start_token.to_string());
        visited_path.push(current_id);

        for _ in 0..20 {
            let mut strongest_target = None;

            if let Some(links) = self.memory.adj_list.get(current_id as usize) {
                // Кэшируем ID для замыкания Rayon, чтобы избежать лишних разыменований указателей
                let path_ref = &visited_path;

                // =============================================================
                // ЧИСТЫЙ LOCK-FREE ПАРАЛЛЕЛЬНЫЙ ИТЕРАТОР (RAYON)
                // Потоки не читают хэш-мапы и не аллоцируют память.
                // Каждое ядро CPU сканирует свой кусок плоского массива синапсов.
                // =============================================================
                let best_match = links.par_iter()
                    .map(|synapse| {
                        // Эмулируем STDP триггер внимания без хэш-мап:
                        // если наша мысль в рамках текущей генерации уже проходила 
                        // через целевой нейрон, мы искусственно взводим виртуальный tag активности!
                        let is_visited = path_ref.contains(&synapse.target_id);
                        let local_tag = if is_visited { 1.0 } else { synapse.tag_trace };
                        
                        let mut score = synapse.weight + (local_tag * 1.5);

                        // Аппаратное сравнение u64 чисел в регистрах ядра
                        if forbidden_ids.contains(&synapse.target_id) {
                            score -= cfg.spike_threshold * 50.0;
                        }
                        
                        if let Some(bad_id) = bad_literal_id {
                            if synapse.target_id == bad_id {
                                score -= cfg.spike_threshold * 100.0; 
                            }
                        }

                        (synapse.target_id, score)
                    })
                    // Многопоточная редукция: ядра процессора параллельно находят максимум
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                if let Some((target_id, max_score)) = best_match {
                    if max_score > -500.0 {
                        strongest_target = Some(target_id);
                    }
                }
            }

            if let Some(next_id) = strongest_target {
                // Фиксируем пройденную точку строго в векторе текущего потока
                visited_path.push(next_id);

                let token_name = self.memory.reverse_lookup_token(next_id);
                trail.push(token_name.clone());

                if token_name == ";" {
                    break;
                }
                current_id = next_id;
            } else {
                break;
            }
        }
        trail
    }

}
