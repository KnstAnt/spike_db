use crate::config::{BrainConfig, CONFIG};
use crate::models::{NeuronState, NeuronType, Synapse};
use bincode_next as bincode;
use byteorder::BigEndian as BE;
use byteorder::{ReadBytesExt, WriteBytesExt};
use sled::{Db, Tree};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering}; // Импортируем CONFIG из модуля config

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
    pub fn new(path: &str, config: BrainConfig) -> Self {
        let db = sled::open(path).expect("Не удалось открыть базу данных SpikeDB");
        let neurons = db.open_tree("neurons").expect("Ошибка дерева нейронов");
        let synapses = db.open_tree("synapses").expect("Ошибка дерева синапсов");
        let vocabulary = db.open_tree("vocabulary").expect("Ошибка дерева словаря");

        // Инициализируем глобальный OnceLock
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

    fn encode_synapse_key(source_id: u64, target_id: u64) -> Vec<u8> {
        let mut wtr = Vec::with_capacity(16);
        wtr.write_u64::<BE>(source_id).unwrap();
        wtr.write_u64::<BE>(target_id).unwrap();
        wtr
    }

    fn encode_prefix_key(source_id: u64) -> Vec<u8> {
        let mut wtr = Vec::with_capacity(8);
        wtr.write_u64::<BE>(source_id).unwrap();
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

    fn process_impulse_to_neuron(&self, neuron_id: u64, charge: f32) -> bool {
        let mut key_bytes = Vec::with_capacity(8);
        use byteorder::{BigEndian, WriteBytesExt};
        key_bytes.write_u64::<BigEndian>(neuron_id).unwrap();

        if let Some(ivec) = self.neurons.get(&key_bytes).expect("Ошибка чтения нейрона")
        {
            let (mut neuron, _): (NeuronState, usize) =
                bincode::decode_from_slice(&ivec, bincode::config::standard()).unwrap();

            // ИСПРАВЛЕНИЕ: Никаких лишних параметров, метод сам возьмет CONFIG из памяти!
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

    pub fn apply_reinforcement(&mut self, is_success: bool) {
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
        let mut updated_synapses = Vec::new();

        for result in self.synapses.iter() {
            if let Ok((key_bytes, val_bytes)) = result {
                let (mut synapse, _): (Synapse, usize) =
                    bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();
                synapse.decay_tag_lazy(self.current_tick);

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
                    updated_synapses.push((key_bytes, synapse));
                }
            }
        }
        for (key, synapse) in updated_synapses {
            let bytes = bincode::encode_to_vec(&synapse, bincode::config::standard()).unwrap();
            self.synapses.insert(key, bytes).unwrap();
        }
    }

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

                use byteorder::{BigEndian, ReadBytesExt};
                let mut source_rdr = &key_bytes[0..8];
                let source_id = source_rdr.read_u64::<BigEndian>().unwrap();
                let mut target_rdr = &key_bytes[8..16];
                let target_id = target_rdr.read_u64::<BigEndian>().unwrap();

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
        println!(
            "  -> Удалено изолированных мета-нейронов: {}\n[КОНТРАСТНЫЙ СОН]: Очистка завершена.\n",
            removed_neurons_count
        );
    }

    pub fn tick(&mut self) {
        let cfg = CONFIG.get().expect("Конфиг не инициализирован");
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
                    let (mut synapse, _): (Synapse, usize) =
                        bincode::decode_from_slice(&val_bytes, bincode::config::standard())
                            .unwrap();
                    synapse.trigger(self.current_tick);
                    synapses_to_update.push((key_bytes.clone(), synapse.clone()));
                    let mut rdr = &key_bytes[8..16];
                    let target_id = rdr.read_u64::<BE>().unwrap();
                    if self.process_impulse_to_neuron(target_id, synapse.weight) {
                        next_spikes.push(target_id);
                    }
                }
            }
        }
        for (key, synapse) in synapses_to_update {
            let bytes = bincode::encode_to_vec(&synapse, bincode::config::standard()).unwrap();
            self.synapses.insert(key, bytes).unwrap();
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
    /// ИНСПЕКЦИЯ МЫШЛЕНИЯ: Находит ID нейрона, с которым у данного нейрона самая сильная связь в БД.
    /// Возвращает ID цели и вес этой связи.
    pub fn get_strongest_prediction(&self, source_id: u64) -> Option<(u64, f32)> {
        let prefix = Self::encode_prefix_key(source_id);
        let mut strongest_target = None;
        let mut max_weight = -1.0;

        for result in self.synapses.scan_prefix(&prefix) {
            if let Ok((key_bytes, val_bytes)) = result {
                let (synapse, _): (Synapse, usize) = bincode::decode_from_slice(&val_bytes, bincode::config::standard()).unwrap();
                
                if synapse.weight > max_weight {
                    max_weight = synapse.weight;
                    
                    use byteorder::{BigEndian, ReadBytesExt};
                    let mut rdr = &key_bytes[8..16];
                    let target_id = rdr.read_u64::<BigEndian>().unwrap();
                    strongest_target = Some((target_id, synapse.weight));
                }
            }
        }
        strongest_target
    }    
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn get_test_db_path() -> String {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("./test_spikedb_{}", ts)
    }
    #[test]
    fn test_basic_lif_propagation() {
        let path = get_test_db_path();
        let mut net = SpikingNetwork::new(&path, BrainConfig::default());
        let s = net.create_neuron(NeuronType::Sensor);
        let m = net.create_neuron(NeuronType::Motor);
        net.set_synapse(s, m, 1.2);
        net.inject_stimulus(s, 1.2);
        assert_eq!(net.active_spikes_count(), 1);
        net.tick();
        assert_eq!(net.active_spikes_count(), 1);
        drop(net);
        let _ = fs::remove_dir_all(&path);
    }
    #[test]
    fn test_chunking_evolution() {
        let path = get_test_db_path();
        let mut net = SpikingNetwork::new(&path, BrainConfig::default());
        let token_a = net.create_neuron(NeuronType::Sensor);
        let token_b = net.create_neuron(NeuronType::Sensor);
        for _ in 0..3 {
            net.inject_stimulus(token_a, 1.2);
            net.tick();
            net.inject_stimulus(token_b, 1.2);
            net.tick();
            while net.active_spikes_count() > 0 {
                net.tick();
            }
        }
        let mut key_bytes = Vec::with_capacity(8);
        key_bytes.write_u64::<BE>(2).unwrap();
        let meta_exists = net.neurons.contains_key(&key_bytes).unwrap();
        assert!(meta_exists);
        drop(net);
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn test_critic_reinforcement() {
        let path = get_test_db_path();
        let mut net = SpikingNetwork::new(&path, BrainConfig::default());
        let s = net.create_neuron(NeuronType::Hidden);
        let m = net.create_neuron(NeuronType::Motor);
        net.set_synapse(s, m, 0.5);
        net.inject_stimulus(s, 1.0);
        net.tick();
        net.apply_reinforcement(true);
        let updated_weight = net.get_synapse_weight(s, m);
        assert!(updated_weight > 0.5);
        drop(net);
        let _ = fs::remove_dir_all(&path);
    }
}
