use bincode_next as bincode;

use crate::models::{NeuronState, NeuronType, Synapse};
use byteorder::{BigEndian, WriteBytesExt};
use sled::{Db, Tree};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

const COINCIDENCE_THRESHOLD: u32 = 3;

// Коэффициенты обучения
const LEARNING_RATE: f32 = 0.3; // На сколько увеличивать вес при успехе
const PUNISH_RATE: f32 = 0.2; // На сколько уменьшать вес при ошибке


pub struct SpikeEvent {
    pub neuron_id: u64,
    pub target_tick: u64,
}

pub struct SpikingNetwork {
    db: Db,
    neurons: Tree,
    synapses: Tree,
    vocabulary: Tree, 
    pub current_tick: u64,
    event_queue: VecDeque<SpikeEvent>,
    next_neuron_id: AtomicU64,
    previous_tick_spikes: Vec<u64>,
    coincidence_tracker: HashMap<(u64, u64), u32>,
}

impl SpikingNetwork {
    pub fn new(path: &str) -> Self {
        let db = sled::open(path).expect("Не удалось открыть базу данных SpikeDB");
        let neurons = db
            .open_tree("neurons")
            .expect("Ошибка создания дерева нейронов");
        let synapses = db
            .open_tree("synapses")
            .expect("Ошибка создания дерева синапсов");
        let vocabulary = db.open_tree("vocabulary").expect("Ошибка создания дерева словаря");

        Self {
            db,
            neurons,
            synapses,
            vocabulary,
            current_tick: 0,
            event_queue: VecDeque::new(),
            next_neuron_id: AtomicU64::new(0),
            previous_tick_spikes: Vec::new(),
            coincidence_tracker: HashMap::new(),
        }
    }

    /// Ищет ID нейрона по текстовому токену. Если токен новый — создает для него Sensor-нейрон.
    pub fn get_or_create_token_neuron(&self, token: &str) -> u64 {
        // Проверяем, есть ли уже такое слово в БД Sled
        if let Some(ivec) = self.vocabulary.get(token.as_bytes()).expect("Ошибка чтения словаря") {
            use byteorder::{BigEndian, ReadBytesExt};
            let mut rdr = &ivec[..];
            return rdr.read_u64::<BigEndian>().unwrap();
        }

        // Если слова нет — выращиваем под него новый Sensor-нейрон
        let new_id = self.create_neuron(NeuronType::Sensor);

        // Кодируем ID в байты для записи значения в словарь
        let mut id_bytes = Vec::with_capacity(8);
        use byteorder::BigEndian as BE;
        id_bytes.write_u64::<BE>(new_id).unwrap();

        // Сохраняем связь "слово -> ID" в таблицу vocabulary
        self.vocabulary.insert(token.as_bytes(), id_bytes).expect("Ошибка записи в словарь");
        
        new_id
    }

    fn encode_synapse_key(source_id: u64, target_id: u64) -> Vec<u8> {
        let mut wtr = Vec::with_capacity(16);
        wtr.write_u64::<BigEndian>(source_id).unwrap();
        wtr.write_u64::<BigEndian>(target_id).unwrap();
        wtr
    }

    fn encode_prefix_key(source_id: u64) -> Vec<u8> {
        let mut wtr = Vec::with_capacity(8);
        wtr.write_u64::<BigEndian>(source_id).unwrap();
        wtr
    }

