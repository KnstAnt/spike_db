use spikedb::database_manager::SpikeDB;
use spikedb::config::BrainConfig;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn run_performance_stress_test() {
    println!("\n============================================================");
    println!("=== ЗАПУСК ЧИСТОГО СТРЕСС-ТЕСТА БЫСТРОДЕЙСТВИЯ (БЕЗ SLEEP) ===");
    println!("============================================================");

    let mut config = BrainConfig::default();
    config.coincidence_threshold = 8; 
    config.learning_rate = 0.4;       
    config.leak_tau = 10.0;           
    let _ = fs::write("brain_config.toml", toml::to_string_pretty(&config).unwrap());

    let db = SpikeDB::open("dummy_ram_path");

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

    let mut total_lines = 0;
    let mut total_tokens = 0;
    let mut all_tokens_to_inject = Vec::new();

    let special_chars = ['=', ';', '{', '}', '(', ')', '+', '-', '>', '<', ':', ',', '\'', '&', '!'];
    for path in &file_paths {
        let file_content = fs::read_to_string(path).expect("Ошибка чтения файла");
        for line in file_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
            total_lines += 1;

            let mut current_token = String::new();
            for ch in trimmed.chars() {
                if ch.is_whitespace() {
                    if !current_token.is_empty() {
                        all_tokens_to_inject.push(current_token.clone());
                        total_tokens += 1;
                        current_token.clear();
                    }
                } else if special_chars.contains(&ch) {
                    if !current_token.is_empty() {
                        all_tokens_to_inject.push(current_token.clone());
                        total_tokens += 1;
                        current_token.clear();
                    }
                    all_tokens_to_inject.push(ch.to_string());
                    total_tokens += 1;
                } else {
                    current_token.push(ch);
                }
            }
            if !current_token.is_empty() {
                all_tokens_to_inject.push(current_token);
                total_tokens += 1;
            }
        }
    }

    println!("[БЕНЧМАРК]: Массив данных успешно загружен в ОЗУ теста.");
    println!("            Всего уникальных строк кода: {}", total_lines);
    println!("            Всего импульсов-токенов для накачки: {}", total_tokens);
    println!("------------------------------------------------------------");
    println!("[БЕНЧМАРК]: Чистая lock-free лавина запущена...");

    // =================================================================
    // ИСТИННЫЙ ЗАМЕР ВРЕМЕНИ В ОЗУ
    // =================================================================
    let start_time = Instant::now();

    // Залпом выстреливаем все 18 000 токенов в канал на максимальной скорости CPU!
    for token in all_tokens_to_inject {
        db.inject_token(&token, 1.2);
    }

    // Дофаминовое подкрепление
    db.approve_success(true);

    // Только ТЕПЕРЬ, когда эволюция зафиксирована, ложимся спать
    db.trigger_sleep();
    
    // Синхронизируем окончание сна
    let _finish_sync = db.inspect_prediction("let");

    let duration = start_time.elapsed();
    // =================================================================

    let duration_secs = duration.as_secs_f32();
    let duration_millis = duration.as_millis();
    let tokens_per_sec = total_tokens as f32 / duration_secs;

    println!("------------------------------------------------------------");
    println!("=== РЕЗУЛЬТАТЫ ИСТИННОГО БЕНЧМАРКА SpikeMemory ===");
    println!("Затрачено чистого процессорного времени: {} мс ({:.4} сек)", duration_millis, duration_secs);
    println!("Реальная скорость сквозного мышления: {:.2} токенов/сек", tokens_per_sec);
    println!("============================================================");

    drop(db);
    let _ = fs::remove_file("brain_config.toml");
}
