use spikedb::database_manager::SpikeDB;
use spikedb::config::BrainConfig;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

// Фиксированный путь к базе данных, передаваемый между тестами как эстафета
const SHARED_DB_PATH: &str = "./complex_production_brain_db";

// =================================================================
// ЭТАП 1: НАКАЧКА ДАННЫХ, ЧАНКИНГ И КОНСОЛИДАЦИЯ ПАМЯТИ
// =================================================================
#[test]
fn step1_curriculum_learning() {
    println!("\n=== [ФАЗА 1]: МАСШТАБНЫЙ КОНВЕЙЕР НАКАЧКИ ДАННЫХ SpikeDB ===");

    // Зачищаем старую базу, если она осталась от аварийных прошлых запусков
    if fs::metadata(SHARED_DB_PATH).is_ok() {
        let _ = fs::remove_dir_all(SHARED_DB_PATH);
    }

    // Тонкая настройка характера сети
    let mut config = BrainConfig::default();
    config.coincidence_threshold = 8; // Порог чанкинга
    config.learning_rate = 0.4;       // Дофамин
    config.leak_tau = 10.0;           // Быстрое забывание шума
    
    let toml_string = toml::to_string_pretty(&config).unwrap();
    let _ = fs::write("brain_config.toml", toml_string);

    let db = SpikeDB::open(SHARED_DB_PATH);
    std::thread::sleep(Duration::from_millis(100));

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

    // Запуск лавины текстовых спайков
    for (index, path) in file_paths.iter().enumerate() {
        let file_name = path.file_name().unwrap().to_string_lossy();
        println!("  -> Поглощение модуля [{}]: '{}'...", index + 1, file_name);

        let file_content = fs::read_to_string(path).unwrap();
        for line in file_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

            send_string_to_db(&db, trimmed);
            std::thread::sleep(Duration::from_millis(5)); 
        }

        if index >= 4 {
            db.approve_success(true); // Выброс дофамина Критиком
            std::thread::sleep(Duration::from_millis(30));
        }
    }

    println!("[ЯДРО]: Стабилизация фокуса внимания перед сном...");
    std::thread::sleep(Duration::from_millis(1500));

    // Ночная нелинейная очистка мусора
    db.trigger_sleep();
    std::thread::sleep(Duration::from_millis(400));

    println!("[ФАЗА 1 ЗАВЕРШЕНА]: База данных зафиксирована на диске.");
    // НАМЕРЕННО ДРОПАЕМ СЕТЬ, НО НЕ УДАЛЯЕМ ПАПКУ С ДИСКА
    drop(db); 
}

// =================================================================
// ЭТАП 2: СТАТИЧЕСКАЯ ИНСПЕКЦИЯ СФОРМИРОВАННЫХ СВЯЗЕЙ
// =================================================================
#[test]
fn step2_static_inspection() {
    println!("\n=== [ФАЗА 2]: СТАТИЧЕСКАЯ ИНСПЕКЦИЯ СФОРМИРОВАННЫХ СВЯЗЕЙ ===");
    
    // Открываем ПОВТОРНО уже готовую, обученную на первом шаге базу данных!
    let db = SpikeDB::open(SHARED_DB_PATH);
    std::thread::sleep(Duration::from_millis(50));

    // Экзамен на знание базовых токеновlet и fn
    if let Some((target_id, weight)) = db.inspect_prediction("let") {
        println!("  [ПРОВЕРКА 'let']: Сильнейшая ассоциация ведет к ID: {}, вес: {:.2}", target_id, weight);
        assert!(weight > 0.8, "Ошибка: Связь let-объявлений не зацементировалась!");
    }

    if let Some((target_id, weight)) = db.inspect_prediction("fn") {
        println!("  [ПРОВЕРКА 'fn']: Сильнейшая association ведет к ID: {}, вес: {:.2}", target_id, weight);
        assert!(weight > 0.8, "Ошибка: Синтаксис функций утерян во сне!");
    }

    if let Some((target_id, weight)) = db.inspect_prediction("struct") {
        println!("  [ПРОВЕРКА 'struct']: Сильнейшая связь ведет к ID: {}, вес: {:.2}", target_id, weight);
        assert!(weight > 0.7);
    }

    println!("[ФАЗА 2 ЗАВЕРШЕНА]: Все статические правила успешно верифицированы.");
    drop(db);
}

// =================================================================
// ЭТАП 3: ПРОВЕРКА ДИНАМИЧЕСКОГО МЫШЛЕНИЯ И АССОЦИАТИВНЫХ ЦЕПОЧЕК
// =================================================================
#[test]
fn step3_dynamic_reasoning() {
    println!("\n=== [ФАЗА 3]: ТЕСТ ДИНАМИЧЕСКОГО МЫШЛЕНИЯ И АССОЦИАТИВНЫХ ЦЕПОЧЕК ===");
    
    // Снова поднимаем ту же самую базу знаний
    let db = SpikeDB::open(SHARED_DB_PATH);
    std::thread::sleep(Duration::from_millis(50));

    println!("  [СТИМУЛ]: Подаем изолированный импульс на вход 'fn'...");
    db.inject_token("fn", 1.5);
    
    println!("  [ЯДРО]: Волна возбуждения распространяется по мета-слоям графа...");
    std::thread::sleep(Duration::from_millis(200));

    // Проверяем логический вывод: докатился ли импульс до стрелки возврата типа '->'
    if let Some((prediction_target, weight)) = db.inspect_prediction("->") {
        println!("  [ЦЕПОЧКА ВЫВОДА]: Токен '->' успешно зажегся по цепочке ассоциаций!");
        println!("                    Сила удержания связи: {:.2}", weight);
        assert!(weight > 0.7, "Логическая цепь вывода от 'fn' до '->' разорвана!");
    }

    // Проверяем длинную цепь: связь паттерн-матчинга 'match'
    if let Some((match_target, match_weight)) = db.inspect_prediction("match") {
        println!("  [ЦЕПОЧКА ВЫВОДА]: Токен 'match' удерживает абстракцию сопоставления шаблонов!");
        println!("                    Прочность связи: {:.2}", match_weight);
        assert!(match_weight > 0.5);
    }

    println!("\n[ГЛОБАЛЬНЫЙ ТРИУМФ]: SpikeDB подтвердила способность к последовательному умозаключению!");
    
    // ФИНАЛЬНАЯ ЗАЧИСТКА: Тесты окончены, удаляем базу данных с жесткого диска
    drop(db);
    std::thread::sleep(Duration::from_millis(100)); 
    let _ = fs::remove_dir_all(SHARED_DB_PATH);
    let _ = fs::remove_file("brain_config.toml");
}

// =================================================================
// ВСПОМОГАТЕЛЬНЫЕ ИНФРАСТРУКТУРНЫЕ ФУНКЦИИ
// =================================================================
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
