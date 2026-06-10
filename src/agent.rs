use std::fs;
use std::process::Command;
use crate::database_manager::SpikeDB;

pub struct SpikeCompilerAgent {
    db: SpikeDB,
    target_file: String,
    pub is_test_mode: bool,
    pub mock_error_code: String,
}

impl SpikeCompilerAgent {
    pub fn new(db: SpikeDB, target_file: &str) -> Self {
        Self {
            db,
            target_file: target_file.to_string(),
            is_test_mode: false,
            mock_error_code: String::new(),
        }
    }

    /// Восприятие среды компилятора (с поддержкой моков для тестов)
    pub fn run_cargo_check(&self) -> Result<String, String> {
        if self.is_test_mode {
            if self.mock_error_code.is_empty() || self.mock_error_code == "Success" {
                return Ok("Success".to_string());
            } else {
                return Err(self.mock_error_code.clone());
            }
        }

        // Реальный вызов Cargo для продакшена
        let output = Command::new("cargo")
            .arg("check")
            .output()
            .expect("Не удалось запустить cargo check");

        if output.status.success() {
            return Ok("Success".to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(start) = stderr.find("error[") {
            if let Some(end) = stderr[start..].find("]") {
                return Err(stderr[start + 6..start + end].to_string());
            }
        }
        Err("UnknownError".to_string())
    }

    /// Чтение первой (целевой) строки сломанного файла
    pub fn read_target_line(&self) -> String {
        fs::read_to_string(&self.target_file)
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// ПУБЛИЧНЫЙ МЕТОД: Перезапись целевой строки в файле новой гипотезой
    pub fn apply_hypothesis(&self, hypothesis: &str) {
        fs::write(&self.target_file, hypothesis).expect("Ошибка записи в файл");
    }

    /// ГЛАВНЫЙ ЦИКЛ АВТОНОМНОГО МЫШЛЕНИЯ И АДАПТАЦИИ
    pub fn perceive_and_adapt(&mut self) -> String {
        // Проверяем среду компилятором
        match self.run_cargo_check() {
            Ok(_) => {
                let current_code = self.read_target_line();
                current_code
            }
            Err(error_code) => {
                let broken_line = self.read_target_line();
                
                // Собираем массив токенов контекста коллизии
                let mut context_tokens = vec![error_code.clone()];
                let special_chars = ['=', ';', '{', '}', '(', ')', '.', ':', ',', '\'', '&', '!'];
                let mut current_token = String::new();
                for ch in broken_line.chars() {
                    if ch.is_whitespace() {
                        if !current_token.is_empty() { context_tokens.push(current_token.clone()); current_token.clear(); }
                    } else if special_chars.contains(&ch) {
                        if !current_token.is_empty() { context_tokens.push(current_token.clone()); current_token.clear(); }
                        context_tokens.push(ch.to_string());
                    } else {
                        current_token.push(ch);
                    }
                }
                if !current_token.is_empty() { context_tokens.push(current_token); }

                // =============================================================
                // НАКАЧКА КОНТЕТКСТА И ЕСТЕСТВЕННЫЙ РЕЗОНАНС
                // =============================================================
                for token in &context_tokens {
                    self.db.inject_token(token, 1.5); // Бьем током контекст
                }
                
                // ИСПРАВЛЕНИЕ: Даем фоновому потоку SpikeDB прокрутить 15 тиков симуляции.
                // За это время заряды от E0308 и знака "=" естественным путем перетекут
                // по графу и взведут tag_trace у правильных скрытых чанков синтаксиса!
                std::thread::sleep(std::time::Duration::from_millis(150));

                // Просим сеть саму сгенерировать мутацию на основе коллизии токов
                let generated_tokens = self.db.generate_code_hypothesis("let", context_tokens.clone());
                let hypothesis_code = generated_tokens.join(" ");

                // ПРИМЕНЕНИЕ ГИПОТЕЗЫ
                self.apply_hypothesis(&hypothesis_code);
                std::thread::sleep(std::time::Duration::from_millis(20));

                // Проверяем, исправило ли это ситуацию
                match self.run_cargo_check() {
                    Ok(_) => {
                        self.db.approve_success(true); // Успех, закрепляем синапсы!
                        hypothesis_code
                    }
                    Err(_) => {
                        self.db.approve_success(false); // Штраф, откат
                        self.apply_hypothesis(&broken_line);
                        broken_line
                    }
                }
            }
        }
    }
}
