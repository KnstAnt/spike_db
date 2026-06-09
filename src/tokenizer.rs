use crate::network::SpikingNetwork;

pub struct SpikeTokenizer;

impl SpikeTokenizer {
    /// Принимает строку кода, бьет её на лексемы и преобразует в вектор ID нейронов через SpikeDB
    pub fn tokenize_and_register(brain: &SpikingNetwork, code: &str) -> Vec<u64> {
        let mut token_ids = Vec::new();
        let mut current_token = String::new();

        // Символы, которые являются самостоятельными токенами-операторами
        let special_chars = ['=', ';', '{', '}', '(', ')', '+', '-', '>', '<'];

        for ch in code.chars() {
            if ch.is_whitespace() {
                // Если уперлись в пробел — закрываем накопленный токен
                if !current_token.is_empty() {
                    let id = brain.get_or_create_token_neuron(&current_token);
                    token_ids.push(id);
                    current_token.clear();
                }
            } else if special_chars.contains(&ch) {
                // Если встретили спец-символ (например, равенство или точку с запятой)
                // Сначала закрываем текущее накопленное слово (если оно было)
                if !current_token.is_empty() {
                    let id = brain.get_or_create_token_neuron(&current_token);
                    token_ids.push(id);
                    current_token.clear();
                }
                // Теперь регистрируем сам спец-символ как отдельный независимый токен
                let id = brain.get_or_create_token_neuron(&ch.to_string());
                token_ids.push(id);
            } else {
                // Иначе просто продолжаем собирать слово по буквам
                current_token.push(ch);
            }
        }

        // Обрабатываем последний токен, если строка закончилась не на пробел/знак
        if !current_token.is_empty() {
            let id = brain.get_or_create_token_neuron(&current_token);
            token_ids.push(id);
        }

        token_ids
    }
}
