use crate::network::SpikingNetwork;
use crate::config::BrainConfig;
use crossbeam_channel::{unbounded, Sender, select};
use std::thread::{self, JoinHandle};
use std::time::Duration;

enum SpikeCommand {
    InjectLine { tokens: Vec<String>, charge: f32, reinforce: Option<bool> },
    // ИСПРАВЛЕНИЕ: Добавляем канал обратного ответа для фиксации окончания сна!
    SleepAndPrune { tx_response: crossbeam_channel::Sender<()> },
    GenerateTrail { start_token: String, context_tokens: Vec<String>, tx_response: crossbeam_channel::Sender<Vec<String>> },
    SyncBarrier { tx_response: crossbeam_channel::Sender<()> },    
    Shutdown,
}

pub struct SpikeDB {
    tx: Sender<SpikeCommand>,
    thread_handle: Option<JoinHandle<()>>,
}

impl SpikeDB {
    pub fn open(_dummy_path: &str) -> Self {
        let (tx, rx) = unbounded::<SpikeCommand>();
        let config = BrainConfig::load_from_file();
        let (tx_ready, rx_ready) = crossbeam_channel::bounded::<()>(1);

        let thread_handle = thread::spawn(move || {
            let mut network = SpikingNetwork::new(config);
            println!("[SpikeDB]: Фоновый асинхронный поток In-Memory вычислений успешно запущен.");
            let _ = tx_ready.send(());

            loop {
                let mut should_shutdown = false;

                let tick_timeout = if network.active_spikes_count() > 0 {
                    Duration::from_millis(1)
                } else {
                    Duration::from_secs(3600)
                };

                select! {
                    recv(rx) -> msg => {
                        match msg {
                            Ok(SpikeCommand::InjectLine { tokens, charge, reinforce }) => {
                                network.clear_runtime_attention_buffers();

                                for token in tokens {
                                    let id = network.memory.get_or_create_token(&token);
                                    network.inject_stimulus(id, charge);
                                    network.tick();
                                }
                                
                                while network.active_spikes_count() > 0 {
                                    network.tick();
                                }

                                if let Some(is_success) = reinforce {
                                    network.apply_reinforcement(is_success);
                                }
                            }
                            Ok(SpikeCommand::SleepAndPrune { tx_response }) => {
                                while network.active_spikes_count() > 0 {
                                    network.tick();
                                }
                                network.sleep_and_prune();
                                // Маякуем тесту, что гомеостаз сна успешно завершен
                                let _ = tx_response.send(());
                            }
                            Ok(SpikeCommand::GenerateTrail { start_token, context_tokens, tx_response }) => {
                                let trail = network.generate_autonomous_mutation(&start_token, &context_tokens);
                                let _ = tx_response.send(trail);
                            }
                            Ok(SpikeCommand::SyncBarrier { tx_response }) => {
                                while network.active_spikes_count() > 0 { network.tick(); }
                                let _ = tx_response.send(());
                            }
                            Ok(SpikeCommand::Shutdown) | Err(_) => {
                                println!("[SpikeDB]: Остановка фонового потока...");
                                should_shutdown = true;
                            }
                        }
                    }
                    default(tick_timeout) => {
                        if network.active_spikes_count() > 0 {
                            network.tick();
                        }
                    }
                }

                if should_shutdown {
                    break;
                }
            }
        });

        let _ = rx_ready.recv();

        Self {
            tx,
            thread_handle: Some(thread_handle),
        }
    }

    pub fn inject_string_context(&self, tokens: Vec<String>, charge: f32, reinforce: Option<bool>) {
        let _ = self.tx.send(SpikeCommand::InjectLine { tokens, charge, reinforce });
    }

    /// ИСПРАВЛЕНИЕ: Теперь метод trigger_sleep жестко блокирует вызывающий поток 
    /// до тех пор, пока Ядро полностью не завершит ночной прунинг! Без костыльных sleep.
    pub fn trigger_sleep(&self) {
        let (tx_response, rx_response) = crossbeam_channel::bounded::<()>(1);
        if self.tx.send(SpikeCommand::SleepAndPrune { tx_response }).is_ok() {
            let _ = rx_response.recv();
        }
    }

    pub fn generate_code_hypothesis(&self, start_token: &str, context_tokens: Vec<String>) -> Vec<String> {
        let (tx_response, rx_response) = crossbeam_channel::bounded::<Vec<String>>(1);
        let _ = self.tx.send(SpikeCommand::GenerateTrail {
            start_token: start_token.to_string(),
            context_tokens,
            tx_response,
        });
        rx_response.recv().unwrap_or_default()
    }

    /// ПУБЛИЧНЫЙ МЕТОД СИНХРОНИЗАЦИИ: Гарантирует, что Ядро "дожует" все мысли 
    /// перед тем, как тест двинется дальше. Без костыльных задержек sleep.
    pub fn wait_flush_barrier(&self) {
        let (tx_response, rx_response) = crossbeam_channel::bounded::<()>(1);
        if self.tx.send(SpikeCommand::SyncBarrier { tx_response }).is_ok() {
            let _ = rx_response.recv();
        }
    }    
}

impl Drop for SpikeDB {
    fn drop(&mut self) {
        let _ = self.tx.send(SpikeCommand::Shutdown);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}
