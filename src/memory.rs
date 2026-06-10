use std::collections::HashMap;
use crate::models::{NeuronState, Synapse, NeuronType};

pub struct SpikeMemory {
    // Плоские массивы в RAM для максимального кэш-локального доступа CPU
    pub neurons: Vec<NeuronState>,
    // Индекс графа: ID источника -> Список исходящих синапсов (цель, вес, след)
    pub adj_list: Vec<Vec<Synapse>>,
    // Словарь токенов
    pub vocabulary: HashMap<String, u64>,
}

impl SpikeMemory {
    /// Создает чистую оперативную память графа знаний
    pub fn new() -> Self {
        Self {
            neurons: Vec::with_capacity(10000),
            adj_list: Vec::with_capacity(10000),
            vocabulary: HashMap::with_capacity(5000),
        }
    }

    // =================================================================
    // РЕЖИМ ИЗМЕНЕНИЯ (Мутабельный - &mut self) -> Для Фазы Обучения
    // =================================================================

    /// Выращивает новый нейрон в памяти и возвращает его числовой ID
    pub fn create_neuron(&mut self, neuron_type: NeuronType) -> u64 {
        let id = self.neurons.len() as u64;
        self.neurons.push(NeuronState::new(neuron_type));
        self.adj_list.push(Vec::new()); // Инициализируем пустой список связей для этого узла
        id
    }

    /// Регистрирует слово в словаре. Если его нет — создает Sensor-нейрон.
    pub fn get_or_create_token(&mut self, token: &str) -> u64 {
        if let Some(&id) = self.vocabulary.get(token) {
            return id;
        }
        let new_id = self.create_neuron(NeuronType::Sensor);
        self.vocabulary.insert(token.to_string(), new_id);
        new_id
    }

    /// Прокладывает или обновляет синапс между двумя нейронами
    pub fn set_synapse(&mut self, source_id: u64, target_id: u64, weight: f32) {
        let source_idx = source_id as usize;
        
        // Проверяем, существует ли уже такая связь
        if let Some(synapse) = self.adj_list[source_idx].iter_mut().find(|s| s.target_id == target_id) {
            synapse.weight = weight; // Обновляем вес
        } else {
            // Создаем новую связь
            self.adj_list[source_idx].push(Synapse {
                target_id,
                weight,
                tag_trace: 0.0,
                last_used_tick: 0,
            });
        }
    }

    // =================================================================
    // РЕЖИМ ТОЛЬКО ДЛЯ ЧТЕНИЯ (Немутабельный - &self) -> Для Экспертизы и Вывода
    // =================================================================

    /// Возвращает вес синапса между двумя узлами (0.0 если связи нет)
    pub fn get_synapse_weight(&self, source_id: u64, target_id: u64) -> f32 {
        if let Some(links) = self.adj_list.get(source_id as usize) {
            if let Some(synapse) = links.iter().find(|s| s.target_id == target_id) {
                return synapse.weight;
            }
        }
        0.0
    }

    /// Находит токен в словаре без права на создание новых нейронов
    pub fn lookup_token_id(&self, token: &str) -> Option<u64> {
        self.vocabulary.get(token).copied()
    }

    /// Обратный поиск имени токена по его ID для вывода результатов мышления
    pub fn reverse_lookup_token(&self, target_id: u64) -> String {
        for (word, &id) in &self.vocabulary {
            if id == target_id {
                return word.clone();
            }
        }
        format!("meta_id_{}", target_id)
    }
}
