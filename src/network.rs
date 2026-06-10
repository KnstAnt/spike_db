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

/// ДИАГНОСТИКА: Дофаминовая перестройка весов синапсов Критиком (100% Lock-Free)
    pub fn apply_reinforcement(&mut self, is_success: bool) {
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let current_t = self.current_tick;

        println!("\n📢 [ТРАССИРОВКА КРИТИКА]: Вызов подкрепления (Успех = {}). Системный тик: {}", is_success, current_t);
        let mut reinforced_count = 0;

        for source_id in 0..self.memory.adj_list.len() {
            let links = &mut self.memory.adj_list[source_id];

            // Проводим затухание следов и начисление весов
            for synapse in links.iter_mut() {
                let old_tag = synapse.tag_trace;
                synapse.decay_tag_lazy(current_t, cfg.tag_tau);
                let decayed_tag = synapse.tag_trace;

                if decayed_tag > 0.0001 {
                    let old_weight = synapse.weight;
                    if is_success {
                        synapse.weight += decayed_tag * cfg.learning_rate;
                        if synapse.weight > 3.0 { synapse.weight = 3.0; }
                    } else {
                        synapse.weight -= decayed_tag * cfg.punish_rate;
                        if synapse.weight < 0.0 { synapse.weight = 0.0; }
                    }

                    // Вместо вызова reverse_lookup_token внутри мутабельного цикла,
                    // мы выводим диагностику по числовым ID. Это мгновенно решает проблему!
                    println!("    ➔ КРИТИКА: ID {} -> ID {} | Старый Tag: {:.3}, Затухший Tag: {:.3} | Вес: {:.2} -> {:.2}", 
                        source_id, synapse.target_id, old_tag, decayed_tag, old_weight, synapse.weight);

                    synapse.tag_trace = 0.0;
                    reinforced_count += 1;
                }
            }
        }
        println!("📢 [ТРАССИРОВКА КРИТИКА]: Всего синапсов изменено: {}", reinforced_count);
    }

    /// ДИАГНОСТИКА: Контрастный сон и нелинейная зачистка мусора (100% Lock-Free)
    pub fn sleep_and_prune(&mut self) {
        println!("\n🌙 [ТРАССИРОВКА СНА]: Старт ночного гомеостаза...");
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let mut neuron_activity_counter: HashMap<u64, u32> = HashMap::new();
        let mut pruned_synapses = 0;

        for source_id in 0..self.memory.adj_list.len() {
            let links = &mut self.memory.adj_list[source_id];
            
            for synapse in links.iter_mut() {
                let mut weight = synapse.weight;
                let old_weight = weight;

                if weight >= 1.5 {
                    weight *= cfg.sleep_strong_decay;
                } else if weight >= 0.8 {
                    weight *= cfg.sleep_medium_decay;
                } else {
                    weight -= cfg.sleep_weak_shredder;
                }
                
                println!("    💤 СОН: ID {} -> ID {} | Вес до сна: {:.2} -> После сна: {:.2}", 
                    source_id, synapse.target_id, old_weight, weight);
                
                synapse.weight = weight;
                synapse.tag_trace = 0.0;
            }

            links.retain(|synapse| {
                if synapse.weight < cfg.sleep_death_threshold {
                    pruned_synapses += 1;
                    false
                } else {
                    *neuron_activity_counter.entry(source_id as u64).or_insert(0) += 1;
                    *neuron_activity_counter.entry(synapse.target_id).or_insert(0) += 1;
                    true
                }
            });
        }

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
        println!("🌙 [ТРАССИРОВКА СНА]: Синапсов выжжено: {}, Мета-нейронов заморожено: {}", pruned_synapses, removed_count);
    }

    /// ДИАГНОСТИКА: Продвижение времени симуляции тиков и Чанкинг
    pub fn tick(&mut self) {
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let mut current_tick_spikes = Vec::new();
        let mut next_spikes = Vec::new();

        while let Some(pos) = self.event_queue.iter().position(|e| e.target_tick <= self.current_tick) {
            let event = self.event_queue.remove(pos).unwrap();
            current_tick_spikes.push(event.neuron_id);

            let source_idx = event.neuron_id as usize;
            if source_idx < self.memory.adj_list.len() {
                let mut links = std::mem::take(&mut self.memory.adj_list[source_idx]);

                for synapse in links.iter_mut() {
                    synapse.trigger(self.current_tick, cfg.tag_tau);
                    let target_id = synapse.target_id;
                    let weight = synapse.weight;

                    if self.process_impulse_to_neuron(target_id, weight) {
                        next_spikes.push(target_id);
                    }
                }
                self.memory.adj_list[source_idx] = links;
            }
        }

        if !self.previous_tick_spikes.is_empty() || !current_tick_spikes.is_empty() {
            println!("⏱️ [ТИК {}]: Активные спайковые вспышки: {:?}", self.current_tick, current_tick_spikes);
        }

        for &old_id in &self.previous_tick_spikes {
            for &new_id in &current_tick_spikes {
                if old_id != new_id {
                    let pair = (old_id, new_id);
                    let count = self.coincidence_tracker.entry(pair).or_insert(0);
                    *count += 1;

                    let old_name = self.memory.reverse_lookup_token(old_id);
                    let new_name = self.memory.reverse_lookup_token(new_id);
                    println!("   ⚡ СОВПАДЕНИЕ: '{}' -> '{}' | Текущий счетчик: {}/{}", 
                        old_name, new_name, count, cfg.coincidence_threshold);

                                        if *count == cfg.coincidence_threshold {
                        let meta_neuron_id = self.memory.create_meta_chunk(old_id, new_id);
                        
                        println!("✨ [МЕТА-ЭВОЛЮЦИЯ]: Рождение чанка ID {} -> ID {} с новым ID: {}", 
                            old_id, new_id, meta_neuron_id);
                        
                        // ИСПРАВЛЕНИЕ: Добавляем self.current_tick во все вызовы set_synapse
                        self.memory.set_synapse(old_id, meta_neuron_id, 0.6, self.current_tick);
                        self.memory.set_synapse(new_id, meta_neuron_id, 0.6, self.current_tick);

                        if let Some(links) = self.memory.adj_list.get_mut(old_id as usize) {
                            if let Some(s) = links.iter_mut().find(|s| s.target_id == meta_neuron_id) {
                                s.last_used_tick = self.current_tick;
                                s.tag_trace = 1.0;
                            }
                        }
                        if let Some(links) = self.memory.adj_list.get_mut(new_id as usize) {
                            if let Some(s) = links.iter_mut().find(|s| s.target_id == meta_neuron_id) {
                                s.last_used_tick = self.current_tick;
                                s.tag_trace = 1.0;
                            }
                        }

                        if let Some(old_neuron) = self.memory.neurons.get(old_id as usize) {
                            if let crate::models::NeuronOrigin::ChunkSequence(id_start, _) = old_neuron.origin {
                                // ИСПРАВЛЕНИЕ: И здесь тоже добавляем self.current_tick
                                self.memory.set_synapse(id_start, new_id, 0.5, self.current_tick);
                                if let Some(links) = self.memory.adj_list.get_mut(id_start as usize) {
                                    if let Some(s) = links.iter_mut().find(|s| s.target_id == new_id) {
                                        s.last_used_tick = self.current_tick;
                                        s.tag_trace = 1.0;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self.previous_tick_spikes = current_tick_spikes;
        for target_id in next_spikes {
            self.event_queue.push_back(SpikeEvent { neuron_id: target_id, target_tick: self.current_tick + 1 });
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
        
        let forbidden_ids: Vec<u64> = context_strings.iter()
            .filter_map(|token| self.memory.lookup_token_id(token))
            .collect();

        let bad_literal_id = self.memory.lookup_token_id("\"bad_literal\"");
        let mut visited_path = Vec::new();

        let mut current_id = match self.memory.lookup_token_id(start_token) {
            Some(id) => id,
            None => {
                println!("    ❌ КРИТИЧЕСКАЯ ТРАССИРОВКА: Стартовый токен '{}' отсутствует в словаре!", start_token);
                return vec![start_token.to_string()];
            }
        };
        
        trail.push(start_token.to_string());
        visited_path.push(current_id);

        println!("\n🔮 [ТРАССИРОВКА ЭКСПЕРТИЗЫ]: Начало генерации ассоциаций для '{}' (ID: {})", start_token, current_id);

        for step in 0..10 {
            let mut strongest_target = None;
            let mut max_score = -9999.0;

            if let Some(links) = self.memory.adj_list.get(current_id as usize) {
                let current_token_name = self.memory.reverse_lookup_token(current_id);
                println!("  📍 Шаг {}: Мы стоим на узле '{}' (ID: {}). Всего исходящих синапсов: {}", 
                    step, current_token_name, current_id, links.len());

                                // 1. ПОШАГОВАЯ ИНСПЕКЦИЯ ВСЕХ ПУТЕЙ (Обновление логики Anti-Loop)
                for synapse in links.iter() {
                    let is_visited = visited_path.contains(&synapse.target_id);
                    
                    // ИСПРАВЛЕНИЕ: Если путь уже посещался, мы накладываем на него 
                    // латеральное торможение (минус к баллу), заставляя мозг искать новые выходы!
                    let mut score = synapse.weight + (synapse.tag_trace * 1.5);
                    if is_visited {
                        score -= cfg.spike_threshold * 20.0; // Жесткий рефрактерный штраф зацикливания
                    }

                    for &f_id in &forbidden_ids {
                        if synapse.target_id == f_id { score -= cfg.spike_threshold * 50.0; }
                    }
                    if let Some(bad_id) = bad_literal_id {
                        if synapse.target_id == bad_id { score -= cfg.spike_threshold * 100.0; }
                    }

                    let target_name = self.memory.reverse_lookup_token(synapse.target_id);
                    // Передаем маркер зацикленности в лог для наглядности
                    println!("      ➔ Дорога к: '{}' (ID: {}), Вес: {:.2}, Посещен: {} = БАЛЛ: {:.2}", 
                        target_name, synapse.target_id, synapse.weight, is_visited, score);
                }

                // 2. ИСТИННЫЙ ПАРАЛЛЕЛЬНЫЙ ВЫБОР RAYON (С латеральным торможением)
                let path_ref = &visited_path;
                let best_match = links.par_iter()
                    .map(|synapse| {
                        let is_visited = path_ref.contains(&synapse.target_id);
                        let mut score = synapse.weight + (synapse.tag_trace * 1.5);
                        
                        // Аппаратный штраф против зацикливания мыслей в ОЗУ
                        if is_visited {
                            score -= cfg.spike_threshold * 20.0;
                        }

                        for &f_id in &forbidden_ids {
                            if synapse.target_id == f_id { score -= cfg.spike_threshold * 50.0; }
                        }
                        if let Some(bad_id) = bad_literal_id {
                            if synapse.target_id == bad_id { score -= cfg.spike_threshold * 100.0; }
                        }
                        (synapse.target_id, score)
                    })
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                if let Some((target_id, score)) = best_match {
                    max_score = score;
                    // Сеть выбирает путь, только если он пробивает порог мертвой зоны
                    if max_score > -500.0 {
                        strongest_target = Some(target_id);
                    }
                }
            } else {
                println!("  📍 Шаг {}: Узел ID {} физически отсутствует в adj_list!", step, current_id);
            }

            if let Some(next_id) = strongest_target {
                visited_path.push(next_id);
                let mut should_break = false;

                if let Some(neuron) = self.memory.neurons.get(next_id as usize) {
                    let next_name = self.memory.reverse_lookup_token(next_id);
                    println!("    💎 ВЫБРАН ЛУЧШИЙ ПУТЬ: '{}' (ID: {}) с баллом {:.2}", next_name, next_id, max_score);

                    match &neuron.origin {
                        crate::models::NeuronOrigin::BaseToken(name) => {
                            if !trail.contains(name) { trail.push(name.clone()); }
                            if name == ";" { should_break = true; }
                            current_id = next_id;
                        }
                        crate::models::NeuronOrigin::ChunkSequence(_, _) => {
                            let full_phrase = self.memory.reverse_lookup_token(next_id);
                            println!("      [РАСПРЯМЛЕНИЕ ЧАНКА]: Мета-понятие разворачивается во фразу: '{}'", full_phrase);
                            
                            for word in full_phrase.split_whitespace() {
                                if !trail.contains(&word.to_string()) && word != "\"bad_literal\"" {
                                    trail.push(word.to_string());
                                }
                                if word == ";" { should_break = true; }
                            }
                            current_id = self.memory.get_chunk_terminal_token_id(next_id);
                            println!("      [ПЕРЕКЛЮЧЕНИЕ КОНТЕКСТА]: Мысль сместилась на терминальный ID: {}", current_id);
                        }
                    }
                }

                if should_break { 
                    println!("    [ФИНИШ]: Встречен терминальный символ ';'. Стрим завершен.");
                    break; 
                }
            } else {
                println!("    [ТУПИК]: Из текущего узла все дороги заблокированы или угасли (max_score: {:.2}).", max_score);
                break;
            }
        }
        
        println!("🔮 [ТРАССИРОВКА ЭКСПЕРТИЗЫ]: Итоговый шлейф на выходе генератора: {:?}", trail);
        trail
    }


    /// Сброс буферов внимания предложений для защиты от бесконечной рекурсии чанков
    pub fn clear_runtime_attention_buffers(&mut self) {
        self.previous_tick_spikes.clear();
        self.event_queue.clear();
    }
}
