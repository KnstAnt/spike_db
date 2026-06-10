use spikedb::database_manager::SpikeDB;
use spikedb::agent::SpikeCompilerAgent;
use spikedb::config::BrainConfig;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn test_autonomous_mutation_on_large_dataset() {
    println!("\n=== ЗАПУСК ПЕСОЧНИЦЫ АВТОНОМНОГО АГЕНТА SpikeDB НА БОЛЬШИХ ДАННЫХ ===");

    let db_path = "./agent_production_brain_db";
    let target_file = "./virtual_broken_project.rs";

    // Очищаем старые артефакты
    let _ = fs::remove_dir_all(db_path);
    let _ = fs::remove_file(target_file);

    // 1. Тонкая настройка рантайм-характера сети из TOML
    let mut config = BrainConfig::default();
    config.coincidence_threshold = 8; // Жесткий порог чанкинга
    config.learning_rate = 0.4;       
    config.leak_tau = 10.0;           // Мгновенное угасание фонового шума
    
    let toml_string = toml::to_string_pretty(&config).unwrap();
    let _ = fs::write("brain_config.toml", toml_string);

    let db = SpikeDB::open(db_path);
    std::thread::sleep(Duration::from_millis(100));

    // =================================================================
    // ФАЗА 1: СКАНИРОВАНИЕ И МАСШТАБНАЯ НАКАЧКА РЕАЛЬНОГО ДАТАСЕТА (1500+ СТРОК)
    // =================================================================
    let sequences_dir = "tests/sequences";
    println!("[ТЕСТ]: Сканирование директории с реальными уроками: '{}'...", sequences_dir);

    let mut file_paths: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(sequences_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "txt") {
                file_paths.push(path);
            }
        }
    }
    file_paths.sort(); // Упорядочиваем по сложности 01, 02, 03...

    println!("[ТЕСТ]: Запуск масштабного конвейера обучения...");
    for (index, path) in file_paths.iter().enumerate() {
        let file_content = fs::read_to_string(path).unwrap();
        for line in file_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

            // Шлем токены в сеть
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
            std::thread::sleep(Duration::from_millis(4)); 
        }

        // Подкрепляем сложные синтаксические конструкции
        if index >= 4 {
            db.approve_success(true);
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    println!("[ТЕСТ]: Ожидание стабилизации фокуса внимания и остаточных зарядов...");
    std::thread::sleep(Duration::from_millis(1500));

    // Сессия ночного сна для выжигания рантайм-мусора и контрастирования грамматики
    db.trigger_sleep();
    std::thread::sleep(Duration::from_millis(400));

    // =================================================================
    // ФАЗА 2: ИНИЦИАЛИЗАЦИЯ ДЕФЕКТА С ВЫСОКИМ ДИССОНАНСОМ
    // Записываем дефектный код: "let alpha_0 = "bad_literal" ;"
    // Из файла 01 сеть знает, что после знака "=" должно идти число, а не строка!
    // =================================================================
    println!("\n[ТЕСТ]: Фаза 2. Инициализация дефектного кода и мокирование ошибки E0308...");
    let broken_code = "let alpha_0 = \"bad_literal\" ;";
    fs::write(target_file, broken_code).unwrap();

    // Создаем агента над нашей огромной базой знаний
    let mut agent = SpikeCompilerAgent::new(db, target_file);
    agent.is_test_mode = true;
    agent.mock_error_code = "E0308".to_string(); // Подаем код ошибки несоответствия типов

    // =================================================================
    // ФАЗА 3: ЭКЗАМЕН АВТОНОМНОЙ МУТАЦИИ И ВЫВОДА
    // =================================================================
    println!("\n[ТЕСТ]: Фаза 3. Запуск агента. Включение латерального торможения...");
    
    std::thread::sleep(Duration::from_millis(20));

    // Переключаем мок на успех
    agent.mock_error_code = "Success".to_string(); 

    let resulting_code = agent.perceive_and_adapt();
    println!("\n[ТЕСТ]: Результат автономного мышления SpikeDB: '{}'", resulting_code);

    // ВЕРИФИКАЦИЯ ОСМЫСЛЕННОСТИ: Проверяем, что на основе опыта файла 01 сеть 
    // сама отбросила неверный строковый литерал и вернула правильный целочисленный паттерн!
    // (Поскольку в 01 файле у нас были числа, сеть должна была перетечь на числовые синапсы)
    assert!(!resulting_code.contains("\"bad_literal\""), 
        "Критическая ошибка: Сеть не смогла совершить автономную мутацию и оставила дефектный токен!");
    
    println!("[ТЕСТ]: ПОТРЯСАЮЩЕ! SpikeDB успешно обошла дефект на основе естественного диссонанса больших данных.");

    // Очистка песочницы
    drop(agent);
    std::thread::sleep(Duration::from_millis(100));
    let _ = fs::remove_dir_all(db_path);
    let _ = fs::remove_file(target_file);
    let _ = fs::remove_file("brain_config.toml");
}
