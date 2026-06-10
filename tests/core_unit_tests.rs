use spikedb::network::SpikingNetwork;
use spikedb::models::NeuronType;
use spikedb::config::BrainConfig;
use std::fs;

// Вспомогательная функция для генерации чистого пути тестовой БД
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
    assert_eq!(net.active_spikes_count(), 1, "Сенсор должен породить активный спайк");

    net.tick();
    assert_eq!(net.active_spikes_count(), 1, "Моторный нейрон должен перехватить импульс");

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
        while net.active_spikes_count() > 0 { net.tick(); }
    }

    // Проверяем, создался ли мета-нейрон.
    // Ключом в Sled является u64 в Big-Endian байтах
    let mut key_bytes = Vec::with_capacity(8);
    use byteorder::{BigEndian, WriteBytesExt};
    key_bytes.write_u64::<BigEndian>(2).unwrap();

    let meta_exists = net.neurons.contains_key(&key_bytes).unwrap();
    assert!(meta_exists, "Подсистема чанкинга должна автоматически создать Мета-Нейрон ID 2");

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
    assert!(updated_weight > 0.5, "Критик должен увеличить базовый вес связи");

    drop(net);
    let _ = fs::remove_dir_all(&path);
}
