use crate::network::SpikingNetwork;
use crate::config::BrainConfig;
use crossbeam_channel::{unbounded, Sender, select};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Команды внутреннего протокола асинхронного менеджера базы данных
enum DbCommand {
    InjectText { token: String, charge: f32 },
    ApplyReinforcement { is_success: bool },
    SleepAndPrune,
    // Передаем текстовый токен, Ядро само безопасно найдет его ID внутри своего потока
    InspectRequest { token: String, tx_response: crossbeam_channel::Sender<Option<(u64, f32)>> },
    Shutdown,
}

pub struct SpikeDB {
    tx: Sender<DbCommand>,
    thread_handle: Option<JoinHandle<()>>,
}

impl SpikeDB {
    /// Открывает базу данных Sled и запускает изолированного Актора
    pub fn open(path: &str) -> Self {
        let (tx, rx) = unbounded::<DbCommand>();
        let path_str = path.to_string();
        let config = BrainConfig::load_from_file();

        let thread_handle = thread::spawn(move || {
            // ИСПРАВЛЕНИЕ: База данных открывается ОДИН РАЗ за всю жизнь программы!
            let mut network = SpikingNetwork::new(&path_str, config);
            println!("[SpikeDB]: Фоновый асинхронный поток базы данных успешно запущен.");

            loop {
                // Настройка таймера тика в зависимости от активности сети
                let tick_timeout = if network.active_spikes_count() > 0 {
                    Duration::from_millis(1)
                } else {
                    Duration::from_secs(3600)
                };

                select! {
                    recv(rx) -> msg => {
                        match msg {
                            Ok(DbCommand::InjectText { token, charge }) => {
                                let id = network.get_or_create_token_neuron(&token);
                                network.inject_stimulus(id, charge);
                            }
                            Ok(DbCommand::ApplyReinforcement { is_success }) => {
                                network.apply_reinforcement(is_success);
                            }
                            Ok(DbCommand::SleepAndPrune) => {
                                network.sleep_and_prune();
                            }
                            Ok(DbCommand::InspectRequest { token, tx_response }) => {
                                let id = network.get_or_create_token_neuron(&token);
                                let res = network.get_strongest_prediction(id);
                                let _ = tx_response.send(res);
                            }
                            Ok(DbCommand::Shutdown) | Err(_) => {
                                println!("[SpikeDB]: Фиксация транзакций Sled и остановка потока...");
                                break;
                            }
                        }
                    }
                    default(tick_timeout) => {
                        if network.active_spikes_count() > 0 {
                            network.tick();
                        }
                    }
                }
            }
        });

        Self {
            tx,
            thread_handle: Some(thread_handle),
        }
    }

    pub fn inject_token(&self, token: &str, charge: f32) {
        let _ = self.tx.send(DbCommand::InjectText { token: token.to_string(), charge });
    }

    pub fn approve_success(&self, is_success: bool) {
        let _ = self.tx.send(DbCommand::ApplyReinforcement { is_success });
    }

    pub fn trigger_sleep(&self) {
        let _ = self.tx.send(DbCommand::SleepAndPrune);
    }

    /// Безопасный асинхронный запрос инспекции через каналы
    pub fn inspect_prediction(&self, token: &str) -> Option<(u64, f32)> {
        let (tx_response, rx_response) = crossbeam_channel::bounded::<Option<(u64, f32)>>(1);
        
        if self.tx.send(DbCommand::InspectRequest { token: token.to_string(), tx_response }).is_ok() {
            rx_response.recv().unwrap_or(None)
        } else {
            None
        }
    }
}

impl Drop for SpikeDB {
    fn drop(&mut self) {
        let _ = self.tx.send(DbCommand::Shutdown);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}
