use spikedb::config::BrainConfig;
use spikedb::database_manager::SpikeDB;
use spikedb::models::NeuronType;
use spikedb::network::SpikingNetwork;

// =================================================================
// 1. LIF-ДИНАМИКА И ЗАТУХАНИЕ ПОТЕНЦИАЛА (LEAK & COOLDOWN)
// =================================================================

#[test]
fn test_positive_lif_threshold_and_cooldown() {
    println!("\n=== [LIF+]: Проверка пробития порога и кулдауна ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let neuron = net.memory.get_or_create_token("LIF_Target");

    net.inject_stimulus(neuron, 1.0);
    assert_eq!(
        net.active_spikes_count(),
        1,
        "Нейрон должен был пробить порог!"
    );

    net.tick();
    net.clear_runtime_attention_buffers();

    let cfg = spikedb::config::CONFIG.get().unwrap();
    if let Some(n) = net.memory.neurons.get_mut(neuron as usize) {
        // Импульс во время глубокого рефрактерного отдыха ОБЯЗАН быть отвергнут!
        let fired = n.receive_impulse(
            2.0,
            net.current_tick,
            cfg.leak_tau,
            cfg.spike_threshold,
            cfg.cooldown_ticks,
        );
        assert!(
            !fired,
            "Отрицательный сценарий сломан: нейрон сработал во время кулдауна!"
        );
    }
}

#[test]
fn test_negative_lif_leakage_over_time() {
    println!("\n=== [LIF-]: Проверка утечки мембранного потенциала ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let neuron = net.memory.get_or_create_token("Leak_Target");

    net.inject_stimulus(neuron, 0.5);
    net.clear_runtime_attention_buffers();

    // Продвигаем время симуляции на 50 тиков вхолостую
    for _ in 0..50 {
        net.tick();
    }

    let cfg = spikedb::config::CONFIG.get().unwrap();
    if let Some(n) = net.memory.neurons.get_mut(neuron as usize) {
        // Из-за честного пассивного остывания старые 0.5 утекли, спайка быть не должно!
        let fired = n.receive_impulse(
            0.5,
            net.current_tick,
            cfg.leak_tau,
            cfg.spike_threshold,
            cfg.cooldown_ticks,
        );
        assert!(
            !fired,
            "Отрицательный сценарий сломан: потенциал не утек, и нейрон ошибочно выдал спайк!"
        );
    }
}

#[test]
fn test_positive_global_relaxation_contract() {
    println!("\n=== [RELAXATION+]: Проверка соблюдения исходного контракта релаксации ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let neuron = net.memory.get_or_create_token("Relax_Target");

    net.inject_stimulus(neuron, 0.8);
    net.clear_runtime_attention_buffers();

    for _ in 0..50 {
        net.tick();
    }

    if let Some(n) = net.memory.neurons.get(neuron as usize) {
        println!(
            "  -> [ИНСПЕКЦИЯ]: Потенциал после 50 тиков релаксации: {:.4}",
            n.potential
        );
        assert!(
            n.potential < 0.1,
            "КРИТИЧЕСКИЙ СЛОМ: Нейрон застрял на старом потенциале без релаксации!"
        );
        assert_eq!(
            n.last_updated_tick,
            net.current_tick - 1,
            "Часы нейрона рассинхронизировались с сетью!"
        );
    }
}

// =================================================================
// 2. ХРОНОЛОГИЧЕСКИЙ ЧАНКИНГ И СИНТАКСИЧЕСКОЕ ВЕТО
// =================================================================

#[test]
fn test_positive_pure_sensory_chunking() {
    println!("\n=== [CHUNK+]: Проверка рождения базового чанка ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    net.set_learning_mode(true); // Включаем затвор пластичности
    let a = net.memory.get_or_create_token("let");
    let b = net.memory.get_or_create_token("var");

    let old_neurons_count = net.memory.neurons.len();

    for _ in 0..10 {
        net.inject_stimulus(a, 1.2);
        net.tick();
        net.inject_stimulus(b, 1.2);
        net.tick();
        net.clear_runtime_attention_buffers();
    }

    assert!(
        net.memory.neurons.len() > old_neurons_count,
        "Чанкинг должен был вырастить мета-нейрон!"
    );
}

#[test]
fn test_negative_meta_chunk_explosion_filter() {
    println!("\n=== [CHUNK-]: Защита от комбинаторного скрещивания чанков ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    net.set_learning_mode(true);

    let meta_a = net.memory.create_meta_chunk(0, 1);
    let meta_b = net.memory.create_meta_chunk(2, 3);

    let old_neurons_count = net.memory.neurons.len();

    for _ in 0..15 {
        net.inject_stimulus(meta_a, 1.2);
        net.tick();
        net.inject_stimulus(meta_b, 1.2);
        net.tick();
        net.clear_runtime_attention_buffers();
    }

    assert_eq!(
        net.memory.neurons.len(),
        old_neurons_count,
        "Отрицательный сценарий сломан: Скрытые мета-нейроны смогли скреститься!"
    );
}

// =================================================================
// 3. АТОМАРНАЯ КРИТИКА И ЭКСПОРЕНЦИАЛЬНОЕ ЗАТУХАНИЕ (CRITIC & TAGS)
// =================================================================

#[test]
fn test_positive_immediate_reinforcement() {
    println!("\n=== [CRITIC+]: Проверка мгновенного дофаминового подкрепления ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let s = net.memory.get_or_create_token("Source");
    let t = net.memory.get_or_create_token("Target");

    net.memory.set_synapse(s, t, 0.5, net.current_tick);
    net.apply_reinforcement(true);

    let weight = net.get_synapse_weight(s, t);
    assert!(
        weight > 0.5,
        "Критик обязан был увеличить вес горячего синапса!"
    );
}

#[test]
fn test_negative_reinforcement_of_decayed_tags() {
    println!("\n=== [CRITIC-]: Проверка игнорирования остывших синапсов ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let s = net.memory.get_or_create_token("OldSource");
    let t = net.memory.get_or_create_token("OldTarget");

    net.memory.set_synapse(s, t, 0.5, net.current_tick);

    for _ in 0..200 {
        net.tick();
    }

    let old_weight = net.get_synapse_weight(s, t);
    net.apply_reinforcement(true);

    let new_weight = net.get_synapse_weight(s, t);
    assert_eq!(
        old_weight, new_weight,
        "Критик ошибочно подкрепил остывший синапс!"
    );
}

// =================================================================
// 4. КОНТРАСТНЫЙ СОН И СИНАПТИЧЕСКИЙ ГОМЕОСТАЗ (SLEEP & PRUNING)
// =================================================================

#[test]
fn test_positive_sleep_and_prune_contrast() {
    println!("\n=== [SLEEP+]: Проверка выжигания мусорных связей во сне ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let s = net.memory.get_or_create_token("Fn");
    let correct = net.memory.get_or_create_token("Correct");
    let noise = net.memory.get_or_create_token("Noise");

    net.memory.set_synapse(s, correct, 1.6, net.current_tick);
    net.memory.set_synapse(s, noise, 0.5, net.current_tick);

    net.sleep_and_prune();

    let strong_weight = net.get_synapse_weight(s, correct);
    let weak_weight = net.get_synapse_weight(s, noise);

    assert!(
        strong_weight > 1.0,
        "Сильная связь должна была пережить сон!"
    );
    assert_eq!(
        weak_weight, 0.0,
        "Слабый синаптический шум не был уничтожен сном!"
    );
}

#[test]
fn test_negative_critic_synaptic_divergence_protection() {
    println!("\n=== [CRITIC-]: Краш-тест защиты от лавинного раздувания веера ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let s = net.memory.get_or_create_token("HyperSource");

    for i in 0..20 {
        let t = net
            .memory
            .get_or_create_token(&format!("StormTarget_{}", i));
        net.memory.set_synapse(s, t, 0.5, net.current_tick);
    }

    if let Some(links) = net.memory.adj_list.get_mut(s as usize) {
        for synapse in links.iter_mut() {
            synapse.tag_trace = 1.0;
        }
    }

    net.apply_reinforcement(true);

    let final_links_count = net.memory.adj_list[s as usize].len();
    let cfg = spikedb::config::CONFIG.get().unwrap();

    assert!(
        final_links_count <= cfg.max_synaptic_fanout,
        "КАТАСТРОФА: Критик пропустил лавинное раздувание веера синапсов!"
    );
}

// =================================================================
// 5. АСИНХРОННАЯ МНОГОПОТОЧНАЯ ИНТЕГРАЦИЯ И СТРЕСС-БАРЬЕРЫ
// =================================================================

#[test]
fn test_pure_async_lif_threshold_and_cooldown() {
    println!("\n=== [LIF+ TEST]: Асинхронная проверка через Актёра ===");
    let db = SpikeDB::open("sandbox_lif_cooldown");

    for _ in 0..10 {
        let line = vec!["fn".to_string(), "correct_name".to_string()];
        db.inject_string_context(line, 1.2, Some(true));
    }
    db.wait_flush_barrier();

    let res = db.generate_code_hypothesis("fn", vec![]);
    assert!(
        res.contains(&"correct_name".to_string()),
        "Порог не пробился или синапс не укрепился!"
    );

    let duplicate_line = vec!["fn".to_string(), "correct_name".to_string()];
    db.inject_string_context(duplicate_line, 1.2, Some(true));
    db.wait_flush_barrier();

    db.trigger_sleep();
    db.wait_flush_barrier();
}

#[test]
fn test_negative_actor_inter_sentence_avalanche_protection() {
    println!("\n=== [OVERLOAD- TEST]: Абсолютный краш-тест резонансного шторма словаря ===");
    let db = SpikeDB::open("sandbox_avalanche_cyclotron_real");

    println!("  -> [ЦИКЛОТРОН]: Запуск лавинной бомбардировки перекрестного графа...");
    for i in 0..300 {
        let line_fn = vec![
            "fn".to_string(),
            format!("func_{}", i),
            "(".to_string(),
            ")".to_string(),
            "->".to_string(),
            "bool".to_string(),
            "{".to_string(),
        ];
        let line_let = vec![
            "let".to_string(),
            format!("var_{}", i),
            ":".to_string(),
            "u32".to_string(),
            "=".to_string(),
            format!("{}", i),
            ";".to_string(),
        ];
        let line_if = vec![
            "if".to_string(),
            format!("var_{}", i),
            "==".to_string(),
            "0".to_string(),
            "{".to_string(),
            "}".to_string(),
        ];

        db.inject_string_context(line_fn, 1.5, Some(true));
        db.inject_string_context(line_let, 1.5, Some(true));
        db.inject_string_context(line_if, 1.5, Some(true));
    }
    db.wait_flush_barrier();

    let total_ticks = db.get_current_kernel_tick();
    println!(
        "  -> [ИНСПЕКЦИЯ]: После шторма сеть намотала ТИКА(ОВ): {}",
        total_ticks
    );

    assert!(
        total_ticks < 8000,
        "КАТАСТРОФА: Сеть ушла в бесконечный шторм! Намотано {} тиков вместо положенных ~6000!",
        total_ticks
    );
}

#[test]
fn test_negative_global_relaxation_contract() {
    println!("\n============================================================");
    println!("=== [RELAXATION- TEST]: Контроль пассивного остывания мембраны ===");
    println!("============================================================");
    
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let neuron = net.memory.get_or_create_token("Relax_Target");

    net.inject_stimulus(neuron, 0.8);
    net.event_queue.clear();

    if let Some(n) = net.memory.neurons.get(neuron as usize) {
        println!("  -> [СТАРТ]: Идентификатор токена Relax_Target = {}", neuron);
        println!("  -> [СТАРТ]: Начальный потенциал нейрона = {}", n.potential);
        assert_eq!(n.potential, 0.8, "Заряд не лег на мембрану на старте!");
    }

    println!("  -> [ПАУЗА]: Запуск 20 циклов net.tick()...");
    for step in 1..=20 {
        println!("    --- Вызов net.tick() №{} ---", step);
        net.tick();
        
        // Инспектируем состояние прямо внутри цикла
        if let Some(n) = net.memory.neurons.get(neuron as usize) {
            println!("        Состояние после шага {}: Потенциал = {}, Last_Updated_Tick = {}", 
                step, n.potential, n.last_updated_tick);
        }
    }

    if let Some(n) = net.memory.neurons.get(neuron as usize) {
        println!("  -> [ФИНИШ]: Потенциал нейрона на тике {} = {}", net.current_tick, n.potential);
        assert!(n.potential < 0.15, "Критический слом релаксации потенциала!");
        assert_eq!(n.last_updated_tick, net.current_tick, "Часы нейрона рассинхронизировались с часами сети!");
    }
    println!("============================================================");
}