    pub fn create_neuron(&self, neuron_type: NeuronType) -> u64 {
        const MAX_ID_LIMIT: u64 = u64::MAX / 2;
        let mut key_bytes = Vec::with_capacity(8);

        loop {
            let mut id = self.next_neuron_id.fetch_add(1, Ordering::SeqCst);

            if id >= MAX_ID_LIMIT {
                let _ = self.next_neuron_id.compare_exchange(
                    id + 1,
                    0,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
                id = 0;
            }

            key_bytes.clear();
            use byteorder::BigEndian as BE;
            key_bytes.write_u64::<BE>(id).unwrap();

            if !self
                .neurons
                .contains_key(&key_bytes)
                .expect("Ошибка чтения БД")
            {
                let state = NeuronState::new(neuron_type);
                let bytes = bincode::encode_to_vec(&state, bincode::config::standard())
                    .expect("Ошибка serialization нейрона");

                self.neurons
                    .insert(&key_bytes, bytes)
                    .expect("Ошибка записи нейрона в БД");
                return id;
            }
        }
    }

    pub fn set_synapse(&self, source_id: u64, target_id: u64, weight: f32) {
        let synapse = Synapse {
            weight,
            tag_trace: 0.0,
            last_used_tick: self.current_tick,
        };

        let key = Self::encode_synapse_key(source_id, target_id);
        let bytes = bincode::encode_to_vec(&synapse, bincode::config::standard())
            .expect("Ошибка сериализации синапса");

        self.synapses
            .insert(key, bytes)
            .expect("Ошибка записи синапса в БД");
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

    fn process_impulse_to_neuron(&self, neuron_id: u64, charge: f32) -> bool {
        let mut key_bytes = Vec::with_capacity(8);
        use byteorder::{BigEndian, WriteBytesExt};
        key_bytes.write_u64::<BigEndian>(neuron_id).unwrap();

        if let Some(ivec) = self.neurons.get(&key_bytes).expect("Ошибка чтения нейрона")
        {
            let (mut neuron, _): (NeuronState, usize) = bincode::decode_from_slice(&ivec, bincode::config::standard())
                .expect("Ошибка десериализации нейрона");

            if neuron.receive_impulse(charge, self.current_tick) {
                let updated_bytes =
                    bincode::encode_to_vec(&neuron, bincode::config::standard()).unwrap();
                self.neurons.insert(key_bytes, updated_bytes).unwrap();
                return true;
            }

            let updated_bytes =
                bincode::encode_to_vec(&neuron, bincode::config::standard()).unwrap();
            self.neurons.insert(key_bytes, updated_bytes).unwrap();
        }
        false
    }

    /// ГЛОБАЛЬНЫЙ СИГНАЛ КРИТИКА (ДОФАМИН / ШТРАФ)
    /// Сканирует базу синапсов и корректирует веса тех путей, которые недавно использовались.
    pub fn apply_reinforcement(&mut self, is_success: bool) {
        let mut updated_synapses = Vec::new();

        // Проходим по ВСЕМ синапсам в базе данных
        for result in self.synapses.iter() {
            if let Ok((key_bytes, val_bytes)) = result {
                let (mut synapse, _): (Synapse, usize) = bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();
                    
                // Применяем ленивое угасание следа до текущего момента времени
                synapse.decay_tag_lazy(self.current_tick);

                // Если связь недавно проявляла активность (остался след tag_trace > 0)
                if synapse.tag_trace > 0.001 {
                    if is_success {
                        // Дофаминовое подкрепление успеха! Связь усиливается пропорционально ее следу
                        synapse.weight += synapse.tag_trace * LEARNING_RATE;
                        // Ограничиваем разумный максимум силы связи
                        if synapse.weight > 3.0 {
                            synapse.weight = 3.0;
                        }
                    } else {
                        // Наказание за ошибку! Связь ослабевает
                        synapse.weight -= synapse.tag_trace * PUNISH_RATE;
                        if synapse.weight < 0.0 {
                            synapse.weight = 0.0;
                        } // Не уходим в деструктивный минус
                    }

                    // Обнуляем след активности после того, как Критик его обработал
                    synapse.tag_trace = 0.0;

                    updated_synapses.push((key_bytes, synapse));
                }
            }
        }

        // Перезаписываем обновленные синапсы обратно в БД Sled
        for (key, synapse) in updated_synapses {
            let bytes = bincode::encode_to_vec(&synapse, bincode::config::standard()).unwrap();
            self.synapses.insert(key, bytes).unwrap();
        }
        let _ = self.db.flush();
    }

    pub fn tick(&mut self) {
        let mut current_tick_spikes = Vec::new();
        let mut next_spikes = Vec::new();
        let mut synapses_to_update = Vec::new();

        while let Some(pos) = self
            .event_queue
            .iter()
            .position(|e| e.target_tick <= self.current_tick)
        {
            let event = self.event_queue.remove(pos).unwrap();
            current_tick_spikes.push(event.neuron_id);

            let prefix = Self::encode_prefix_key(event.neuron_id);

            for result in self.synapses.scan_prefix(&prefix) {
                if let Ok((key_bytes, val_bytes)) = result {
                    let (mut synapse, _): (Synapse, usize) = bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();
                    
                    // ИСПРАВЛЕНИЕ: Импульс проходит через синапс -> взводим след активности (Tag)
                    synapse.trigger(self.current_tick);
                    synapses_to_update.push((key_bytes.clone(), synapse.clone()));

                    use byteorder::{BigEndian, ReadBytesExt};
                    let mut rdr = &key_bytes[8..16];
                    let target_id = rdr.read_u64::<BigEndian>().unwrap();

                    if self.process_impulse_to_neuron(target_id, synapse.weight) {
                        next_spikes.push(target_id);
                    }
                }
            }
        }

        // Записываем обновленные синапсы (со взведенными следами) обратно в БД
        for (key, synapse) in synapses_to_update {
            let bytes = bincode::encode_to_vec(&synapse, bincode::config::standard()).unwrap();
            self.synapses.insert(key, bytes).unwrap();
        }

        // Механизм чанкинга
        for &old_id in &self.previous_tick_spikes {
            for &new_id in &current_tick_spikes {
                if old_id != new_id {
                    let pair = (old_id, new_id);
                    let count = self.coincidence_tracker.entry(pair).or_insert(0);
                    *count += 1;

                    if *count == COINCIDENCE_THRESHOLD {
                        println!(
                            "\n[МЕТА-ЭВОЛЮЦИЯ]: Обнаружена устойчивая последовательность {} -> {}. Рождение нового понятия!",
                            old_id, new_id
                        );
                        let meta_neuron_id = self.create_neuron(NeuronType::Hidden);
                        self.set_synapse(old_id, meta_neuron_id, 1.2);
                        self.set_synapse(new_id, meta_neuron_id, 1.2);
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

        let _ = self.db.flush();
        self.current_tick += 1;
    }

    pub fn active_spikes_count(&self) -> usize {
        self.event_queue.len()
    }

    /// Вспомогательный метод инспекции веса синапса для тестов в main
    pub fn get_synapse_weight(&self, source_id: u64, target_id: u64) -> f32 {
        let key = Self::encode_synapse_key(source_id, target_id);
        if let Some(val_bytes) = self.synapses.get(&key).unwrap() {
            let (synapse, _): (Synapse, usize) =
                bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();
            synapse.weight
        } else {
            0.0
        }
    }
/// УМНЫЙ РЕЖИМ СНА (Контрастный синаптический прунинг)
    /// Увеличивает контраст памяти: сильные связи защищаются, слабые стираются лавинообразно.
    pub fn sleep_and_prune(&mut self) {
        println!("\n[КОНТРАСТНЫЙ СОН]: Анализ графа знаний и выжигание информационного шума...");
        
        // Порог, ниже которого синапс удаляется из БД Sled
        const DEATH_THRESHOLD: f32 = 0.2; 

        let mut synapses_to_remove = Vec::new();
        let mut synapses_to_update = Vec::new();
        let mut neuron_activity_counter: HashMap<u64, u32> = HashMap::new();

        // Статистика для вывода в консоль
        let mut untouched_strong = 0;
        let mut degraded_weak = 0;

        // --- ЭТАП 1: Нелинейное изменение весов синапсов ---
        for result in self.synapses.iter() {
            if let Ok((key_bytes, val_bytes)) = result {
                let (mut synapse, _): (Synapse, usize) =
                    bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();

                use byteorder::{BigEndian, ReadBytesExt};
                let mut source_rdr = &key_bytes[0..8];
                let source_id = source_rdr.read_u64::<BigEndian>().unwrap();
                let mut target_rdr = &key_bytes[8..16];
                let target_id = target_rdr.read_u64::<BigEndian>().unwrap();

                let old_weight = synapse.weight;

                // ДИНАМИЧЕСКИЙ РАСЧЕТ УГАДЫВАНИЯ (Контрастирование)
                if old_weight >= 1.5 {
                    // 1. Супер-сильная связь (Консолидированная память)
                    // Она почти не страдает во время сна — уменьшаем всего на 2%
                    synapse.weight *= 0.98;
                    untouched_strong += 1;
                } else if old_weight >= 0.8 {
                    // 2. Средняя связь (Пока не проверена временем)
                    // Уменьшаем умеренно — на 15%
                    synapse.weight *= 0.85;
                    degraded_weak += 1;
                } else {
                    // 3. Слабая связь (Вероятнее всего, случайный мусор от чанкинга)
                    // Вместо умножения жестко ВЫЧИТАЕМ большой штраф, чтобы она лавинообразно улетела в ноль
                    synapse.weight -= 0.25;
                    degraded_weak += 1;
                }

                if synapse.weight < DEATH_THRESHOLD {
                    // Синапс не перенес ночь — удаляем
                    synapses_to_remove.push(key_bytes);
                } else {
                    // Синапс выжил — фиксируем активность узлов графа
                    *neuron_activity_counter.entry(source_id).or_insert(0) += 1;
                    *neuron_activity_counter.entry(target_id).or_insert(0) += 1;
                    
                    synapses_to_update.push((key_bytes, synapse));
                }
            }
        }

        let removed_synapses_count = synapses_to_remove.len();
        
        // Записываем изменения синапсов в Sled
        for key in synapses_to_remove {
            self.synapses.remove(key).unwrap();
        }
        for (key, synapse) in synapses_to_update {
            let bytes = bincode::encode_to_vec(&synapse, bincode::config::standard()).unwrap();
            self.synapses.insert(key, bytes).unwrap();
        }

        println!("  -> Сильных связей сохранено: {}", untouched_strong);
        println!("  -> Слабых связей попало под сокращение: {}", degraded_weak);
        println!("  -> Полностью выжжено из БД синапсов: {}", removed_synapses_count);

        // --- ЭТАП 2: Сборка мусора изолированных Hidden-нейронов ---
        let mut neurons_to_remove = Vec::new();

        for result in self.neurons.iter() {
            if let Ok((key_bytes, val_bytes)) = result {
                let (neuron, _): (NeuronState, usize) = bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();

                if neuron.neuron_type == NeuronType::Hidden {
                    use byteorder::{BigEndian, ReadBytesExt};
                    let mut rdr = &key_bytes[0..8];
                    let neuron_id = rdr.read_u64::<BigEndian>().unwrap();

                    if !neuron_activity_counter.contains_key(&neuron_id) {
                        neurons_to_remove.push(key_bytes);
                    }
                }
            }
        }

        let removed_neurons_count = neurons_to_remove.len();
        for key in &neurons_to_remove {
            self.neurons.remove(key).unwrap();
        }

        println!("  -> Удалено изолированных мета-нейронов (информационный шум): {}", removed_neurons_count);
        println!("[КОНТРАСТНЫЙ СОН]: Очистка завершена. Контраст графа успешно увеличен.\n");
        
        let _ = self.db.flush();
    }    
}
