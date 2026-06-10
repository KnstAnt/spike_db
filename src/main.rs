use spikedb::database_manager::SpikeDB;
use std::fs;
use std::time::SystemTime;

fn main() {
    println!("=== ИНТЕРФЕЙС КЛАССА SpikeDB ===");

    let unique_timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    let db_path = format!("./spikedb_data_{}", unique_timestamp);
    
    // Просто открываем базу данных. Вся магия потоков скрыта внутри!
    let db = SpikeDB::open(&db_path);

    println!("\nСистема готова. Вводите команды:");
    println!("  - Текст (например: 'let x = 5;') для отправки импульсов.");
    println!("  - 'sleep' для запуска очистки памяти.");
    println!("  - 'good' для отправки дофамина.");
    println!("  - 'exit' для выхода.");
    println!("------------------------------------------------------------");

    let special_chars = ['=', ';', '{', '}', '(', ')', '+', '-', '>', '<'];

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();

        if trimmed == "exit" {
            break; // Выходим из цикла. Метод Drop структуры SpikeDB автоматически закроет потоки!
        } else if trimmed == "sleep" {
            db.trigger_sleep(); // Просто вызываем метод класса
        } else if trimmed == "good" {
            println!("[ИНТЕРФЕЙС]: Отправка сигнала успеха...");
            db.approve_success(true);
        } else if trimmed.is_empty() {
            continue;
        } else {
            println!("[ИНТЕРФЕЙС]: Нарезка и асинхронная отправка строки...");
            
            let mut current_token = String::new();
            for ch in trimmed.chars() {
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
    }

    println!("\n Закрытие интерфейса... Класс SpikeDB автоматически останавливает потоки.");
    drop(db); // Вызов деструктора
    
    std::thread::sleep(std::time::Duration::from_millis(60));
    let _ = fs::remove_dir_all(&db_path);
    println!("Временные файлы зачищены. Пока!");
}
