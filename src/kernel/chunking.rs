use crate::config::BrainConfig;
use crate::memory::SpikeMemory;
use crate::models::NeuronType;
use std::collections::HashMap;

pub fn process_coincidences_kernel(
    memory: &mut SpikeMemory,
    coincidence_tracker: &mut HashMap<(u64, u64), u32>,
    previous_spikes: &[u64],
    current_spikes: &[u64],
    current_tick: u64,
    cfg: &BrainConfig,
) {
    for &old_id in previous_spikes {
        for &new_id in current_spikes {
            if old_id != new_id {
                let is_old_sensor = memory
                    .neurons
                    .get(old_id as usize)
                    .map_or(false, |n| n.neuron_type == NeuronType::Sensor);
                let is_new_sensor = memory
                    .neurons
                    .get(new_id as usize)
                    .map_or(false, |n| n.neuron_type == NeuronType::Sensor);

                if is_old_sensor && is_new_sensor {
                    let pair = (old_id, new_id);
                    let count = coincidence_tracker.entry(pair).or_insert(0);
                    *count += 1;

                    let old_name = memory.reverse_lookup_token(old_id);
                    let new_name = memory.reverse_lookup_token(new_id);
                    println!(
                        "   ⚡ СОВПАДЕНИЕ: '{}' -> '{}' | Текущий счетчик: {}/{}",
                        old_name, new_name, count, cfg.coincidence_threshold
                    );

                    // Наращиваем базовый вес синапса между словами
                    let current_weight = memory.get_synapse_weight(old_id, new_id);

                    // ИСПРАВЛЕНИЕ: Мягкий шаг (+0.05). Базовые связи между словами
                    // ограничиваем уровнем 1.5, защищая сеть от короткого замыкания!
                    let next_weight = if current_weight < 0.1 {
                        cfg.base_hebbian_weight
                    } else {
                        (current_weight + 0.05).min(1.5)
                    };

                    memory.set_synapse(old_id, new_id, next_weight, current_tick);
                    if *count == cfg.coincidence_threshold {
                        let meta_neuron_id = memory.create_meta_chunk(old_id, new_id);
                        println!(
                            "✨ [МЕТА-ЭВОЛЮЦИЯ]: Рождение чанка ID {} -> ID {} с новым ID: {}",
                            old_id, new_id, meta_neuron_id
                        );

                        memory.set_synapse(old_id, meta_neuron_id, 0.6, current_tick);
                        memory.set_synapse(new_id, meta_neuron_id, 0.6, current_tick);
                    }
                } else {
                    let current_weight = memory.get_synapse_weight(old_id, new_id);
                    if current_weight > 0.1 {
                        memory.set_synapse(
                            old_id,
                            new_id,
                            (current_weight + 0.05).min(1.2),
                            current_tick,
                        );
                    }
                }
            }
        }
    }
}
