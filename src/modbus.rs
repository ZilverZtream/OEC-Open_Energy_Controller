#[cfg(feature = "modbus")]
pub mod client {
    use anyhow::{Context as AnyhowContext, Result};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tokio::time::timeout;
    use tokio_modbus::client::tcp;
    use tokio_modbus::prelude::*;
    use tracing::{debug, warn, error};

    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
    const MAX_RETRIES: u32 = 3;

    pub struct ModbusClient {
        context: Arc<Mutex<tokio_modbus::client::Context>>,
        unit_id: u8,
        addr: String,
        timeout_duration: Duration,
    }

    impl ModbusClient {
        /// Connect to a Modbus TCP device with default timeout
        pub async fn connect(addr: &str, unit_id: u8) -> Result<Self> {
            Self::connect_with_timeout(addr, unit_id, DEFAULT_TIMEOUT).await
        }

        /// Connect to a Modbus TCP device with custom timeout
        pub async fn connect_with_timeout(
            addr: &str,
            unit_id: u8,
            timeout_duration: Duration,
        ) -> Result<Self> {
            let socket_addr = addr.parse().context("Invalid address format")?;

            debug!("Connecting to Modbus device at {} (unit {})", addr, unit_id);

            let ctx = timeout(timeout_duration, tcp::connect(socket_addr))
                .await
                .context("Connection timeout")?
                .context("Failed to connect")?;

            Ok(Self {
                context: Arc::new(Mutex::new(ctx)),
                unit_id,
                addr: addr.to_string(),
                timeout_duration,
            })
        }

        /// Read holding registers with automatic retry
        pub async fn read_holding_registers(&self, start: u16, count: u16) -> Result<Vec<u16>> {
            self.retry_operation(|ctx| async move {
                ctx.set_slave(Slave(self.unit_id));
                ctx.read_holding_registers(start, count).await
            })
            .await
            .context(format!("Failed to read holding registers at {}", start))
        }

        /// Read input registers with automatic retry
        pub async fn read_input_registers(&self, start: u16, count: u16) -> Result<Vec<u16>> {
            self.retry_operation(|ctx| async move {
                ctx.set_slave(Slave(self.unit_id));
                ctx.read_input_registers(start, count).await
            })
            .await
            .context(format!("Failed to read input registers at {}", start))
        }

        /// Write a single register
        pub async fn write_single_register(&self, addr: u16, value: u16) -> Result<()> {
            self.retry_operation(|ctx| async move {
                ctx.set_slave(Slave(self.unit_id));
                ctx.write_single_register(addr, value).await?;
                Ok(())
            })
            .await
            .context(format!("Failed to write register at {}", addr))
        }

        /// Write multiple registers with automatic retry
        pub async fn write_multiple_registers(&self, start: u16, values: &[u16]) -> Result<()> {
            let values = values.to_vec(); // Clone for move into closure
            self.retry_operation(|ctx| {
                let values = values.clone();
                async move {
                    ctx.set_slave(Slave(self.unit_id));
                    ctx.write_multiple_registers(start, &values).await?;
                    Ok(())
                }
            })
            .await
            .context(format!("Failed to write multiple registers at {}", start))
        }

        /// Health check - attempts to read a single register
        pub async fn health_check(&self) -> Result<()> {
            debug!("Performing health check on {}", self.addr);
            // Try to read register 0 (most devices support this)
            self.read_holding_registers(0, 1)
                .await
                .map(|_| ())
                .context("Health check failed")
        }

        /// Reconnect to the device
        pub async fn reconnect(&self) -> Result<()> {
            warn!("Reconnecting to Modbus device at {}", self.addr);
            let socket_addr = self.addr.parse()?;
            let new_ctx = timeout(self.timeout_duration, tcp::connect(socket_addr))
                .await
                .context("Reconnection timeout")?
                .context("Failed to reconnect")?;

            let mut ctx = self.context.lock().await;
            *ctx = new_ctx;
            Ok(())
        }

