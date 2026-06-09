use serde::{Serialize, Deserialize};
use bincode_next::{Encode, Decode};

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq)]
pub enum NeuronType {
    Sensor,  // Входной нейрон (восприятие токенов)
    Motor,   // Выходной нейрон (команды/действия)
    Hidden,  // Внутренний нейрон (абстрактные мета-понятия)
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone)]
pub struct NeuronState {
    /// Текущий заряд (потенциал мембраны) от 0.0 до 1.0
    pub potential: f32,
    /// Номер тика, когда заряд обновлялся в последний раз (для ленивого угасания)
    pub last_updated_tick: u64,
    /// Тик, до которого нейрон «отдыхает» и игнорирует входящие сигналы
    pub cooldown_until: u64,
    /// Роль нейрона в архитектуре
    pub neuron_type: NeuronType,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone)]
pub struct Synapse {
    /// Сила связи (плюс = возбуждение, минус = торможение)
    pub weight: f32,
    /// След активности (Synaptic Tag) для будущего дофаминового обучения Критика
    pub tag_trace: f32,
    /// Тик последнего прохождения сигнала через эту связь
    pub last_used_tick: u64,
}

// Константы для математики LIF-модели
const LEAK_TAU: f32 = 20.0;       // Скорость забывания (в тиках)
const SPIKE_THRESHOLD: f32 = 1.0;    // Порог активации
const COOLDOWN_TICKS: u64 = 5;       // Длительность релаксации после спайка
const TAG_TAU: f32 = 15.0; // Константа для угасания следа памяти (в тиках)

impl NeuronState {
    pub fn new(neuron_type: NeuronType) -> Self {
        Self {
            potential: 0.0,
            last_updated_tick: 0,
            cooldown_until: 0,
            neuron_type,
        }
    }

    /// Обработка пришедшего импульса. Возвращает true, если произошел спайк.
    pub fn receive_impulse(&mut self, incoming_charge: f32, current_tick: u64) -> bool {
        // Если нейрон в режиме кулдауна — он "мертв" для сигналов
        if current_tick < self.cooldown_until {
            return false;
        }

        // Вычисляем ленивое угасание за время, пока нейрон молчал
        if current_tick > self.last_updated_tick {
            let delta_t = (current_tick - self.last_updated_tick) as f32;
            self.potential *= (-delta_t / LEAK_TAU).exp();
            self.last_updated_tick = current_tick;
        }

        // Добавляем пришедший заряд
        self.potential += incoming_charge;

        // Не даем заряду уйти в минус (например, из-за тормозящих нейронов)
        if self.potential < 0.0 {
            self.potential = 0.0;
        }

        // Проверяем, пробит ли порог спайка
        if self.potential >= SPIKE_THRESHOLD {
            self.potential = 0.0; // Сброс заряда в ноль
            self.cooldown_until = current_tick + COOLDOWN_TICKS; // Активируем кулдаун
            true // Спайк!
        } else {
            false
        }
    }
}

impl Synapse {
    /// Вызывается, когда через синапс проходит импульс.
    /// Метод обновляет след активности с учетом времени его угасания.
    pub fn trigger(&mut self, current_tick: u64) {
        // Ленивое угасание следа за время, пока связь молчала
        if current_tick > self.last_used_tick {
            let delta_t = (current_tick - self.last_used_tick) as f32;
            self.tag_trace *= (-delta_t / TAG_TAU).exp();
            self.last_used_tick = current_tick;
        }

        // Импульс прошел -> след активности вспыхивает (максимум 1.0)
        self.tag_trace = (self.tag_trace + 0.5).min(1.0);
    }

    /// Ленивое обновление следа активности «на лету» (нужно при аудите Критиком)
    pub fn decay_tag_lazy(&mut self, current_tick: u64) {
        if current_tick > self.last_used_tick {
            let delta_t = (current_tick - self.last_used_tick) as f32;
            self.tag_trace *= (-delta_t / TAG_TAU).exp();
            self.last_used_tick = current_tick;
        }
    }
}
