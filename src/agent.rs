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

    pub fn run_cargo_check(&self) -> Result<String, String> {
        if self.is_test_mode {
            if self.mock_error_code.is_empty() || self.mock_error_code == "Success" {
                return Ok("Success".to_string());
            } else {
                return Err(self.mock_error_code.clone());
            }
        }

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

    pub fn read_target_line(&self) -> String {
        fs::read_to_string(&self.target_file)
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    }

    pub fn apply_hypothesis(&self, hypothesis: &str) {
        fs::write(&self.target_file, hypothesis).expect("Ошибка записи в файл");
    }

    /// ГЛАВНЫЙ ЦИКЛ АВТОНОМНОГО МЫШЛЕНИЯ АГЕНТА (ПАКЕТНЫЙ КОНТУР)
    pub fn perceive_and_adapt(&mut self) -> String {
        match self.run_cargo_check() {
            Ok(_) => self.read_target_line(),
            Err(error_code) => {
                let broken_line = self.read_target_line();
                
                // Нарезаем ломаную строку на лексемы
                let mut context_tokens = vec![error_code.clone()];
                let special_chars = ['=', ';', '{', '}', '(', ')', '.', ':', ',', '\'', '&', '!'];
                let mut current_token = String::new();
                
                for ch in broken_line.chars() {
                    if ch.is_whitespace() {
                        if !current_token.is_empty() { 
                            context_tokens.push(current_token.clone()); 
                            current_token.clear(); 
                        }
                    } else if special_chars.contains(&ch) {
                        if !current_token.is_empty() { 
                            context_tokens.push(current_token.clone()); 
                            current_token.clear(); 
                        }
                        context_tokens.push(ch.to_string());
                    } else {
                        current_token.push(ch);
                    }
                }
                if !current_token.is_empty() { 
                    context_tokens.push(current_token); 
                }

                // =============================================================
                // СИНХРОННАЯ НАКАЧКА ДЕФЕКТА
                // Передаем пачку контекста. Третий параметр Some(false), чтобы 
                // активировать латеральные синаптические маркеры tag_trace,
                // но при этом не начислять ложный дофаминовый вес!
                // =============================================================
                println!("[AGENT]: Пакетная инъекция контекста дефекта в ОЗУ...");
                self.db.inject_string_context(context_tokens.clone(), 1.2, Some(false));
                
                // Гарантируем, что импульсы контекста полностью распределились по графу
                self.db.wait_flush_barrier();

                // Просим Rayon-ядро сгенерировать резонирующую мутацию
                println!("[AGENT]: Запрос автономной резонансной гипотезы...");
                let generated_tokens = self.db.generate_code_hypothesis("let", context_tokens.clone());
                
                let raw_hypothesis = generated_tokens.join(" ");
                let flat_tokens: Vec<&str> = raw_hypothesis.split_whitespace().collect();
                let hypothesis_code = flat_tokens.join(" ");

                // Применяем решение к виртуальному файлу
                self.apply_hypothesis(&hypothesis_code);

                match self.run_cargo_check() {
                    Ok(_) => {
                        println!("[AGENT]: Компилятор удовлетворен! Дофаминовое закрепление мутации.");
                        let flat_strings: Vec<String> = hypothesis_code
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .collect();
                        
                        self.db.inject_string_context(flat_strings, 1.2, Some(true));
                        self.db.wait_flush_barrier();
                        hypothesis_code
                    }
                    Err(_) => {
                        println!("[AGENT]: Мутация отвергнута компилятором. Штрафной откат.");
                        let flat_strings: Vec<String> = hypothesis_code
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .collect();
                        
                        self.db.inject_string_context(flat_strings, 1.2, Some(false));
                        self.db.wait_flush_barrier();
                        self.apply_hypothesis(&broken_line);
                        broken_line
                    }
                }
            }
        }
    }
}