        /// Execute an operation with retry logic
        async fn retry_operation<F, Fut, T>(&self, operation: F) -> Result<T>
        where
            F: Fn(&mut tokio_modbus::client::Context) -> Fut,
            Fut: std::future::Future<Output = std::result::Result<T, std::io::Error>>,
        {
            // Ensure we always attempt at least once, even if MAX_RETRIES is 0
            let max_attempts = MAX_RETRIES.max(1);

            for attempt in 1..=max_attempts {
                let mut ctx = self.context.lock().await;

                match timeout(self.timeout_duration, operation(&mut *ctx)).await {
                    Ok(Ok(result)) => {
                        if attempt > 1 {
                            debug!("Operation succeeded on attempt {}", attempt);
                        }
                        return Ok(result);
                    }
                    Ok(Err(e)) => {
                        warn!("Modbus operation failed (attempt {}): {}", attempt, e);
                        if attempt == max_attempts {
                            return Err(anyhow::anyhow!("Operation failed after {} attempts: {}", max_attempts, e));
                        }
                    }
                    Err(_) => {
                        warn!("Modbus operation timeout (attempt {})", attempt);
                        if attempt == max_attempts {
                            return Err(anyhow::anyhow!("Operation timeout after {} attempts", max_attempts));
                        }
                    }
                }

                // Small delay between retries
                drop(ctx); // Release lock before sleeping
                tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
            }

            // This should never be reached since we always return within the loop
            unreachable!("Retry loop should always return or error")
        }
    }

    impl Clone for ModbusClient {
        fn clone(&self) -> Self {
            Self {
                context: Arc::clone(&self.context),
                unit_id: self.unit_id,
                addr: self.addr.clone(),
                timeout_duration: self.timeout_duration,
            }
        }
    }
}
/// Register mapping for Modbus devices
#[cfg(feature = "modbus")]
pub mod register_map {
    use anyhow::{Result, Context};

    /// Register map trait for different device vendors
    pub trait RegisterMap: Send + Sync {
        fn soc_register(&self) -> u16;
        fn soc_scale(&self) -> f64;

        fn power_register(&self) -> u16;
        fn power_scale(&self) -> f64;

        fn voltage_register(&self) -> u16;
        fn voltage_scale(&self) -> f64;

        fn temperature_register(&self) -> u16;
        fn temperature_scale(&self) -> f64;

        fn health_register(&self) -> u16;
        fn health_scale(&self) -> f64;

        fn power_command_register(&self) -> u16;
        fn power_command_scale(&self) -> f64;

        fn max_charge_power_register(&self) -> u16;
        fn max_discharge_power_register(&self) -> u16;
        fn capacity_register(&self) -> u16;

        /// Control mode register (optional, returns None if not supported)
        /// Used to enable remote control mode on hybrid inverters
        fn control_mode_register(&self) -> Option<u16> {
            None
        }

        /// Value to write to control mode register to enable remote control
        fn remote_control_value(&self) -> u16 {
            1 // Default: 1 = Remote Control
        }
    }

    /// Generic battery register map (common Modbus addresses)
    pub struct GenericBatteryRegisterMap;

    impl RegisterMap for GenericBatteryRegisterMap {
        fn soc_register(&self) -> u16 { 37000 }
        fn soc_scale(&self) -> f64 { 0.1 }

        fn power_register(&self) -> u16 { 37001 }
        fn power_scale(&self) -> f64 { 1.0 }

        fn voltage_register(&self) -> u16 { 37002 }
        fn voltage_scale(&self) -> f64 { 0.1 }

        fn temperature_register(&self) -> u16 { 37003 }
        fn temperature_scale(&self) -> f64 { 0.1 }

        fn health_register(&self) -> u16 { 37004 }
        fn health_scale(&self) -> f64 { 0.1 }

        fn power_command_register(&self) -> u16 { 47000 }
        fn power_command_scale(&self) -> f64 { 1.0 }

        fn max_charge_power_register(&self) -> u16 { 37010 }
        fn max_discharge_power_register(&self) -> u16 { 37011 }
        fn capacity_register(&self) -> u16 { 37012 }
    }

    /// Huawei Luna2000 register map
    pub struct HuaweiLuna2000RegisterMap;

    impl RegisterMap for HuaweiLuna2000RegisterMap {
        fn soc_register(&self) -> u16 { 37760 }
        fn soc_scale(&self) -> f64 { 0.1 }

        fn power_register(&self) -> u16 { 37765 }
        fn power_scale(&self) -> f64 { 1.0 }

        fn voltage_register(&self) -> u16 { 37762 }
        fn voltage_scale(&self) -> f64 { 0.1 }

        fn temperature_register(&self) -> u16 { 37761 }
        fn temperature_scale(&self) -> f64 { 0.1 }

        fn health_register(&self) -> u16 { 37763 }
        fn health_scale(&self) -> f64 { 0.1 }

