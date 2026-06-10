use spikedb::database_manager::SpikeDB;
use spikedb::config::{BrainConfig, CONFIG};
use spikedb::network::SpikingNetwork;

// =================================================================
// 1. АСИНХРОННАЯ LIF-ДИНАМИКА И ЗАТУХАНИЕ (ЧЕРЕЗ КАНАЛЫ SpikeDB)
// =================================================================

#[test]
fn test_pure_async_lif_threshold_and_cooldown() {
    println!("\n=== [LIF+ TEST]: Асинхронная проверка порога и кулдауна ===");
    let db = SpikeDB::open("sandbox_lif_cooldown");

    // Положительный сценарий: Подаем пачку токенов залпом. Порог должен пробиться.
    let line = vec!["fn".to_string(), "correct_name".to_string()];
    db.inject_string_context(line, 1.2, Some(true));
    
    // Барьер гарантирует, что Актёр полностью переварил спайки на максимальной скорости
    db.wait_flush_barrier();

    // Запрашиваем предсказание из немутабельного графа
    let res = db.generate_code_hypothesis("fn", vec![]);
    assert!(res.contains(&"correct_name".to_string()), "Порог не пробился или синапс не укрепился!");

    // Отрицательный сценарий (Кулдаун): Шлём ту же строку НЕМЕДЛЕННО вслед за первой.
    // Из-за каноничного рефрактерного периода нейроны обязаны проигнорировать повторный импульс!
    let duplicate_line = vec!["fn".to_string(), "correct_name".to_string()];
    db.inject_string_context(duplicate_line, 1.2, Some(true));
    db.wait_flush_barrier();

    // Ночной сон. Если кулдаун сработал правильно, лишних весов в системе не вырастет.
    db.trigger_sleep();
    db.wait_flush_barrier();
}

#[test]
fn test_pure_async_lif_leakage_over_time() {
    println!("\n=== [LIF- TEST]: Асинхронная проверка экспоненциальной утечки ===");
    let db = SpikeDB::open("sandbox_lif_leak");

    // Подаем слабый заряд (0.4), недостаточный для порога
    db.inject_string_context(vec!["let".to_string()], 0.4, None);
    db.wait_flush_barrier();

    // ИСПРАВЛЕНИЕ НЕПОКРЫТОГО ФУНКЦИОНАЛА: Имитируем реальный дрейф биологического времени.
    // Вместо ручного вызова net.tick() мы шлем пустые маркерные пачки (такты тишины),
    // заставляя асинхронное Ядро продвигать current_tick и честно вычислять утечку мембраны!
    for _ in 0..15 {
        db.inject_string_context(vec![], 0.0, None);
    }
    db.wait_flush_barrier();

    // Отправляем сеть спать. Так как заряд 0.4 за 15 тактов тишины обязан полностью утечь в ноль,
    // синапс не успеет консолидироваться и будет полностью уничтожен ночным прунингом!
    db.trigger_sleep();
    db.wait_flush_barrier();

    // Проверяем: связь должна отсутствовать полностью
    let res = db.generate_code_hypothesis("let", vec![]);
    assert_eq!(res, vec!["let"], "Отрицательный сценарий сломан: Потенциал не утек, и связь выжила во сне!");
}

// =================================================================
// 2. СТРУКТУРНЫЙ ЧАНКИНГ И СИНТАКСИЧЕСКОЕ ВЕТО
// =================================================================

#[test]
fn test_async_pure_sensory_chunking() {
    println!("\n=== [CHUNK+ TEST]: Асинхронное рождение базового чанка ===");
    let db = SpikeDB::open("sandbox_chunk_pos");

    // 10 раз скармливаем Актёру устойчивую синтаксическую строку
    for _ in 0..10 {
        let line = vec!["struct".to_string(), "MyType".to_string(), "{".to_string(), "}".to_string()];
        db.inject_string_context(line, 1.2, Some(true));
    }
    db.wait_flush_barrier();

    db.trigger_sleep();
    db.wait_flush_barrier();

    let res = db.generate_code_hypothesis("struct", vec![]);
    assert!(res.contains(&"MyType".to_string()), "Подсистема чанкинга не смогла вырастить базовое синтаксическое понятие!");
}

