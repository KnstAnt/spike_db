use spikedb::database_manager::SpikeDB;
use spikedb::agent::SpikeCompilerAgent;
use spikedb::config::BrainConfig;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_autonomous_mutation_on_large_dataset() {
    println!("\n=== ЗАПУСК ПЕСОЧНИЦЫ АВТОНОМНОГО АГЕНТА SpikeDB НА БОЛЬШИХ ДАННЫХ ===");

    let db_path = "dummy_ram_path";
    let target_file = "./virtual_broken_project.rs";
    let _ = fs::remove_file(target_file);

    let mut config = BrainConfig::default();
    config.coincidence_threshold = 8; 
    config.learning_rate = 0.4;       
    config.leak_tau = 10.0;           
    let _ = fs::write("brain_config.toml", toml::to_string_pretty(&config).unwrap());

    let db = SpikeDB::open(db_path);

    // =================================================================
    // ФАЗА 1: НАКАЧКА ОБУЧАЮЩИХ УРОКОВ (CURRICULUM LEARNING)
    // =================================================================
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

    // Добавляем 10 правильных строковых паттернов прямо в ОЗУ перед файлами,
    // чтобы зацементировать в мозге рельсы "to_string()" для строк!
    println!("[ТЕСТ]: Инициализация базового строкового опыта...");
    for i in 0..12 {
        let string_line = vec![
            "let".to_string(), 
            format!("text_{}", i), 
            ":".to_string(), 
            "String".to_string(), 
            "=".to_string(), 
            "word".to_string(), 
            ".".to_string(), 
            "to_string".to_string(), 
            "(".to_string(), 
            ")".to_string(), 
            ";".to_string()
        ];
        db.inject_string_context(string_line, 1.2, Some(true));
    }

    println!("[ТЕСТ]: Запуск пакетного конвейера обучения на файлах...");
    for (index, path) in file_paths.iter().enumerate() {
        let file_content = fs::read_to_string(path).unwrap();
        
        for line in file_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

            let mut line_tokens = Vec::new();
            let special_chars = ['=', ';', '{', '}', '(', ')', '+', '-', '>', '<', ':', ',', '\'', '&', '!'];
            let mut current_token = String::new();
            
            for ch in trimmed.chars() {
                if ch.is_whitespace() {
                    if !current_token.is_empty() { line_tokens.push(current_token.clone()); current_token.clear(); }
                } else if special_chars.contains(&ch) {
                    if !current_token.is_empty() { line_tokens.push(current_token.clone()); current_token.clear(); }
                    line_tokens.push(ch.to_string());
                } else {
                    current_token.push(ch);
                }
            }
            if !current_token.is_empty() { line_tokens.push(current_token); }

            db.inject_string_context(line_tokens, 1.2, Some(index >= 4));
        }
    }

    // Дожидаемся фиксации знаний и усыпляем сеть
    db.wait_flush_barrier();
    println!("[ТЕСТ]: Погружение графа в сон...");
    db.trigger_sleep();
    db.wait_flush_barrier();

    // =================================================================
    // ФАЗА 2: ИНИЦИАЛИЗАЦИЯ ДЕФЕКТА И МОКИРОВАНИЕ СРЕДЫ
    // =================================================================
    let broken_code = "let alpha_0 = \"bad_literal\" ;";
    fs::write(target_file, broken_code).unwrap();

    let mut agent = SpikeCompilerAgent::new(db, target_file);
    agent.is_test_mode = true;
    agent.mock_error_code = "E0308".to_string(); // Мокаем ошибку несоответствия типов

    // =================================================================
    // ФАЗА 3: ЗАПУСК АГЕНТА И ЭКСПЕРТИЗА МУТАЦИИ
    // =================================================================
    // Переключаем мок на успех компиляции в случае применения верной гипотезы
    agent.mock_error_code = "Success".to_string(); 

    let resulting_code = agent.perceive_and_adapt();
    println!("\n[ТЕСТ]: Итоговый результат автономного мышления SpikeDB: '{}'", resulting_code);

    // Верифицируем, что латеральное торможение выжгло дефект, а рекурсивный поиск
    // NeuronOrigin перенаправил синтаксис на строковые рельсы!
    assert!(!resulting_code.contains("\"bad_literal\""), 
        "Критическая ошибка: Сеть оставила дефектный токен в коде!");
        
    assert!(resulting_code.contains("to_string"), 
        "Ошибка: Мысль сети не смогла мутировать в сторону строкового метода!");

    let _ = fs::remove_file(target_file);
    let _ = fs::remove_file("brain_config.toml");
}