        fn power_command_register(&self) -> u16 { 47100 }
        fn power_command_scale(&self) -> f64 { 1.0 }

        fn max_charge_power_register(&self) -> u16 { 37764 }
        fn max_discharge_power_register(&self) -> u16 { 37764 }
        fn capacity_register(&self) -> u16 { 37758 }

        fn control_mode_register(&self) -> Option<u16> {
            Some(47000) // Huawei control mode register
        }

        fn remote_control_value(&self) -> u16 {
            2 // Huawei: 2 = Remote EMS Control
        }
    }

    /// SolarEdge StorEdge register map
    pub struct SolarEdgeRegisterMap;

    impl RegisterMap for SolarEdgeRegisterMap {
        fn soc_register(&self) -> u16 { 62852 }
        fn soc_scale(&self) -> f64 { 1.0 }

        fn power_register(&self) -> u16 { 62853 }
        fn power_scale(&self) -> f64 { 1.0 }

        fn voltage_register(&self) -> u16 { 62854 }
        fn voltage_scale(&self) -> f64 { 0.01 }

        fn temperature_register(&self) -> u16 { 62855 }
        fn temperature_scale(&self) -> f64 { 0.1 }

        fn health_register(&self) -> u16 { 62856 }
        fn health_scale(&self) -> f64 { 1.0 }

        fn power_command_register(&self) -> u16 { 57348 }
        fn power_command_scale(&self) -> f64 { 1.0 }

        fn max_charge_power_register(&self) -> u16 { 62857 }
        fn max_discharge_power_register(&self) -> u16 { 62858 }
        fn capacity_register(&self) -> u16 { 62859 }

        fn control_mode_register(&self) -> Option<u16> {
            Some(0xE004) // SolarEdge control mode register
        }

        fn remote_control_value(&self) -> u16 {
            3 // SolarEdge: 3 = Remote Control via Modbus
        }
    }
}

/// Modbus data parsing utilities
#[cfg(feature = "modbus")]
pub mod parser {
    use byteorder::{BigEndian, ByteOrder};

    /// Parse u16 from register
    pub fn parse_u16(registers: &[u16]) -> u16 {
        registers[0]
    }

    /// Parse i16 from register
    pub fn parse_i16(registers: &[u16]) -> i16 {
        registers[0] as i16
    }

    /// Parse u32 from two registers (big-endian)
    pub fn parse_u32(registers: &[u16]) -> u32 {
        if registers.len() < 2 {
            return 0;
        }
        ((registers[0] as u32) << 16) | (registers[1] as u32)
    }

    /// Parse i32 from two registers (big-endian)
    pub fn parse_i32(registers: &[u16]) -> i32 {
        parse_u32(registers) as i32
    }

    /// Parse f32 from two registers (IEEE 754)
    pub fn parse_f32(registers: &[u16]) -> f32 {
        if registers.len() < 2 {
            return 0.0;
        }
        let bytes = [
            (registers[0] >> 8) as u8,
            registers[0] as u8,
            (registers[1] >> 8) as u8,
            registers[1] as u8,
        ];
        BigEndian::read_f32(&bytes)
    }

    /// Parse scaled value (register value * scale factor)
    pub fn parse_scaled_value(registers: &[u16], scale: f64) -> f64 {
        let raw = parse_i16(registers) as f64;
        raw * scale
    }

    /// Parse scaled u16 value
    pub fn parse_scaled_u16(registers: &[u16], scale: f64) -> f64 {
        let raw = parse_u16(registers) as f64;
        raw * scale
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_u16() {
            assert_eq!(parse_u16(&[0x1234]), 0x1234);
        }

        #[test]
        fn test_parse_i16() {
            assert_eq!(parse_i16(&[0x8000]), -32768);
            assert_eq!(parse_i16(&[0x7FFF]), 32767);
        }

        #[test]
        fn test_parse_u32() {
            assert_eq!(parse_u32(&[0x1234, 0x5678]), 0x12345678);
        }

        #[test]
        fn test_parse_scaled_value() {
            assert_eq!(parse_scaled_value(&[100], 0.1), 10.0);
            assert_eq!(parse_scaled_value(&[500], 0.01), 5.0);
        }
    }
}

#[cfg(not(feature = "modbus"))]
pub mod client {}

#[cfg(not(feature = "modbus"))]
pub mod register_map {}

#[cfg(not(feature = "modbus"))]
pub mod parser {}
