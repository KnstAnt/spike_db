use spikedb::database_manager::SpikeDB;
use spikedb::config::BrainConfig;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn main() {
    println!("============================================================");
    println!("===    ДЕМОНСТРАЦИЯ АССОЦИАТИВНОГО МЫШЛЕНИЯ SpikeDB      ===");
    println!("============================================================");

    // 1. Инициализация и быстрое In-Memory обучение на датасете
    let mut config = BrainConfig::default();
    config.coincidence_threshold = 8; 
    config.learning_rate = 0.4;       
    config.leak_tau = 10.0;           
    let _ = fs::write("brain_config.toml", toml::to_string_pretty(&config).unwrap());

    let db = SpikeDB::open("dummy_ram_path");
    std::thread::sleep(Duration::from_millis(50));

    let sequences_dir = "tests/sequences";
    let mut file_paths: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(sequences_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "txt") {
                file_paths.push(path);
            }
        }
    }
    file_paths.sort();

    println!("[1/3] Запуск конвейера Curriculum Learning (1500+ строк)...");
    for (index, path) in file_paths.iter().enumerate() {
        let file_content = fs::read_to_string(path).unwrap();
        for line in file_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

            let special_chars = ['=', ';', '{', '}', '(', ')', '+', '-', '>', '<', ':', ',', '\'', '&', '!'];
            let mut current_token = String::new();
            for ch in trimmed.chars() {
                if ch.is_whitespace() {
                    if !current_token.is_empty() { db.inject_token(&current_token, 1.2); current_token.clear(); }
                } else if special_chars.contains(&ch) {
                    if !current_token.is_empty() { db.inject_token(&current_token, 1.2); current_token.clear(); }
                    db.inject_token(&ch.to_string(), 1.2);
                } else {
                    current_token.push(ch);
                }
            }
            if !current_token.is_empty() { db.inject_token(&current_token, 1.2); }
        }
        if index >= 4 {
            db.approve_success(true); // Дофаминовое подкрепление сложных тем
        }
    }
    
    println!("[2/3] Погружение сети в Контрастный Сон (Прунинг шума)...");
    db.trigger_sleep();
    std::thread::sleep(Duration::from_millis(100));

    // =================================================================
    // ШАГ 3: ДЕМОНСТРАЦИЯ СВОБОДНЫХ АССОЦИАТИВНЫХ ЦЕПОЧЕК (ЭХО МЫСЛЕЙ)
    // =================================================================
    println!("\n[3/3] ФАЗА ЭКСПЕРТИЗЫ: Генерация ассоциативных траекторий");
    println!("------------------------------------------------------------");

    // Список триггеров-стимулов, которые мы поочередно будем забрасывать в сеть
    let test_triggers = vec!["fn", "let", "struct", "match", "enum"];

    for trigger in test_triggers {
        println!("\n🧠 Подаем входной стимул: '{}'", trigger);
        
        // Подаем пустой вектор контекста, чтобы сеть думала в режиме свободных ассоциаций
        // и опиралась строго на чистый резонанс выученных синапсов и шунтирования!
        let empty_context: Vec<String> = Vec::new();
        
        // Запрашиваем генерацию гипотезы
        let trail = db.generate_code_hypothesis(trigger, empty_context);
        
        // Выводим результат
        print!("   ↳ Сгенерированная цепочка: ");
        if trail.is_empty() {
            println!("[Связи отсутствуют или затухли]");
        } else {
            // Подсвечиваем стрелочками переходы мысли в консоли
            for (i, token) in trail.iter().enumerate() {
                if i > 0 { print!(" ➔ "); }
                print!("'{}'", token);
            }
            println!();
        }
    }

    println!("\n============================================================");
    println!("=== ДЕМОНСТРАЦИЯ ЗАВЕРШЕНА. СЕТЬ ПОЛНОСТЬЮ АВТОНОМНА.    ===");
    println!("============================================================");
    
    drop(db);
    let _ = fs::remove_file("brain_config.toml");
}
