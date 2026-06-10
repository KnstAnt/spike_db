use crate::config::{BrainConfig, CONFIG};
use crate::models::{NeuronState, NeuronType, Synapse};
use bincode_next as bincode;
use byteorder::{BigEndian, WriteBytesExt};
use sled::{Db, Tree};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct SpikeEvent {
    pub neuron_id: u64,
    pub target_tick: u64,
}

pub struct SpikingNetwork {
    db: Db,
    pub neurons: Tree,
    pub synapses: Tree,
    pub vocabulary: Tree,
    pub current_tick: u64,
    event_queue: VecDeque<SpikeEvent>,
    next_neuron_id: AtomicU64, 
    previous_tick_spikes: Vec<u64>,
    coincidence_tracker: HashMap<(u64, u64), u32>,
}

impl SpikingNetwork {
    pub fn new(path: &str, config: BrainConfig) -> Self {
        let db = sled::open(path).expect("Не удалось открыть базу данных SpikeDB");
        let neurons = db.open_tree("neurons").expect("Ошибка дерева нейронов");
        let synapses = db.open_tree("synapses").expect("Ошибка дерева синапсов");
        let vocabulary = db.open_tree("vocabulary").expect("Ошибка дерева словаря");

        let _ = CONFIG.set(config);

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

    // =================================================================
    // ЛУЧШИЕ ПРАКТИКИ: ИНКАПСУЛЯЦИЯ РАБОТЫ С БАЙТАМИ (DRY)
    // =================================================================

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

    /// Декодирует одиночный u64 ID нейрона из Кey-Value ключа Sled
    fn decode_neuron_id(key: &[u8]) -> u64 {
        use byteorder::{BigEndian, ReadBytesExt};
        let mut rdr = &key[0..8];
        rdr.read_u64::<BigEndian>()
            .expect("Ошибка декодирования Neuron ID")
    }

    /// Декодирует составной ключ синапса и возвращает кортеж (source_id, target_id)
    fn decode_synapse_key(key: &[u8]) -> (u64, u64) {
        use byteorder::{BigEndian, ReadBytesExt};
        let mut source_rdr = &key[0..8];
        let source_id = source_rdr
            .read_u64::<BigEndian>()
            .expect("Ошибка декодирования Source ID");

        let mut target_rdr = &key[8..16];
        let target_id = target_rdr
            .read_u64::<BigEndian>()
            .expect("Ошибка декодирования Target ID");

        (source_id, target_id)
    }

    // =================================================================
    // ПРИКЛАДНАЯ ЛОГИКА И НЕЙРОМОРФНЫЕ ВЫЧИСЛЕНИЯ
    // =================================================================

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
                let bytes = bincode::encode_to_vec(&state, bincode::config::standard()).unwrap();
                self.neurons.insert(&key_bytes, bytes).unwrap();
                return id;
            }
        }
    }

    pub fn get_or_create_token_neuron(&self, token: &str) -> u64 {
        if let Some(ivec) = self
            .vocabulary
            .get(token.as_bytes())
            .expect("Ошибка словаря")
        {
            use byteorder::{BigEndian, ReadBytesExt};
            let mut rdr = &ivec[..];
            return rdr.read_u64::<BigEndian>().unwrap();
        }
        let new_id = self.create_neuron(NeuronType::Sensor);
        let mut id_bytes = Vec::with_capacity(8);
        use byteorder::BigEndian as BE;
        id_bytes.write_u64::<BE>(new_id).unwrap();
        self.vocabulary.insert(token.as_bytes(), id_bytes).unwrap();
        new_id
    }

    pub fn set_synapse(&self, source_id: u64, target_id: u64, weight: f32) {
        let synapse = Synapse {
            weight,
            tag_trace: 0.0,
            last_used_tick: self.current_tick,
        };
        let key = Self::encode_synapse_key(source_id, target_id);
        let bytes = bincode::encode_to_vec(&synapse, bincode::config::standard()).unwrap();
        self.synapses.insert(key, bytes).unwrap();
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

    /// Внутрибазовые вычисления (In-Database Computing) мембранного потенциала
    fn process_impulse_to_neuron(&self, neuron_id: u64, charge: f32) -> bool {
        let mut key_bytes = Vec::with_capacity(8);
        use byteorder::{BigEndian, WriteBytesExt};
        key_bytes.write_u64::<BigEndian>(neuron_id).unwrap();

        let mut spiked = false;
        let current_t = self.current_tick;

        let _ = self
            .neurons
            .update_and_fetch(&key_bytes, |old_bytes| {
                if let Some(bytes) = old_bytes {
                    let (mut neuron, _): (NeuronState, usize) =
                        bincode::decode_from_slice(bytes, bincode::config::standard()).unwrap();
                    if neuron.receive_impulse(charge, current_t) {
                        spiked = true;
                    }
                    let updated_bytes =
                        bincode::encode_to_vec(&neuron, bincode::config::standard()).unwrap();
                    Some(updated_bytes)
                } else {
                    None
                }
            })
            .expect("Ошибка атомарного обновления нейрона");

        spiked
    }

    /// Внутрибазовое дофаминовое подкрепление синапсов Критиком
    pub fn apply_reinforcement(&mut self, is_success: bool) {
        let current_t = self.current_tick;

        for result in self.synapses.iter() {
            if let Ok((key_bytes, _)) = result {
                let _ = self.synapses.update_and_fetch(&key_bytes, |old_bytes| {
                    if let Some(bytes) = old_bytes {
                        let (mut synapse, _): (Synapse, usize) =
                            bincode::decode_from_slice(bytes, bincode::config::standard()).unwrap();
                        synapse.decay_tag_lazy(current_t);

                        if synapse.tag_trace > 0.001 {
                            let cfg = CONFIG.get().expect("Конфиг не инициализирован");
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
                            let updated_bytes =
                                bincode::encode_to_vec(&synapse, bincode::config::standard())
                                    .unwrap();
                            Some(updated_bytes)
                        } else {
                            Some(bytes.to_vec())
                        }
                    } else {
                        None
                    }
                });
            }
        }
    }

    /// Контрастный сон и нелинейный синаптический прунинг
    pub fn sleep_and_prune(&mut self) {
        println!("\n[КОНТРАСТНЫЙ СОН]: Анализ графа знаний и выжигание информационного шума...");
        const DEATH_THRESHOLD: f32 = 0.2;
        let mut synapses_to_remove = Vec::new();
        let mut synapses_to_update = Vec::new();
        let mut neuron_activity_counter: HashMap<u64, u32> = HashMap::new();

        for result in self.synapses.iter() {
            if let Ok((key_bytes, val_bytes)) = result {
                let (mut synapse, _): (Synapse, usize) =
                    bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();

                // Извлекаем ID через чистый изолированный парсер
                let (source_id, target_id) = Self::decode_synapse_key(&key_bytes);
                let old_weight = synapse.weight;

                if old_weight >= 1.5 {
                    synapse.weight *= 0.98;
                } else if old_weight >= 0.8 {
                    synapse.weight *= 0.85;
                } else {
                    synapse.weight -= 0.25;
                }

                if synapse.weight < DEATH_THRESHOLD {
                    synapses_to_remove.push(key_bytes);
                } else {
                    *neuron_activity_counter.entry(source_id).or_insert(0) += 1;
                    *neuron_activity_counter.entry(target_id).or_insert(0) += 1;
                    synapses_to_update.push((key_bytes, synapse));
                }
            }
        }

        for key in synapses_to_remove {
            self.synapses.remove(key).unwrap();
        }
        for (key, synapse) in synapses_to_update {
            let bytes = bincode::encode_to_vec(&synapse, bincode::config::standard()).unwrap();
            self.synapses.insert(key, bytes).unwrap();
        }

        let mut neurons_to_remove = Vec::new();
        for result in self.neurons.iter() {
            if let Ok((key_bytes, val_bytes)) = result {
                let (neuron, _): (NeuronState, usize) =
                    bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();
                if neuron.neuron_type == NeuronType::Hidden {
                    let neuron_id = Self::decode_neuron_id(&key_bytes);
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
        println!(
            "  -> Удалено изолированных мета-нейронов: {}\n[КОНТРАСТНЫЙ СОН]: Очистка завершена.\n",
            removed_neurons_count
        );
    }

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
            let prefix = Self::encode_prefix_key(event.neuron_id);
            let current_t = self.current_tick;
            for result in self.synapses.scan_prefix(&prefix) {
                if let Ok((key_bytes, _)) = result {
                    let mut current_weight = 0.0;
                    let _ = self
                        .synapses
                        .update_and_fetch(&key_bytes, |old_bytes| {
                            if let Some(bytes) = old_bytes {
                                let (mut synapse, _): (Synapse, usize) =
                                    bincode::decode_from_slice(bytes, bincode::config::standard())
                                        .unwrap();
                                synapse.trigger(current_t);
                                current_weight = synapse.weight;
                                let updated_bytes =
                                    bincode::encode_to_vec(&synapse, bincode::config::standard())
                                        .unwrap();
                                Some(updated_bytes)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    let (_, target_id) = Self::decode_synapse_key(&key_bytes);
                    if self.process_impulse_to_neuron(target_id, current_weight) {
                        next_spikes.push(target_id);
                    }
                }
            }
        }
        for &old_id in &self.previous_tick_spikes {
            for &new_id in &current_tick_spikes {
                if old_id != new_id {
                    let pair = (old_id, new_id);
                    let count = self.coincidence_tracker.entry(pair).or_insert(0);
                    *count += 1;
                    if *count == cfg.coincidence_threshold {
                        println!("\n[МЕТА-ЭВОЛЮЦИЯ]: Обнаружена устойчивая последовательность {} -> {}. Рождение нового понятия!", old_id, new_id);
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
    pub fn get_strongest_prediction(&self, source_id: u64) -> Option<(u64, f32)> {
        let prefix = Self::encode_prefix_key(source_id);
        let mut strongest_target = None;
        let mut max_weight = -1.0;
        for result in self.synapses.scan_prefix(&prefix) {
            if let Ok((key_bytes, val_bytes)) = result {
                let (synapse, _): (Synapse, usize) =
                    bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();
                if synapse.weight > max_weight {
                    max_weight = synapse.weight;
                    let (_, target_id) = Self::decode_synapse_key(&key_bytes);
                    strongest_target = Some((target_id, synapse.weight));
                }
            }
        }
        strongest_target
    }
}
