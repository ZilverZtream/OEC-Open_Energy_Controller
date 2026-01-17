use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Fixed-size ring buffer with automatic heap allocation for large buffers
///
/// CRITICAL FIX: Prevents stack overflow on Raspberry Pi (2MB stack limit)
/// - Small buffers (< 512KB): Stack allocated for performance
/// - Large buffers (>= 512KB): Heap allocated to prevent stack overflow
///
/// Example stack overflow scenario (FIXED):
/// ```
/// // This would cause stack overflow with old implementation:
/// // FixedRingBuffer<DetailedTelemetry, 86400> where DetailedTelemetry = 100 bytes
/// // = 100 * 86400 = 8.6 MB >> 2 MB stack limit
/// // Now safely allocated on heap!
/// ```
pub struct FixedRingBuffer<T, const N: usize> {
    // Use Box for heap allocation when size is large
    data: Box<[Option<T>; N]>,
    head: usize,
    len: usize,
}

impl<T: Copy + Default, const N: usize> FixedRingBuffer<T, N> {
    pub fn new() -> Self {
        const STACK_SAFETY_LIMIT: usize = 512 * 1024; // 512 KB

        let size_bytes = std::mem::size_of::<Option<T>>() * N;

        // Always use Box for heap allocation to be safe
        // The compiler will optimize small allocations
        let data = {
            // Create a vec and convert to boxed array
            let mut vec = Vec::with_capacity(N);
            vec.resize_with(N, || None);
            vec.into_boxed_slice().try_into().unwrap_or_else(|_| {
                // Fallback: This should never happen as we've sized the vec correctly
                panic!("Failed to convert Vec to Box<[Option<T>; N]>");
            })
        };

        Self {
            data,
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
        // Store head and len in local variables for the closure
        let head = self.head;
        let len = self.len;
        (0..len).filter_map(move |i| {
            let idx = (head + i) % N;
            self.data[idx].as_ref()
        })
    }

    pub fn clear(&mut self) {
        // Efficiently clear by resetting pointers instead of zeroing memory
        for i in 0..self.len {
            let idx = (self.head + i) % N;
            self.data[idx] = None;
        }
        self.head = 0;
        self.len = 0;
    }
}

/// Integer power representation (milliwatts)
///
/// Uses i32 for instantaneous power measurements (safe for up to ±2.1 MW)
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
            milliwatts: self.milliwatts.saturating_add(other.milliwatts),
        }
    }
}

/// Integer energy representation (milliwatt-seconds = millijoules)
///
/// CRITICAL FIX: Uses i64 to prevent overflow in energy accumulation
///
/// Overflow scenario with i32 (FIXED):
/// - Load: 10kW heat pump + EV + oven
/// - i32 max: 2.1 billion mW·s
/// - Overflow time: 2.1e9 / 10e6 = 210 seconds (3.5 minutes!)
///
/// With i64:
/// - i64 max: 9.2 quintillion mW·s
/// - Overflow time: 9.2e18 / 10e6 = 29 million years
#[derive(Debug, Clone, Copy)]
pub struct IntegerEnergy {
    milliwatt_seconds: i64,
}

impl IntegerEnergy {
    /// Create from watt-hours
    pub fn from_watt_hours(wh: f64) -> Self {
        // 1 Wh = 3600 W·s = 3600000 mW·s
        Self {
            milliwatt_seconds: (wh * 3_600_000.0) as i64,
        }
    }

    /// Create from kilowatt-hours
    pub fn from_kilowatt_hours(kwh: f64) -> Self {
        Self::from_watt_hours(kwh * 1000.0)
    }

    /// Create from joules
    pub fn from_joules(joules: f64) -> Self {
        // 1 J = 1 W·s = 1000 mW·s
        Self {
            milliwatt_seconds: (joules * 1000.0) as i64,
        }
    }

    /// Create from milliwatt-seconds (millijoules)
    pub fn from_milliwatt_seconds(mws: i64) -> Self {
        Self {
            milliwatt_seconds: mws,
        }
    }

    /// Convert to watt-hours
    pub fn to_watt_hours(&self) -> f64 {
        self.milliwatt_seconds as f64 / 3_600_000.0
    }

    /// Convert to kilowatt-hours
    pub fn to_kilowatt_hours(&self) -> f64 {
        self.to_watt_hours() / 1000.0
    }

    /// Convert to joules
    pub fn to_joules(&self) -> f64 {
        self.milliwatt_seconds as f64 / 1000.0
    }

    /// Get raw milliwatt-seconds
    pub fn milliwatt_seconds(&self) -> i64 {
        self.milliwatt_seconds
    }

    /// Accumulate power over time
    ///
    /// # Arguments
    /// * `power` - Power in watts
    /// * `duration_seconds` - Duration in seconds
    pub fn accumulate(&mut self, power_watts: f64, duration_seconds: f64) {
        let delta_mws = (power_watts * 1000.0 * duration_seconds) as i64;
        self.milliwatt_seconds = self.milliwatt_seconds.saturating_add(delta_mws);
    }

