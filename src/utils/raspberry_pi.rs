use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct FixedRingBuffer<T, const N: usize> {
    data: [Option<T>; N],
    head: usize,
    len: usize,
}

impl<T: Copy + Default, const N: usize> FixedRingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            data: [None; N],
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        let index = (self.head + self.len) % N;
        if self.len == N {
            self.head = (self.head + 1) % N;
        } else {
            self.len += 1;
        }
        self.data[index] = Some(item);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len).filter_map(move |i| {
            let idx = (self.head + i) % N;
            self.data[idx].as_ref()
        })
    }

    pub fn clear(&mut self) {
        self.data = [None; N];
        self.head = 0;
        self.len = 0;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IntegerPower {
    milliwatts: i32,
}

impl IntegerPower {
    pub fn from_watts(watts: f64) -> Self {
        Self {
            milliwatts: (watts * 1000.0) as i32,
        }
    }

    pub fn from_milliwatts(mw: i32) -> Self {
        Self { milliwatts: mw }
    }

    pub fn to_watts(&self) -> f64 {
        self.milliwatts as f64 / 1000.0
    }

    pub fn to_kilowatts(&self) -> f64 {
        self.milliwatts as f64 / 1_000_000.0
    }

    pub fn milliwatts(&self) -> i32 {
        self.milliwatts
    }

    pub fn add(&self, other: IntegerPower) -> IntegerPower {
        Self {
            milliwatts: self.milliwatts.saturating_add(other.milliwatts),
        }
    }

    pub fn sub(&self, other: IntegerPower) -> IntegerPower {
        Self {
            milliwatts: self.milliwatts.saturating_sub(other.milliwatts),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IntegerVoltage {
    millivolts: i32,
}

impl IntegerVoltage {
    pub fn from_volts(volts: f64) -> Self {
        Self {
            millivolts: (volts * 1000.0) as i32,
        }
    }

    pub fn from_millivolts(mv: i32) -> Self {
        Self { millivolts: mv }
    }

    pub fn to_volts(&self) -> f64 {
        self.millivolts as f64 / 1000.0
    }

    pub fn millivolts(&self) -> i32 {
        self.millivolts
    }
}

pub struct TelemetryAggregator<T> {
    buffer: VecDeque<T>,
    max_buffer_size: usize,
    flush_interval: Duration,
    last_flush: Instant,
}

impl<T> TelemetryAggregator<T> {
    pub fn new(flush_interval_secs: u64, max_buffer_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_buffer_size),
            max_buffer_size,
            flush_interval: Duration::from_secs(flush_interval_secs),
            last_flush: Instant::now(),
        }
    }

    pub fn push(&mut self, item: T) {
        if self.buffer.len() >= self.max_buffer_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }

    pub fn should_flush(&self) -> bool {
        self.last_flush.elapsed() >= self.flush_interval || self.buffer.len() >= self.max_buffer_size
    }

    pub fn flush(&mut self) -> Vec<T> {
        self.last_flush = Instant::now();
        self.buffer.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(feature = "db")]
pub mod database {
    use anyhow::Result;

    pub async fn configure_sqlite_for_raspberry_pi<'c>(pool: &sqlx::SqlitePool) -> Result<()> {
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(pool)
            .await?;

        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(pool)
            .await?;

        sqlx::query("PRAGMA cache_size = -8000")
            .execute(pool)
            .await?;

        sqlx::query("PRAGMA temp_store = MEMORY")
            .execute(pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_ring_buffer() {
        let mut buffer = FixedRingBuffer::<i32, 4>::new();
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4);

        assert_eq!(buffer.len(), 4);

        buffer.push(5);
        assert_eq!(buffer.len(), 4);

        let values: Vec<_> = buffer.iter().copied().collect();
        assert_eq!(values, vec![2, 3, 4, 5]);
    }

    #[test]
    fn test_integer_power() {
        let power = IntegerPower::from_watts(2.5);
        assert_eq!(power.milliwatts(), 2500);
        assert_eq!(power.to_watts(), 2.5);

        let sum = power.add(IntegerPower::from_watts(1.5));
        assert_eq!(sum.to_watts(), 4.0);
    }

    #[test]
    fn test_integer_voltage() {
        let voltage = IntegerVoltage::from_volts(230.5);
        assert_eq!(voltage.millivolts(), 230500);
        assert_eq!(voltage.to_volts(), 230.5);
    }

    #[test]
    fn test_telemetry_aggregator() {
        let mut agg = TelemetryAggregator::new(60, 100);

        for i in 0..50 {
            agg.push(i);
        }

        assert_eq!(agg.len(), 50);
        assert!(!agg.is_empty());

        let flushed = agg.flush();
        assert_eq!(flushed.len(), 50);
        assert!(agg.is_empty());
    }
}
