mod models;
mod network;
mod tokenizer;

use models::NeuronType;
use network::SpikingNetwork;
use tokenizer::SpikeTokenizer;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    println!("=== Запуск SpikeDB: Чтение и разбор кода ===");

    let unique_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
        
    let db_path = format!("./spikedb_data_{}", unique_timestamp);
    let mut brain = SpikingNetwork::new(&db_path);

    // Строка кода Rust для обучения нашей импульсной сети
    let rust_code = "let x = 5;";
    println!("Обучающий поток данных: '{}'\n", rust_code);

    // Прогоняем чтение строки 3 раза подряд, чтобы сработал Чанкинг совпадений
    for episode in 1..=3 {
        println!("--- Чтение строки, итерация №{} ---", episode);
        
        // Превращаем текст в последовательность ID биологических сенсоров
        let neuron_sequence = SpikeTokenizer::tokenize_and_register(&brain, rust_code);
        
        if episode == 1 {
            println!("  [Словарь создан]: Текст успешно спроецирован в ID нейронов: {:?}", neuron_sequence);
        }

        // Поочередно бьем током в каждый токен, симулируя чтение слева направо
        for &neuron_id in &neuron_sequence {
            brain.inject_stimulus(neuron_id, 1.2);
            brain.tick(); // Шаг времени для обработки спайка и накопления статистики
        }

        // Остужаем сеть между строками
        while brain.active_spikes_count() > 0 {
            brain.tick();
        }
    }

    println!("\n=== Тест завершен ===");
    println!("Текст успешно преобразован в устойчивые ассоциативные мета-понятия в Sled.");

    drop(brain); 
    std::thread::sleep(Duration::from_millis(20));
    let _ = fs::remove_dir_all(&db_path);
}