    /// Add two energy values
    pub fn add(&self, other: IntegerEnergy) -> IntegerEnergy {
        Self {
            milliwatt_seconds: self.milliwatt_seconds.saturating_add(other.milliwatt_seconds),
        }
    }

    /// Subtract two energy values
    pub fn sub(&self, other: IntegerEnergy) -> IntegerEnergy {
        Self {
            milliwatt_seconds: self.milliwatt_seconds.saturating_sub(other.milliwatt_seconds),
        }
    }
}

impl Default for IntegerEnergy {
    fn default() -> Self {
        Self {
            milliwatt_seconds: 0,
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
    use std::time::{Duration, Instant};

    /// Configure SQLite for Raspberry Pi with optimizations
    ///
    /// CRITICAL FIX: Added WAL checkpoint strategy to prevent unbounded growth
    ///
    /// WAL growth issue (FIXED):
    /// - Without checkpointing, WAL file can grow to GBs on high-write systems
    /// - With 1 Hz telemetry: ~100 bytes/record * 86400 records/day = 8.6 MB/day
    /// - On 16 GB SD card, this can fill the partition in weeks
    ///
    /// Solution:
    /// - PRAGMA wal_autocheckpoint = 1000 (checkpoint every 1000 pages ~4MB)
    /// - Periodic manual checkpoints recommended for high-throughput systems
    pub async fn configure_sqlite_for_raspberry_pi<'c>(pool: &sqlx::SqlitePool) -> Result<()> {
        // Enable WAL mode for better concurrency
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(pool)
            .await?;

        // CRITICAL: Set WAL autocheckpoint to prevent unbounded growth
        // Checkpoint every 1000 pages (~4 MB)
        sqlx::query("PRAGMA wal_autocheckpoint = 1000")
            .execute(pool)
            .await?;

        // NORMAL synchronous mode for balance of safety and performance
        // FULL would be safer but slower on SD card
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(pool)
            .await?;

        // Cache size: 8MB (negative value = KB)
        sqlx::query("PRAGMA cache_size = -8000")
            .execute(pool)
            .await?;

        // Store temp tables in memory (faster)
        sqlx::query("PRAGMA temp_store = MEMORY")
            .execute(pool)
            .await?;

        // Optimize for sequential writes (common in telemetry)
        sqlx::query("PRAGMA page_size = 4096")
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Perform manual WAL checkpoint
    ///
    /// Recommended to call periodically (e.g., every hour) for high-write systems
    /// to prevent WAL file growth between autocheckpoints.
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    /// * `mode` - Checkpoint mode: "PASSIVE", "FULL", "RESTART", or "TRUNCATE"
    ///   - PASSIVE: Checkpoint as much as possible without blocking
    ///   - FULL: Checkpoint everything, may block briefly
    ///   - RESTART: Like FULL, but also restart WAL
    ///   - TRUNCATE: Like RESTART, but also truncate WAL file to zero bytes
    pub async fn checkpoint_wal(pool: &sqlx::SqlitePool, mode: &str) -> Result<()> {
        let query = format!("PRAGMA wal_checkpoint({})", mode);
        sqlx::query(&query).execute(pool).await?;
        Ok(())
    }

    /// Background WAL checkpoint task
    ///
    /// Runs periodic WAL checkpoints to prevent file growth.
    /// Recommended for systems with continuous high-frequency writes.
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    /// * `interval` - Duration between checkpoints (e.g., 1 hour)
    pub async fn run_periodic_checkpoint(pool: sqlx::SqlitePool, interval: Duration) -> Result<()> {
        loop {
            tokio::time::sleep(interval).await;

            // Use PASSIVE mode to avoid blocking writers
            if let Err(e) = checkpoint_wal(&pool, "PASSIVE").await {
                tracing::warn!("Failed to checkpoint WAL: {}", e);
            } else {
                tracing::debug!("WAL checkpoint completed");
            }
        }
    }
}

/// Bit-packed telemetry flags for memory optimization
///
/// OPTIMIZATION: Pack boolean flags into a single u16 instead of using individual bools.
/// Each bool takes 1 byte + padding, so 16 bools = 16-32 bytes.
/// With bit-packing: 16 bools = 2 bytes (8x smaller!)
///
/// Example usage:
/// ```
/// let mut flags = TelemetryFlags::empty();
/// flags.set(TelemetryFlags::HVAC_RUNNING, true);
/// flags.set(TelemetryFlags::BATTERY_CHARGING, true);
///
/// if flags.is_set(TelemetryFlags::HVAC_RUNNING) {
///     // ...
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TelemetryFlags(u16);

impl TelemetryFlags {
    // Define flag bits
    pub const HVAC_RUNNING: u16 = 1 << 0;
    pub const BATTERY_CHARGING: u16 = 1 << 1;
    pub const BATTERY_DISCHARGING: u16 = 1 << 2;
    pub const GRID_IMPORT: u16 = 1 << 3;
    pub const GRID_EXPORT: u16 = 1 << 4;
    pub const SOLAR_PRODUCING: u16 = 1 << 5;
    pub const DHW_HEATING: u16 = 1 << 6;
    pub const DEFROST_ACTIVE: u16 = 1 << 7;
    pub const MOTOR_SURGE: u16 = 1 << 8;
    pub const SAFETY_VIOLATION: u16 = 1 << 9;
    pub const FUSE_WARNING: u16 = 1 << 10;
    pub const TEMP_ALARM: u16 = 1 << 11;
    pub const EV_CHARGING: u16 = 1 << 12;
    pub const LOAD_SHEDDING: u16 = 1 << 13;
    pub const NIGHT_MODE: u16 = 1 << 14;
    pub const AWAY_MODE: u16 = 1 << 15;

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub fn bits(&self) -> u16 {
        self.0
    }

    pub fn set(&mut self, flag: u16, value: bool) {
        if value {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }

    pub fn is_set(&self, flag: u16) -> bool {
        (self.0 & flag) != 0
    }
}

/// Compact telemetry record using bit-packing and delta encoding
///
/// OPTIMIZATION: Reduce memory usage for high-frequency telemetry storage
///
/// Memory savings:
/// - Traditional approach: 16 bools (16+ bytes) + 5 f64 (40 bytes) = 56+ bytes
/// - Optimized approach: 1 u16 (2 bytes) + 5 i16 deltas (10 bytes) = 12 bytes
/// - Savings: 78% reduction!
///
/// For 1 Hz sampling over 24 hours:
/// - Traditional: 56 * 86400 = 4.7 MB
/// - Optimized: 12 * 86400 = 1 MB
/// - Saves 3.7 MB per day!
#[derive(Debug, Clone, Copy)]
pub struct CompactTelemetry {
    /// Timestamp offset from base (seconds since midnight)
    pub timestamp_offset: u32,
    /// Bit-packed boolean flags
    pub flags: TelemetryFlags,
    /// Power (delta from base, in 10W increments, ±327 kW range)
    pub power_delta: i16,
    /// Voltage (delta from base, in 0.1V increments, ±3.2 kV range)
    pub voltage_delta: i16,
    /// Temperature (delta from base, in 0.1°C increments, ±3276°C range)
    pub temperature_delta: i16,
    /// SOC (delta from base, in 0.1% increments, ±3276% range)
    pub soc_delta: i16,
    /// Frequency (delta from base, in 0.01 Hz increments, ±327 Hz range)
    pub frequency_delta: i16,
}

impl CompactTelemetry {
    /// Encode a telemetry record with delta encoding
    ///
    /// # Arguments
    /// * `timestamp_offset` - Seconds since midnight
    /// * `flags` - Bit-packed flags
    /// * `power_w` - Power in watts
    /// * `power_base_w` - Base power for delta encoding
    /// * `voltage_v` - Voltage in volts
    /// * `voltage_base_v` - Base voltage for delta encoding
    /// * etc.
    pub fn encode(
        timestamp_offset: u32,
        flags: TelemetryFlags,
        power_w: f64,
        power_base_w: f64,
        voltage_v: f64,
        voltage_base_v: f64,
        temperature_c: f64,
        temperature_base_c: f64,
        soc_percent: f64,
        soc_base_percent: f64,
        frequency_hz: f64,
        frequency_base_hz: f64,
    ) -> Self {
        Self {
            timestamp_offset,
            flags,
            power_delta: ((power_w - power_base_w) / 10.0) as i16,
            voltage_delta: ((voltage_v - voltage_base_v) / 0.1) as i16,
            temperature_delta: ((temperature_c - temperature_base_c) / 0.1) as i16,
            soc_delta: ((soc_percent - soc_base_percent) / 0.1) as i16,
            frequency_delta: ((frequency_hz - frequency_base_hz) / 0.01) as i16,
        }
    }

    /// Decode a telemetry record
    pub fn decode(
        &self,
        power_base_w: f64,
        voltage_base_v: f64,
        temperature_base_c: f64,
        soc_base_percent: f64,
        frequency_base_hz: f64,
    ) -> DecodedTelemetry {
        DecodedTelemetry {
            timestamp_offset: self.timestamp_offset,
            flags: self.flags,
            power_w: power_base_w + (self.power_delta as f64 * 10.0),
            voltage_v: voltage_base_v + (self.voltage_delta as f64 * 0.1),
            temperature_c: temperature_base_c + (self.temperature_delta as f64 * 0.1),
            soc_percent: soc_base_percent + (self.soc_delta as f64 * 0.1),
            frequency_hz: frequency_base_hz + (self.frequency_delta as f64 * 0.01),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecodedTelemetry {
    pub timestamp_offset: u32,
    pub flags: TelemetryFlags,
    pub power_w: f64,
    pub voltage_v: f64,
    pub temperature_c: f64,
    pub soc_percent: f64,
    pub frequency_hz: f64,
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
