use crate::models::{NeuronOrigin, NeuronState, NeuronType, Synapse};
use std::collections::HashMap;

pub struct SpikeMemory {
    // ЕДИНЫЙ РАВНОЗНАЧНЫЙ СПИСОК НЕЙРОНОВ В ОЗУ
    pub neurons: Vec<NeuronState>,
    pub adj_list: Vec<Vec<Synapse>>,
    pub vocabulary: HashMap<String, u64>,
}

impl SpikeMemory {
    pub fn new() -> Self {
        Self {
            neurons: Vec::with_capacity(10000),
            adj_list: Vec::with_capacity(10000),
            vocabulary: HashMap::with_capacity(5000),
        }
    }

    /// Выращивает базовый сенсорный нейрон токена
    pub fn get_or_create_token(&mut self, token: &str) -> u64 {
        if let Some(&id) = self.vocabulary.get(token) {
            return id;
        }

        let id = self.neurons.len() as u64;
        // Записываем происхождение токена
        self.neurons.push(NeuronState::new(
            NeuronType::Sensor,
            NeuronOrigin::BaseToken(token.to_string()),
        ));
        self.adj_list.push(Vec::new());

        self.vocabulary.insert(token.to_string(), id);
        id
    }

    /// Выращивает мета-нейрон чанкинга на основе СВЯЗИ между двумя нейронами
    pub fn create_meta_chunk(&mut self, source_id: u64, target_id: u64) -> u64 {
        let id = self.neurons.len() as u64;

        // Записываем ссылки на базовые нейроны происхождения
        self.neurons.push(NeuronState::new(
            NeuronType::Hidden,
            NeuronOrigin::ChunkSequence(source_id, target_id),
        ));
        self.adj_list.push(Vec::new());
        id
    }

    /// Прокладывает или лениво релаксирует и активирует существующий синапс в ОЗУ,
    /// строго соблюдая биологический ход времени и угасание следов пластичности.
    pub fn set_synapse(&mut self, source_id: u64, target_id: u64, weight: f32, current_tick: u64) {
        let source_idx = source_id as usize;
        let cfg = crate::config::CONFIG
            .get()
            .expect("Конфиг не инициализирован");

        if let Some(synapse) = self.adj_list[source_idx]
            .iter_mut()
            .find(|s| s.target_id == target_id)
        {
            synapse.weight = weight;

            // =================================================================
            // ВОЗВРАТ ИСХОДНОГО КОНТРАКТА: ПАССИВНАЯ СИНАПТИЧЕСКАЯ РЕЛАКСАЦИЯ
            // ИСПРАВЛЕНИЕ: Перед тем как взвести tag_trace, мы ОБЯЗАНЫ дать
            // синапсу честно остыть во времени по экспоненте затухания,
            // согласно дельте тиков, прошедших с его последнего использования!
            // =================================================================
            synapse.decay_tag_lazy(current_tick, cfg.tag_tau);

            // Накапливаем след активности на основе УЖЕ отрелаксировавшего значения
            synapse.tag_trace = (synapse.tag_trace + 0.5).min(1.0);
            synapse.last_used_tick = current_tick;
        } else {
            self.adj_list[source_idx].push(Synapse {
                target_id,
                weight,
                tag_trace: 1.0,
                last_used_tick: current_tick,
                cooldown_until: 0,
            });
        }
    }

    pub fn get_synapse_weight(&self, source_id: u64, target_id: u64) -> f32 {
        if let Some(links) = self.adj_list.get(source_id as usize) {
            if let Some(synapse) = links.iter().find(|s| s.target_id == target_id) {
                return synapse.weight;
            }
        }
        0.0
    }

    pub fn lookup_token_id(&self, token: &str) -> Option<u64> {
        self.vocabulary.get(token).copied()
    }

    /// КАНОНИЧЕСКИЙ РЕКУРСИВНЫЙ ПОИСК ПО ССЫЛКАМ ПРОИСХОЖДЕНИЯ
    /// Разворачивает иерархию мета-понятий в готовую цепочку токенов!
    pub fn reverse_lookup_token(&self, target_id: u64) -> String {
        if let Some(neuron) = self.neurons.get(target_id as usize) {
            match &neuron.origin {
                // Если нейрон создан на основе базового токена — возвращаем его имя
                NeuronOrigin::BaseToken(name) => name.clone(),

                // Если нейрон мета-понятие — рекурсивно раскручиваем имена базовых нейронов!
                NeuronOrigin::ChunkSequence(id_a, id_b) => {
                    let part_a = self.reverse_lookup_token(*id_a);
                    let part_b = self.reverse_lookup_token(*id_b);
                    // Склеиваем цепочку через пробел
                    format!("{} {}", part_a, part_b)
                }
            }
        } else {
            format!("unknown_id_{}", target_id)
        }
    }
    /// Рекурсивно раскручивает граф происхождения чанка и возвращает ID
    /// самого последнего (терминального) сенсорного токена, которым заканчивается фраза.
    pub fn get_chunk_terminal_token_id(&self, target_id: u64) -> u64 {
        if let Some(neuron) = self.neurons.get(target_id as usize) {
            match &neuron.origin {
                // Если это базовое слово — это и есть искомый ID
                NeuronOrigin::BaseToken(_) => target_id,
                // Если это чанк — рекурсивно ныряем в правое (последнее) плечо последовательности!
                NeuronOrigin::ChunkSequence(_, id_b) => self.get_chunk_terminal_token_id(*id_b),
            }
        } else {
            target_id
        }
    }
    /// Рекурсивно проверяет, содержит ли чанк или базовый токен
    /// хотя бы один запрещенный идентификатор из списка вето.
    pub fn is_chunk_containing_forbidden_ids(&self, target_id: u64, forbidden_ids: &[u64]) -> bool {
        if forbidden_ids.contains(&target_id) {
            return true;
        }
        if let Some(neuron) = self.neurons.get(target_id as usize) {
            match &neuron.origin {
                NeuronOrigin::BaseToken(_) => false,
                // Если это чанк — рекурсивно проверяем оба его плеча!
                NeuronOrigin::ChunkSequence(id_a, id_b) => {
                    self.is_chunk_containing_forbidden_ids(*id_a, forbidden_ids)
                        || self.is_chunk_containing_forbidden_ids(*id_b, forbidden_ids)
                }
            }
        } else {
            false
        }
    }
}
