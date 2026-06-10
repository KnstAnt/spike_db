use crate::config::BrainConfig;
use crate::memory::SpikeMemory;
use rayon::prelude::*;

pub fn generate_trail_kernel(
    memory: &SpikeMemory,
    start_token: &str,
    context_strings: &[String],
    cfg: &BrainConfig,
) -> Vec<String> {
    let mut trail = Vec::new();
    let forbidden_ids: Vec<u64> = context_strings
        .iter()
        .filter_map(|t| memory.lookup_token_id(t))
        .collect();
    let bad_literal_id = memory.lookup_token_id("\"bad_literal\"");
    let mut visited_path = Vec::new();

    let mut current_id = match memory.lookup_token_id(start_token) {
        Some(id) => id,
        None => return vec![start_token.to_string()],
    };

    trail.push(start_token.to_string());
    visited_path.push(current_id);

    println!(
        "\n🔮 [ТРАССИРОВКА ЭКСПЕРТИЗЫ]: Начало генерации ассоциаций для '{}' (ID: {})",
        start_token, current_id
    );

    for step in 0..10 {
        let mut strongest_target = None;
        let mut max_score = -9999.0;

        if let Some(links) = memory.adj_list.get(current_id as usize) {
            let current_token_name = memory.reverse_lookup_token(current_id);
            println!(
                "  📍 Шаг {}: Мы стоим на узле '{}' (ID: {}). Всего исходящих синапсов: {}",
                step,
                current_token_name,
                current_id,
                links.len()
            );

            for synapse in links.iter() {
                let is_visited = visited_path.contains(&synapse.target_id);
                let mut score = synapse.weight + (synapse.tag_trace * 1.5);
                if is_visited {
                    score -= 5.0;
                }

                if memory.is_chunk_containing_forbidden_ids(synapse.target_id, &forbidden_ids) {
                    score -= cfg.spike_threshold * 50.0; // Жестко душим чанк, если внутри есть яд дефекта
                }
                if let Some(bad_id) = bad_literal_id {
                    if synapse.target_id == bad_id {
                        score -= cfg.spike_threshold * 100.0;
                    }
                }

                let target_name = memory.reverse_lookup_token(synapse.target_id);
                println!(
                    "      ➔ Дорога к: '{}' (ID: {}), Вес: {:.2}, Посещен: {} = БАЛЛ: {:.2}",
                    target_name, synapse.target_id, synapse.weight, is_visited, score
                );
            }

            let path_ref = &visited_path;
            let best_match = links
                .par_iter()
                .map(|synapse| {
                    let is_visited = path_ref.contains(&synapse.target_id);
                    let mut score = synapse.weight + (synapse.tag_trace * 1.5);
                    if is_visited {
                        score -= 5.0;
                    }

                    // ИСПРАВЛЕНИЕ: Душим мета-нейроны в параллельных потоках
                    if memory.is_chunk_containing_forbidden_ids(synapse.target_id, &forbidden_ids) {
                        score -= cfg.spike_threshold * 50.0;
                    }
                    if let Some(bad_id) = bad_literal_id {
                        if memory.is_chunk_containing_forbidden_ids(synapse.target_id, &[bad_id]) {
                            score -= cfg.spike_threshold * 100.0;
                        }
                    }
                    (synapse.target_id, score)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some((target_id, score)) = best_match {
                max_score = score;
                if max_score >= 0.0 {
                    strongest_target = Some(target_id);
                }
            }
        }

        if let Some(next_id) = strongest_target {
            visited_path.push(next_id);
            let mut should_break = false;

            if let Some(neuron) = memory.neurons.get(next_id as usize) {
                let next_name = memory.reverse_lookup_token(next_id);
                println!(
                    "    💎 ВЫБРАН ЛУЧШИЙ ПУТЬ: '{}' (ID: {}) с баллом {:.2}",
                    next_name, next_id, max_score
                );

                match &neuron.origin {
                    crate::models::NeuronOrigin::BaseToken(name) => {
                        if !trail.contains(name) {
                            trail.push(name.clone());
                        }
                        if name == ";" {
                            should_break = true;
                        }
                        current_id = next_id;
                    }
                    crate::models::NeuronOrigin::ChunkSequence(_, _) => {
                        let full_phrase = memory.reverse_lookup_token(next_id);
                        println!("      [РАСПРЯМЛЕНИЕ ЧАНКА]: Мета-понятие разворачивается во фразу: '{}'", full_phrase);

                        for word in full_phrase.split_whitespace() {
                            if !trail.contains(&word.to_string()) && word != "\"bad_literal\"" {
                                trail.push(word.to_string());
                            }
                            if word == ";" {
                                should_break = true;
                            }
                        }
                        current_id = memory.get_chunk_terminal_token_id(next_id);
                        println!("      [ПЕРЕКЛЮЧЕНИЕ КОНТЕКСТА]: Мысль сместилась на терминальный ID: {}", current_id);
                    }
                }
            }

            if should_break {
                println!("    [ФИНИШ]: Встречен терминальный символ ';'. Стрим завершен.");
                break;
            }
        } else {
            println!(
                "    [ТУПИК]: Из текущего узла все дороги заблокированы (max_score: {:.2}).",
                max_score
            );
            break;
        }
    }

    println!(
        "🔮 [ТРАССИРОВКА ЭКСПЕРТИЗЫ]: Итоговый шлейф на выходе генератора: {:?}",
        trail
    );
    trail
}