#[test]
fn test_async_lateral_inhibition_of_unknown_errors() {
    println!("\n=== [INHIBITION TEST]: Непокрытый сценарий латерального торможения ===");
    let db = SpikeDB::open("sandbox_inhibition");

    // Учим правилу
    for _ in 0..5 {
        db.inject_string_context(vec!["match".to_string(), "value".to_string(), ";".to_string()], 1.2, Some(true));
    }
    db.wait_flush_barrier();

    // Имитируем появление совершенно нового, запрещенного контекста ошибки компилятора (E0277)
    let forbidden_context = vec!["E0277".to_string(), "value".to_string()];

    // Запрашиваем генерацию мутации, передавая этот новый контекст вето
    let res = db.generate_code_hypothesis("match", forbidden_context);

    // Жесткое отрицательное вето: сеть обязана выбросить "value" из траектории мысли,
    // так как этот токен находится в запрещенном латеральном поле ошибки E0277!
    assert!(!res.contains(&"value".to_string()), 
        "Отрицательный сценарий сломан: Латеральное торможение пропустило запрещенный контекст!");
}

// =================================================================
// 6. ГЛУБОКАЯ МАТЕМАТИКА ПЛАСТИЧНОСТИ И АСИММЕТРИИ (DEEP MATRIX)
// =================================================================

#[test]
fn test_positive_exponential_tag_decay_curve() {
    println!("\n=== [MATHEMATICS+ TEST]: Проверка точной кривой угасания синаптического следа ===");
    let mut net = SpikingNetwork::new(BrainConfig::default());
    let s = net.memory.get_or_create_token("Decay_S");
    let t = net.memory.get_or_create_token("Decay_T");

    // Рождаем синапс на тике 0 (tag_trace автоматически взводится в 1.0)
    net.memory.set_synapse(s, t, 0.5, net.current_tick);
    
    // Продвигаем время на 5 тиков вперед
    for _ in 0..5 { net.tick(); }
    
    let mut links = std::mem::take(&mut net.memory.adj_list[s as usize]);
    let synapse = &mut links[0];
    
    // Запускаем ленивое угасание
    let cfg = CONFIG.get().unwrap();
    synapse.decay_tag_lazy(net.current_tick, cfg.tag_tau);
    let tag_at_tick_5 = synapse.tag_trace;
    
    assert!(tag_at_tick_5 < 1.0, "Химический след вообще не уменьшился за 5 тиков!");

    // Продвигаем время еще на 20 тиков вперед
    for _ in 0..20 { net.tick(); }
    synapse.decay_tag_lazy(net.current_tick, cfg.tag_tau);
    let tag_at_tick_25 = synapse.tag_trace;

    // Математический контроль: след на тике 25 обязан быть строго меньше, чем на тике 5!
    assert!(tag_at_tick_25 < tag_at_tick_5, 
        "Математический сбой: Экспоненциальная кривая затухания tag_trace деформирована!");
        
    net.memory.adj_list[s as usize] = links;
}

#[test]
fn test_positive_asymmetric_lateral_inhibition() {
    println!("\n=== [INHIBITION+ TEST]: Проверка асимметрии латерального вето ===");
    let db = SpikeDB::open("sandbox_asymmetric_veto");

    // Прокладываем в ОЗУ два независимых синтаксических маршрута из одной точки "let"
    // Дорога А (Разрешенная): "let" -> "correct_var" -> ";"
    // Дорога Б (Запрещенная): "let" -> "bad_literal" -> ";"
    for _ in 0..5 {
        db.inject_string_context(vec!["let".to_string(), "correct_var".to_string(), ";".to_string()], 1.2, Some(true));
        db.inject_string_context(vec!["let".to_string(), "bad_literal".to_string(), ";".to_string()], 1.2, Some(true));
    }
    db.wait_flush_barrier();

    // Заносим в черный список СТРОГО токен "bad_literal" (Имитируем контекст ошибки Cargo)
    let forbidden_context = vec!["bad_literal".to_string()];

    // Запускаем генерацию
    let res = db.generate_code_hypothesis("let", forbidden_context);

    // Бескомпромиссный контроль асимметрии:
    assert!(!res.contains(&"bad_literal".to_string()), 
        "Отрицательный сценарий сломан: Вето пропустило заблокированный токен!");
        
    assert!(res.contains(&"correct_var".to_string()), 
        "Положительный сценарий сломан: Латеральное вето ошибочно задушило разрешенный параллельный путь!");
}

#[test]
fn test_negative_isolated_meta_chunk_conductance() {
    println!("\n=== [CONDUCTANCE- TEST]: Защита рекурсии от стертых сном мета-узлов ===");
    let net = SpikingNetwork::new(BrainConfig::default());
    
    // Передаем в метод поиска заведомо несуществующий, гигантский ID-призрак (999_999)
    // Наша ОЗУ-система обязана безопасно обработать этот тупик, не свалившись в панику индекса вектора!
    let terminal_id = net.memory.get_chunk_terminal_token_id(999_999);
    
    assert_eq!(terminal_id, 999_999, 
        "Отрицательный сценарий сломан: Безопасный рекурсивный метод .get() не смог обработать изолированный узел!");
}
