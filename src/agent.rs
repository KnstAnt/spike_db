use std::process::Command;
use std::fs;
use crate::database_manager::SpikeDB;

pub struct SpikeCompilerAgent {
    db: SpikeDB,
    target_file: String,
}

impl SpikeCompilerAgent {
    pub fn new(db: SpikeDB, target_file: &str) -> Self {
        Self {
            db,
            target_file: target_file.to_string(),
        }
    }

    /// Шаг 1: Восприятие среды. Запускает cargo check и вытаскивает код ошибки
    pub fn run_cargo_check(&self) -> Result<String, String> {
        let output = Command::new("cargo")
            .arg("check")
            .output()
            .expect("Не удалось запустить cargo check");

        if output.status.success() {
            return Ok("Success".to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Простейший парсер кода ошибки (ищет например error[E0308])
        if let Some(start) = stderr.find("error[") {
            if let Some(end) = stderr[start..].find("]") {
                let error_code = &stderr[start + 6..start + end];
                return Err(error_code.to_string());
            }
        }

        Err("UnknownError".to_string())
    }

    /// Шаг 2: Чтение сломанного файла
    pub fn read_broken_line(&self) -> String {
        fs::read_to_string(&self.target_file)
            .unwrap_or_default()
            .lines()
            .next() // Для прототипа работаем с первой строкой файла
            .unwrap_or("")
            .to_string()
    }

    /// Шаг 3: Контур размышления и генерации гипотез
    pub fn evolve_and_fix(&mut self) {
        println!("[AGENT]: Запуск цикла исправления кода...");

        // Проверяем статус компиляции
        match self.run_cargo_check() {
            Ok(_) => {
                println!("[AGENT]: Код уже успешно компилируется! Обучение не требуется.");
            }
            Err(error_code) => {
                println!("[AGENT]: Обнаружена ошибка компилятора: {}", error_code);
                let broken_code = self.read_broken_line();
                println!("[AGENT]: Исходный поврежденный код: '{}'", broken_code);

                // Накачиваем сеть контекстом ошибки и сломанного кода
                self.db.inject_token(&error_code, 1.5);
                
                // Передаем токены сломанной строки в сеть, запуская ассоциативный процесс
                // (Используем наш синтаксический разборщик)
                let special_chars = ['=', ';', '{', '}', '(', ')', '.', ':', ','];
                let mut current_token = String::new();
                for ch in broken_code.chars() {
                    if ch.is_whitespace() {
                        if !current_token.is_empty() {
                            self.db.inject_token(&current_token, 1.2);
                            current_token.clear();
                        }
                    } else if special_chars.contains(&ch) {
                        if !current_token.is_empty() {
                            self.db.inject_token(&current_token, 1.2);
                            current_token.clear();
                        }
                        self.db.inject_token(&ch.to_string(), 1.2);
                    } else {
                        current_token.push(ch);
                    }
                }
                if !current_token.is_empty() {
                    self.db.inject_token(&current_token, 1.2);
                }

                // Даем сети тики времени, чтобы сформировались ассоциативные цепочки
                // В этот момент в фоновом потоке SpikeDB рождаются траектории
                std::thread::sleep(std::time::Duration::from_millis(100));

                println!("[AGENT]: Сбор ассоциаций и генерация гипотез...");
                
                // Сюда мы будем собирать успешные варианты кодов, отсортированные по сложности правок
                let mut successful_hypotheses: Vec<(String, usize)> = Vec::new();

                // Реализуем генеративный пул...
                // Чтобы двинуться дальше, нам нужно настроить Ядро SpikeDB возвращать не просто
                // статичное предсказание одной связи, а выплескивать наружу сгенерированные текстовые цепочки токенов.
            }
        }
    }
}
