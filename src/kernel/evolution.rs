use std::collections::HashMap;
use crate::memory::SpikeMemory;
use crate::models::NeuronType;
use crate::config::BrainConfig;

pub fn apply_reinforcement_kernel(memory: &mut SpikeMemory, current_tick: u64, is_success: bool, cfg: &BrainConfig) {
    let mut reinforced_count = 0;

    for source_id in 0..memory.adj_list.len() {
        let links = &mut memory.adj_list[source_id];

        for synapse in links.iter_mut() {
            let old_tag = synapse.tag_trace;
            synapse.decay_tag_lazy(current_tick, cfg.tag_tau);
            let decayed_tag = synapse.tag_trace;

            if decayed_tag > 0.0001 {
                let old_weight = synapse.weight;
                if is_success {
                    synapse.weight += decayed_tag * cfg.learning_rate;
                    // ИСПРАВЛЕНИЕ: Используем динамический потолок из конфига вместо 3.0!
                    if synapse.weight > cfg.max_synapse_weight { 
                        synapse.weight = cfg.max_synapse_weight; 
                    }
                } else {
                    synapse.weight -= decayed_tag * cfg.punish_rate;
                    if synapse.weight < 0.0 { synapse.weight = 0.0; }
                }

                if (synapse.weight - old_weight).abs() > 0.01 {
                    println!("    ➔ КРИТИКА: ID {} -> ID {} | Вес: {:.2} -> {:.2}", 
                        source_id, synapse.target_id, old_weight, synapse.weight);
                }

                synapse.tag_trace = 0.0;
                reinforced_count += 1;
            }
        }

        // =================================================================
        // ДИНАМИЧЕСКИЙ ГОМЕОСТАЗ ВЕЕРА (БЕЗ МАГИЧЕСКИХ ЧИСЕЛ)
        // ИСПРАВЛЕНИЕ: Лимит отсечения веера считывается из max_synaptic_fanout!
        // =================================================================
        if links.len() > cfg.max_synaptic_fanout {
            links.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
            links.truncate(cfg.max_synaptic_fanout); 
        }
    }
    
    if reinforced_count > 0 {
        println!("📢 [ТРАССИРОВКА КРИТИКА]: Всего синапсов изменено: {}", reinforced_count);
    }
}

pub fn sleep_and_prune_kernel(memory: &mut SpikeMemory) {
    println!("\n🌙 [ТРАССИРОВКА СНА]: Старт ночного гомеостаза...");
    let cfg = crate::config::CONFIG.get().expect("Конфиг не инициализирован");
    let mut neuron_activity_counter: HashMap<u64, u32> = HashMap::new();
    let mut pruned_synapses = 0;

    for source_id in 0..memory.adj_list.len() {
        let links = &mut memory.adj_list[source_id];
        
        for synapse in links.iter_mut() {
            let mut weight = synapse.weight;
            let old_weight = weight;

            if weight >= 1.5 {
                weight *= cfg.sleep_strong_decay;
            } else if weight >= 0.8 {
                weight *= cfg.sleep_medium_decay;
            } else {
                weight -= cfg.sleep_weak_shredder;
            }
            
            println!("    💤 СОН: ID {} -> ID {} | Вес до сна: {:.2} -> После сна: {:.2}", 
                source_id, synapse.target_id, old_weight, weight);
            
            synapse.weight = weight;
            synapse.tag_trace = 0.0;
        }

        links.retain(|synapse| {
            let death_threshold = cfg.sleep_death_threshold.max(0.35);
            if synapse.weight < death_threshold {
                pruned_synapses += 1;
                false
            } else {
                *neuron_activity_counter.entry(source_id as u64).or_insert(0) += 1;
                *neuron_activity_counter.entry(synapse.target_id).or_insert(0) += 1;
                true
            }
        });
    }

    let mut removed_count = 0;
    for id in 0..memory.neurons.len() {
        if let Some(neuron) = memory.neurons.get_mut(id) {
            if neuron.neuron_type == NeuronType::Hidden && !neuron_activity_counter.contains_key(&(id as u64)) {
                neuron.potential = 0.0;
                neuron.cooldown_until = u64::MAX;
                removed_count += 1;
            }
        }
    }
    println!("🌙 [ТРАССИРОВКА СНА]: Синапсов выжжено: {}, Мета-нейронов заморожено: {}", pruned_synapses, removed_count);
}