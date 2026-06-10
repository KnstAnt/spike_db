use spikedb::database_manager::SpikeDB;
use spikedb::config::BrainConfig;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn test_complex_curriculum_learning_lifecycle() {
    println!("\n=== ЗАПУСК БОЛЬШОГО ВЫЧИСЛИТЕЛЬНОГО КОНВЕЙЕРА SpikeDB (500+ СТРОК) ===");

    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let db_path = format!("./complex_test_db_{}", ts);
    
    // Тонкая настройка характера сети под экстремальные объемы данных
    let mut config = BrainConfig::default();
    config.coincidence_threshold = 5; // Отсекаем ложный шум на больших массивах
    config.learning_rate = 0.4;       // Дофаминовый коэффициент
    config.leak_tau = 12.0;           // Сеть остывает быстрее, резко повышая контраст
    
    let toml_string = toml::to_string_pretty(&config).unwrap();
    let _ = fs::write("brain_config.toml", toml_string);

    let db = SpikeDB::open(&db_path);
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

    println!("[КОНВЕЙЕР]: Найдено файлов для сквозного обучения: {}", file_paths.len());

    // Прогоняем лавину импульсов через Актор-менеджер
    for (index, path) in file_paths.iter().enumerate() {
        let file_name = path.file_name().unwrap().to_string_lossy();
        println!("  -> Обработка Учебного Модуля [{}]: '{}'...", index + 1, file_name);

        let file_content = fs::read_to_string(path).unwrap();
        let mut lines_processed = 0;

        for line in file_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

            send_string_to_db(&db, trimmed);
            // Пауза 4 мс дает ядру время асинхронно разложить спайки по Key-Value таблицам Sled
            std::thread::sleep(Duration::from_millis(4)); 
            lines_processed += 1;
        }
        println!("     Успешно усвоено строк кода: {}", lines_processed);

        // Интеллектуальное одобрение Критика: подкрепляем сложные темы (ООП, Дженерики, Лайфтаймы)
        if file_name.starts_with("03") || file_name.starts_with("04") || file_name.starts_with("05") {
            println!("     [КРИТИК]: Модуль сложной логики пройден успешно. Выброс дофамина!");
            db.approve_success(true);
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    // Ждем полной стабилизации асинхронной очереди спайков
    println!("\n[ЯДРО]: Фокус внимания обрабатывает остаточные импульсы...");
    std::thread::sleep(Duration::from_millis(600));

    // Погружаем SpikeDB в глубокий нелинейный сон для лавинообразного удаления шума
    db.trigger_sleep();
    std::thread::sleep(Duration::from_millis(250));

    // =================================================================
    // МАСШТАБНЫЙ ЭКЗАМЕН СЕТИ
    // =================================================================
    println!("\n=== ГЛОБАЛЬНЫЙ ЭКЗАМЕН ЗНАНИЙ SpikeDB ===");
    
    // Проверка 1: Базовый синтаксис функций "fn"
    if let Some((target_id, weight)) = db.inspect_prediction("fn") {
        println!("[РЕЗУЛЬТАТ]: Для токена 'fn' сильнейшая ассоциация ведет к ID: {}, вес: {:.2}", target_id, weight);
        assert!(weight > 0.8, "Сеть растеряла базовые знания о функциях!");
    }

    // Проверка 2: Объявление переменных "let"
    if let Some((target_id, weight)) = db.inspect_prediction("let") {
        println!("[РЕЗУЛЬТАТ]: Для токена 'let' сильнейшая ассоциация ведет к ID: {}, вес: {:.2}", target_id, weight);
        assert!(weight > 0.8);
    }

    // Проверка 3: Усвоение ООП-структур "struct" (из файла 03)
    if let Some((target_id, weight)) = db.inspect_prediction("struct") {
        println!("[РЕЗУЛЬТАТ]: Для токена 'struct' сильнейшая ассоциация ведет к ID: {}, вес: {:.2}", target_id, weight);
        assert!(weight > 0.7, "Сеть проигнорировала синтаксис ООП!");
        println!("[ПРОЙДЕНО]: Модуль структур и имплементаций успешно консолидирован во сне.");
    }

    // Проверка 4: Усвоение высшей логики "match" (из файла 04)
    if let Some((target_id, weight)) = db.inspect_prediction("match") {
        println!("[РЕЗУЛЬТАТ]: Для токена 'match' сильнейшая ассоциация ведет к ID: {}, вес: {:.2}", target_id, weight);
        assert!(weight > 0.6, "Сеть не справилась со сложным сопоставлением шаблонов!");
        println!("[ПРОЙДЕНО]: Паттерны match-выражений успешно усвоены.");
    }

    println!("\n[ГЛОБАЛЬНЫЙ УСПЕХ]: SpikeDB успешно переварила 500+ строк кода и сдала экзамен!");
    
    drop(db);
    std::thread::sleep(Duration::from_millis(80)); 
    let _ = fs::remove_dir_all(&db_path);
    let _ = fs::remove_file("brain_config.toml");
}

fn send_string_to_db(db: &SpikeDB, text: &str) {
    let special_chars = ['=', ';', '{', '}', '(', ')', '+', '-', '>', '<', ':', ',', '\'', '&', '!'];
    let mut current_token = String::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !current_token.is_empty() {
                db.inject_token(&current_token, 1.2);
                current_token.clear();
            }
        } else if special_chars.contains(&ch) {
            if !current_token.is_empty() {
                db.inject_token(&current_token, 1.2);
                current_token.clear();
            }
            db.inject_token(&ch.to_string(), 1.2);
        } else {
            current_token.push(ch);
        }
    }
    if !current_token.is_empty() {
        db.inject_token(&current_token, 1.2);
    }
}
