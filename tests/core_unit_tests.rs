use spikedb::network::SpikingNetwork;
use spikedb::models::NeuronType;
use spikedb::config::BrainConfig;

#[test]
fn test_basic_lif_propagation() {
    println!("\n=== [CORE TEST]: Проверка LIF-динамики в ОЗУ ===");
    
    // ИСПРАВЛЕНИЕ: Новый конструктор принимает только конфиг, без пути к файлу!
    let mut net = SpikingNetwork::new(BrainConfig::default());

    // Выращиваем нейроны в памяти через обращение к SpikeMemory (.memory)
    let s = net.memory.create_neuron(NeuronType::Sensor);
    let m = net.memory.create_neuron(NeuronType::Motor);

    // Прокладываем связь
    net.memory.set_synapse(s, m, 1.2);
    
    // Подаем импульс (мутабельный метод &mut self)
    net.inject_stimulus(s, 1.2);
    assert_eq!(net.active_spikes_count(), 1, "Сенсор должен породить активный спайк");

    // Шаг времени симуляции
    net.tick();
    assert_eq!(net.active_spikes_count(), 1, "Моторный нейрон должен перехватить импульс");
}

#[test]
fn test_chunking_evolution() {
    println!("\n=== [CORE TEST]: Проверка мета-эволюции (Чанкинга) в ОЗУ ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());

    let token_a = net.memory.create_neuron(NeuronType::Sensor);
    let token_b = net.memory.create_neuron(NeuronType::Sensor);

    // Повторяем последовательность A -> B для срабатывания совпадения
    for _ in 0..3 {
        net.inject_stimulus(token_a, 1.2);
        net.tick();
        net.inject_stimulus(token_b, 1.2);
        net.tick();
        while net.active_spikes_count() > 0 { net.tick(); }
    }

    // В нашей новой In-Memory структуре нейроны лежат в плоском Vec.
    // Изначально были созданы ID 0 и 1. Чанкинг должен вырастить скрытый нейрон с ID 2!
    assert!(net.memory.neurons.len() > 2, "Подсистема чанкинга должна автоматически создать Мета-Нейрон ID 2");
    
    // Проверяем, что тип рожденного нейрона — Hidden
    assert_eq!(net.memory.neurons[2].neuron_type, NeuronType::Hidden);
}

#[test]
fn test_critic_reinforcement() {
    println!("\n=== [CORE TEST]: Проверка дофаминового подкрепления Критика ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());

    let s = net.memory.create_neuron(NeuronType::Hidden);
    let m = net.memory.create_neuron(NeuronType::Motor);

    net.memory.set_synapse(s, m, 0.5);
    net.inject_stimulus(s, 1.0);
    net.tick();

    // Критик подкрепляет связи (мутабельный метод)
    net.apply_reinforcement(true);

    // Проверяем вес (немутабельный метод чтения)
    let updated_weight = net.get_synapse_weight(s, m);
    assert!(updated_weight > 0.5, "Критик должен был увеличить базовый вес связи");
}
