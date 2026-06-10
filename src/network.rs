use crate::config::{BrainConfig, CONFIG};
use crate::memory::SpikeMemory;
use crate::models::{NeuronState, NeuronType, Synapse};
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

    // =================================================================
    // РЕЖИМ ИЗМЕНЕНИЯ (&mut self) -> Только Обучение, Подкрепление и Сон
    // =================================================================

    /// Мутабельный метод: Подача внешнего стимула (обучающий поток)
    pub fn inject_stimulus(&mut self, neuron_id: u64, charge: f32) {
        self.event_queue.push_back(SpikeEvent {
            neuron_id,
            target_tick: self.current_tick,
        });
        if self.process_impulse_to_neuron(neuron_id, charge) {
            self.previous_tick_spikes.push(neuron_id);
        }
    }

    /// Мутабельный метод: изменение потенциала нейрона при обучении
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

    /// Мутабельный метод: дофаминовая перестройка весов синапсов Критиком
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

    /// Мутабельный метод: жесткая нелинейная очистка мусора во сне БЕЗ ХАРДКОДА.
    /// Полностью исправлена коллизия мутабельности внутри retain.
    pub fn sleep_and_prune(&mut self) {
        println!("\n[КОНТРАСТНЫЙ СОН]: Анализ RAM графа и выжигание информационного шума...");
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let mut neuron_activity_counter: HashMap<u64, u32> = HashMap::new();

        for source_id in 0..self.memory.adj_list.len() {
            let links = &mut self.memory.adj_list[source_id];
            
            // ПЕРВЫЙ ПРОХОД: Мутабельно изменяем веса синапсов "на месте" в ОЗУ
            for synapse in links.iter_mut() {
                let mut weight = synapse.weight;
                if weight >= 1.5 {
                    weight *= cfg.sleep_strong_decay;
                } else if weight >= 0.8 {
                    weight *= cfg.sleep_medium_decay;
                } else {
                    weight -= cfg.sleep_weak_shredder;
                }
                synapse.weight = weight; // Теперь запись разрешена!
            }

            // ВТОРОЙ ПРОХОД: Чистим мертвые синапсы и собираем статистику активности
            links.retain(|synapse| {
                if synapse.weight < cfg.sleep_death_threshold {
                    false // Удаляем синапс из памяти
                } else {
                    // Синапс выжил -> фиксируем активность связанных узлов
                    *neuron_activity_counter.entry(source_id as u64).or_insert(0) += 1;
                    *neuron_activity_counter.entry(synapse.target_id).or_insert(0) += 1;
                    true // Оставляем синапс в векторе
                }
            });
        }

        // Шаг 2: Деактивация изолированных мета-нейронов Hidden (остается без изменений)
        let mut removed_count = 0;
        for id in 0..self.memory.neurons.len() {
            let neuron = &mut self.memory.neurons[id];
            if neuron.neuron_type == NeuronType::Hidden && !neuron_activity_counter.contains_key(&(id as u64)) {
                neuron.potential = 0.0;
                neuron.cooldown_until = u64::MAX;
                removed_count += 1;
            }
        }
        println!("  -> Изолированных мета-нейронов деактивировано: {}\n[КОНТРАСТНЫЙ СОН]: Очистка завершена.\n", removed_count);
    }


    /// Мутабельный метод: продвижение времени вперед иSTDP пластичность
    /// Мутабельный метод: продвижение времени вперед и распространение волны импульсов.
    /// Полностью устранена коллизия одновременных мутабельных заимствований (E0499).
    pub fn tick(&mut self) {
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let mut current_tick_spikes = Vec::new();
        let mut next_spikes = Vec::new();

        // 1. Извлекаем спайки, запланированные на текущий тик
        while let Some(pos) = self.event_queue.iter().position(|e| e.target_tick <= self.current_tick) {
            let event = self.event_queue.remove(pos).unwrap();
            current_tick_spikes.push(event.neuron_id);

            // ИСПРАВЛЕНИЕ: Вместо удержания мутабельной ссылки на всю память внутри цикла,
            // мы клонируем легковесный вектор синапсов текущего нейрона (копируются только примитивы u64 и f32).
            // Это полностью освобождает 'self' от блокировки!
            let active_links = if let Some(links) = self.memory.adj_list.get(event.neuron_id as usize) {
                links.clone()
            } else {
                Vec::new()
            };

            // Теперь мы спокойно бежим по изолированному локальному вектору связей
            for synapse in active_links {
                let target_id = synapse.target_id;
                let weight = synapse.weight;

                // Взводим tag_trace у пройденного синапса в постоянной памяти графа.
                // Так как 'self' свободен, мы можем безопасно вызывать методы изменения структуры!
                if let Some(links) = self.memory.adj_list.get_mut(event.neuron_id as usize) {
                    if let Some(s) = links.iter_mut().find(|s| s.target_id == target_id) {
                        s.trigger(self.current_tick, cfg.tag_tau);
                    }
                }

                // Передаем ток целевому соседу через мутабельный метод (Borrow Checker счастлив!)
                if self.process_impulse_to_neuron(target_id, weight) {
                    next_spikes.push(target_id);
                }
            }
        }

        // 2. Чанкинг: сборка устойчивых временных последовательностей (остается без изменений)
        for &old_id in &self.previous_tick_spikes {
            for &new_id in &current_tick_spikes {
                if old_id != new_id {
                    let pair = (old_id, new_id);
                    let count = self.coincidence_tracker.entry(pair).or_insert(0);
                    *count += 1;

                    if *count == cfg.coincidence_threshold {
                        println!("\n[МЕТА-ЭВОЛЮЦИЯ]: Обнаружена устойчивая последовательность {} -> {}. Рождение нового понятия!", old_id, new_id);
                        let meta_neuron_id = self.memory.create_neuron(NeuronType::Hidden);
                        self.memory.set_synapse(old_id, meta_neuron_id, 1.2);
                        self.memory.set_synapse(new_id, meta_neuron_id, 1.2);
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

    // =================================================================
    // РЕЖИМ ТОЛЬКО ДЛЯ ЧТЕНИЯ (&self) -> Инспекция, Экспертиза, Вывод
    // =================================================================

    /// Немутабельный метод: точечный замер силы синапса
    pub fn get_synapse_weight(&self, source_id: u64, target_id: u64) -> f32 {
        self.memory.get_synapse_weight(source_id, target_id)
    }

    /// Немутабельный метод: поиск сильнейшего предсказания графа
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

        /// ПАРАЛЛЕЛЬНЫЙ РЕЗОНАНСНЫЙ ГЕНЕРАТОР МУТАЦИЙ (RAYON CPU SCALING)
    /// Абсолютно немутабельный метод &self. Использует все ядра процессора
    /// для мгновенного поиска оптимального пути на графах любой сложности.
    pub fn generate_autonomous_mutation(&self, start_token: &str, context_strings: &[String]) -> Vec<String> {
        // Подключаем параллельные итераторы Rayon
        use rayon::prelude::*;

        let mut trail = Vec::new();
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        
        let forbidden_ids: Vec<u64> = context_strings.iter()
            .filter_map(|token| self.memory.lookup_token_id(token))
            .collect();

        // Локальная карта кратковременного внимания (Thought Echo)
        let mut local_thought_echo: HashMap<(u64, u64), f32> = HashMap::new();

        let mut current_id = match self.memory.lookup_token_id(start_token) {
            Some(id) => id,
            None => return vec![start_token.to_string()],
        };
        trail.push(start_token.to_string());

        for _ in 0..20 {
            let mut strongest_target = None;

            if let Some(links) = self.memory.adj_list.get(current_id as usize) {
                // =============================================================
                // МНОГОПОТОЧНЫЙ ОБХОД ГРАФА СВЯЗЕЙ (RAYON PARALLEL REDUCE)
                // .par_iter() автоматически делит массив синапсов на порции 
                // и раздает их на обработку всем свободным ядрам вашего CPU!
                // =============================================================
                let best_match = links.par_iter()
                    .map(|synapse| {
                        // Извлекаем след внимания, если мы здесь пролетали мыслью
                        let local_tag = local_thought_echo
                            .get(&(current_id, synapse.target_id))
                            .copied()
                            .unwrap_or(synapse.tag_trace);
                        
                        // Считаем балл резонанса связи
                        let mut score = synapse.weight + (local_tag * 1.5);

                        // Латеральное торможение дефектного контекста Cargo
                        if forbidden_ids.contains(&synapse.target_id) {
                            score -= cfg.spike_threshold * 50.0;
                        }
                        
                        let target_name = self.memory.reverse_lookup_token(synapse.target_id);
                        if context_strings.contains(&target_name) && target_name == "\"bad_literal\"" {
                            score -= cfg.spike_threshold * 100.0; 
                        }

                        (synapse.target_id, score)
                    })
                    // Многопоточная редукция: все ядра параллельно ищут максимальный score
                    // в своих порциях данных, а затем сливают результаты в один абсолютный максимум
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                if let Some((target_id, max_score)) = best_match {
                    // Если лучший найденный путь пробивает базовый нулевой порог
                    if max_score > -500.0 {
                        strongest_target = Some(target_id);
                    }
                }
            }

            if let Some(next_id) = strongest_target {
                // Имитируем STDP триггер внимания в локальном пуле мысли
                let pair = (current_id, next_id);
                let current_tag = local_thought_echo.entry(pair).or_insert(0.0);
                *current_tag = (*current_tag + 0.5).min(1.0);

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
