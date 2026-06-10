use spikedb::network::SpikingNetwork;
use spikedb::models::NeuronType;
use spikedb::config::BrainConfig;

#[test]
fn test_basic_lif_propagation() {
    println!("\n=== [CORE TEST]: Проверка LIF-динамики в ОЗУ ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());

    // ИСПРАВЛЕНИЕ: Используем каноничные методы get_or_create_token
    let s = net.memory.get_or_create_token("Sensor_A");
    let m = net.memory.get_or_create_token("Motor_A");

    // Прокладываем связь
    net.memory.set_synapse(s, m, 1.2);
    
    // Подаем импульс
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

    // ИСПРАВЛЕНИЕ: Используем правильные методы создания токенов
    let token_a = net.memory.get_or_create_token("A");
    let token_b = net.memory.get_or_create_token("B");

    // Повторяем последовательность A -> B для срабатывания совпадения
    for _ in 0..3 {
        net.inject_stimulus(token_a, 1.2);
        net.tick();
        net.inject_stimulus(token_b, 1.2);
        net.tick();
        while net.active_spikes_count() > 0 { net.tick(); }
    }

    // Чанкинг должен вырастить скрытый нейрон с ID 2 на основе связи (0, 1)
    assert!(net.memory.neurons.len() > 2, "Подсистема чанкинга должна автоматически создать Мета-Нейрон ID 2");
    
    // Проверяем тип и правильность записи происхождения
    let meta_neuron = &net.memory.neurons[2];
    assert_eq!(meta_neuron.neuron_type, NeuronType::Hidden);
}

#[test]
fn test_critic_reinforcement() {
    println!("\n=== [CORE TEST]: Проверка дофаминового подкрепления Критика ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());

    // ИСПРАВЛЕНИЕ: Создаем нейроны через правильный интерфейс токенов
    let s = net.memory.get_or_create_token("Hidden_Node");
    let m = net.memory.get_or_create_token("Motor_Node");

    net.memory.set_synapse(s, m, 0.5);
    net.inject_stimulus(s, 1.0);
    net.tick();

    // Критик подкрепляет связи
    net.apply_reinforcement(true);

    // Проверяем вес (немутабельный метод чтения)
    let updated_weight = net.get_synapse_weight(s, m);
    assert!(updated_weight > 0.5, "Критик должен был увеличить базовый вес связи");
}
