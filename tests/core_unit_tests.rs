use spikedb::network::SpikingNetwork;
use spikedb::models::NeuronType;
use spikedb::config::BrainConfig;

#[test]
fn test_basic_lif_propagation() {
    println!("\n=== [CORE TEST]: Проверка LIF-динамики в ОЗУ ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());

    let s = net.memory.get_or_create_token("Sensor_A");
    let m = net.memory.get_or_create_token("Motor_A");

    // ИСПРАВЛЕНИЕ: Передаем net.current_tick (0) четвертым аргументом!
    net.memory.set_synapse(s, m, 1.2, net.current_tick);
    
    net.inject_stimulus(s, 1.2);
    assert_eq!(net.active_spikes_count(), 1, "Сенсор должен породить активный спайк");

    net.tick();
    assert_eq!(net.active_spikes_count(), 1, "Motor-нейрон должен перехватить импульс");
}

#[test]
fn test_chunking_evolution() {
    println!("\n=== [CORE TEST]: Проверка мета-эволюции (Чанкинга) в ОЗУ ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());

    let token_a = net.memory.get_or_create_token("A");
    let token_b = net.memory.get_or_create_token("B");

    // Повторяем последовательность A -> B для срабатывания совпадения.
    // С учетом нашего фикса, теперь авто-тик (или ручной сброс) выстраивает их во времени.
    for _ in 0..10 {
        net.inject_stimulus(token_a, 1.2);
        net.tick();
        net.inject_stimulus(token_b, 1.2);
        net.tick();
        while net.active_spikes_count() > 0 { net.tick(); }
        net.clear_runtime_attention_buffers(); // Изолируем итерации
    }

    assert!(net.memory.neurons.len() > 2, "Подсистема чанкинга должна автоматически создать Мета-Нейрон");
    
    // Проверяем тип родившегося нейрона
    let meta_neuron = &net.memory.neurons[2];
    assert_eq!(meta_neuron.neuron_type, NeuronType::Hidden);
}

#[test]
fn test_critic_reinforcement() {
    println!("\n=== [CORE TEST]: Проверка дофаминового подкрепления Критика ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());

    let s = net.memory.get_or_create_token("Hidden_Node");
    let m = net.memory.get_or_create_token("Motor_Node");

    // ИСПРАВЛЕНИЕ: Передаем net.current_tick
    net.memory.set_synapse(s, m, 0.5, net.current_tick);
    net.inject_stimulus(s, 1.0);
    net.tick();

    net.apply_reinforcement(true);

    let updated_weight = net.get_synapse_weight(s, m);
    assert!(updated_weight > 0.5, "Критик должен был увеличить базовый вес связи");
}
