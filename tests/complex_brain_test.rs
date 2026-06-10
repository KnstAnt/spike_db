use spikedb::database_manager::SpikeDB;
use spikedb::config::BrainConfig;
use std::fs;
use std::path::PathBuf;

const SHARED_DB_PATH: &str = "dummy_ram_path";

#[test]
fn step1_curriculum_learning() {
    println!("\n=== [ФАЗА 1]: МАСШТАБНЫЙ КОНВЕЙЕР НАКАЧКИ ДАННЫХ SpikeDB ===");

    let mut config = BrainConfig::default();
    config.coincidence_threshold = 8; 
    config.learning_rate = 0.4;       
    config.leak_tau = 10.0;           
    
    let toml_string = toml::to_string_pretty(&config).unwrap();
    let _ = fs::write("brain_config.toml", toml_string);

    let db = SpikeDB::open(SHARED_DB_PATH);

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

            // Передаем Some(true) для всех обучающих строк, цементируя связи атомарно!
            db.inject_string_context(line_tokens, 1.2, Some(index >= 4));
        }
    }

    // ИСПРАВЛЕНИЕ: Используем каноничный барьер очереди вместо inspect_prediction
    db.wait_flush_barrier();

    println!("[ФАЗА 1 ЗАВЕРШЕНА]: База зафиксирована, погружение в сон...");
    db.trigger_sleep();
    
    // Барьер окончания сна
    db.wait_flush_barrier();

    // ЗАПУСК ЭКСПЕРТИЗЫ
    let forbidden_context: Vec<String> = Vec::new();
    let trail = db.generate_code_hypothesis("fn", forbidden_context);
    println!("  [ДИНАМИЧЕСКИЙ ВЫВОД 'fn']: Ассоциативный шлейф: {:?}", trail);
    
    assert!(!trail.is_empty(), "Сеть не должна выдавать пустой шлейф!");
    assert!(trail.contains(&"fn".to_string()), "Шлейф должен содержать стартовый токен!");

    let _ = fs::remove_file("brain_config.toml");
}
