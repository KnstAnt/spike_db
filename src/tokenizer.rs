use crate::network::SpikingNetwork;

pub struct SpikeTokenizer;

impl SpikeTokenizer {
    /// Разбирает код на лексемы и регистрирует их в RAM-словаре SpikeMemory.
    /// ИСПРАВЛЕНИЕ: Теперь принимает &mut SpikingNetwork, так как создание токена мутирует ОЗУ.
    pub fn tokenize_and_register(brain: &mut SpikingNetwork, code: &str) -> Vec<u64> {
        let mut token_ids = Vec::new();
        let mut current_token = String::new();
        let special_chars = ['=', ';', '{', '}', '(', ')', '+', '-', '>', '<', ':', ',', '\'', '&', '!'];

        for ch in code.chars() {
            if ch.is_whitespace() {
                if !current_token.is_empty() {
                    // ИСПРАВЛЕНИЕ: Вызываем новый метод через .memory и передаем &mut
                    token_ids.push(brain.memory.get_or_create_token(&current_token));
                    current_token.clear();
                }
            } else if special_chars.contains(&ch) {
                if !current_token.is_empty() {
                    token_ids.push(brain.memory.get_or_create_token(&current_token));
                    current_token.clear();
                }
                token_ids.push(brain.memory.get_or_create_token(&ch.to_string()));
            } else {
                current_token.push(ch);
            }
        }

        if !current_token.is_empty() {
            token_ids.push(brain.memory.get_or_create_token(&current_token));
        }
        token_ids
    }
}
