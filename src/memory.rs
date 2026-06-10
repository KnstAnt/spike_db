use std::collections::HashMap;
use crate::models::{NeuronState, Synapse, NeuronType, NeuronOrigin};

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
        self.neurons.push(NeuronState::new(NeuronType::Sensor, NeuronOrigin::BaseToken(token.to_string())));
        self.adj_list.push(Vec::new());

        self.vocabulary.insert(token.to_string(), id);
        id
    }

    /// Выращивает мета-нейрон чанкинга на основе СВЯЗИ между двумя нейронами
    pub fn create_meta_chunk(&mut self, source_id: u64, target_id: u64) -> u64 {
        let id = self.neurons.len() as u64;
        
        // Записываем ссылки на базовые нейроны происхождения
        self.neurons.push(NeuronState::new(NeuronType::Hidden, NeuronOrigin::ChunkSequence(source_id, target_id)));
        self.adj_list.push(Vec::new());
        id
    }

    pub fn set_synapse(&mut self, source_id: u64, target_id: u64, weight: f32) {
        let source_idx = source_id as usize;
        if let Some(synapse) = self.adj_list[source_idx].iter_mut().find(|s| s.target_id == target_id) {
            synapse.weight = weight;
        } else {
            self.adj_list[source_idx].push(Synapse {
                target_id,
                weight,
                tag_trace: 0.0,
                last_used_tick: 0,
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
}
