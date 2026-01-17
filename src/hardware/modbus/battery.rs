use crate::domain::battery::{Battery, BatteryState, BatteryCapabilities};
use crate::modbus::client::ModbusClient;
use crate::modbus::register_map::{RegisterMap, GenericBatteryRegisterMap, HuaweiLuna2000RegisterMap, SolarEdgeRegisterMap};
use crate::modbus::parser;
use anyhow::{Result, Context};
use async_trait::async_trait;
use tracing::debug;

/// Modbus-based battery implementation
///
/// CRITICAL NETWORK SAFETY:
/// This struct creates a persistent TCP connection via ModbusClient.
/// The ModbusClient uses Arc<Mutex<Context>> internally to maintain a single
/// socket connection across all operations. The reconnect() method replaces
/// the context in-place without creating new sockets.
///
/// WARNING: Do NOT instantiate ModbusBattery repeatedly (e.g., inside a control loop).
/// Create ONCE at startup and reuse. Repeated instantiation will exhaust
/// ephemeral ports (~28k on Linux) within 8 hours at 1Hz due to TIME_WAIT state.
pub struct ModbusBattery {
    client: ModbusClient,
    register_map: Box<dyn RegisterMap>,
    capabilities: BatteryCapabilities,
}

impl ModbusBattery {
    /// Create a new ModbusBattery with generic register map
    ///
    /// IMPORTANT: Call this ONCE at startup, not in a loop.
    /// The created instance maintains a persistent TCP connection.
    pub async fn new(addr: &str, unit_id: u8) -> Result<Self> {
        let client = ModbusClient::connect(addr, unit_id)
            .await
            .context("Failed to connect to Modbus battery")?;

        let register_map = Box::new(GenericBatteryRegisterMap);

        // Read capabilities from device
        let capabilities = Self::read_capabilities(&client, &*register_map).await?;

        let battery = Self {
            client,
            register_map,
            capabilities,
        };

        // Enable remote control mode if supported by the device
        battery.enable_remote_control().await?;

        Ok(battery)
    }

    /// Create a new ModbusBattery with Huawei Luna2000 register map
    pub async fn new_huawei(addr: &str, unit_id: u8) -> Result<Self> {
        let client = ModbusClient::connect(addr, unit_id)
            .await
            .context("Failed to connect to Huawei battery")?;

        let register_map = Box::new(HuaweiLuna2000RegisterMap);
        let capabilities = Self::read_capabilities(&client, &*register_map).await?;

        let battery = Self {
            client,
            register_map,
            capabilities,
        };

        // Enable remote control mode (required for Huawei inverters)
        battery.enable_remote_control().await?;

        Ok(battery)
    }

    /// Create a new ModbusBattery with SolarEdge register map
    pub async fn new_solaredge(addr: &str, unit_id: u8) -> Result<Self> {
        let client = ModbusClient::connect(addr, unit_id)
            .await
            .context("Failed to connect to SolarEdge battery")?;

        let register_map = Box::new(SolarEdgeRegisterMap);
        let capabilities = Self::read_capabilities(&client, &*register_map).await?;

        let battery = Self {
            client,
            register_map,
            capabilities,
        };

        // Enable remote control mode (required for SolarEdge inverters)
        battery.enable_remote_control().await?;

        Ok(battery)
    }

    /// Read battery capabilities from device
    async fn read_capabilities(
        client: &ModbusClient,
        register_map: &dyn RegisterMap,
    ) -> Result<BatteryCapabilities> {
        // CRITICAL FIX: Fail early if capabilities cannot be read
        // Using wrong defaults (e.g., 10kWh for a 30kWh battery) causes incorrect
        // SoC calculations, which can lead to deep-discharge or overcharge damage.

        // Read max charge power with retry
        let max_charge_regs = client
            .read_holding_registers(register_map.max_charge_power_register(), 1)
            .await
            .context("Failed to read max_charge_power - battery not responding correctly")?;

        // Read max discharge power with retry
        let max_discharge_regs = client
            .read_holding_registers(register_map.max_discharge_power_register(), 1)
            .await
            .context("Failed to read max_discharge_power - battery not responding correctly")?;

        // Read capacity with retry (MOST CRITICAL - wrong capacity = wrong SoC)
        let capacity_regs = client
            .read_holding_registers(register_map.capacity_register(), 1)
            .await
            .context("Failed to read battery capacity - cannot proceed without accurate specs")?;

        let max_charge_kw = parser::parse_u16(&max_charge_regs) as f64 / 1000.0;
        let max_discharge_kw = parser::parse_u16(&max_discharge_regs) as f64 / 1000.0;
        let capacity_kwh = parser::parse_u16(&capacity_regs) as f64 / 1000.0;

        Ok(BatteryCapabilities {
            capacity_kwh,
            max_charge_kw,
            max_discharge_kw,
            efficiency: 0.95, // 95% typical round-trip efficiency
            degradation_per_cycle: 0.0001, // 0.01% per cycle
        })
    }

    /// Read a single register with scaling
    async fn read_scaled_register(&self, register: u16, scale: f64) -> Result<f64> {
        let regs = self.client.read_holding_registers(register, 1).await?;
        Ok(parser::parse_scaled_u16(&regs, scale))
    }

    /// Write power command to battery (watts, positive=charge, negative=discharge)
    async fn write_power_command(&self, watts: f64) -> Result<()> {
        let scale = self.register_map.power_command_scale();
        let scaled_value = watts / scale;

        // CRITICAL FIX: Prevent integer overflow that would reverse control polarity
        // i16 range is -32,768 to 32,767. If scaled_value exceeds this, it wraps around.
        // Example: 5000W / 0.1 = 50,000 -> wraps to -15,536 (DISCHARGE instead of CHARGE!)
        const I16_MAX: f64 = 32767.0;
        const I16_MIN: f64 = -32768.0;

        if scaled_value > I16_MAX || scaled_value < I16_MIN {
            return Err(anyhow::anyhow!(
                "Power command {} W (scaled: {}) exceeds i16 range [{}, {}]. \
                 This would cause integer overflow and reverse polarity. \
                 Check your register map scale factor ({}) or reduce power.",
                watts,
                scaled_value,
                I16_MIN,
                I16_MAX,
                scale
            ));
        }

        let register_value = scaled_value as i16;

        // Convert i16 to u16 for Modbus
        let value = register_value as u16;

        debug!("Writing power command: {} W (register value: {})", watts, register_value);

        self.client
            .write_single_register(
                self.register_map.power_command_register(),
                value,
            )
            .await
    }

    /// Health check the Modbus connection
    pub async fn health_check(&self) -> Result<()> {
        self.client.health_check().await
    }

    /// Reconnect to the Modbus device if connection is lost
    ///
    /// SAFE: This method reuses the existing ModbusClient and replaces the
    /// TCP context in-place without creating new sockets. Safe to call
    /// from control loops for connection recovery.
    pub async fn reconnect(&self) -> Result<()> {
        self.client.reconnect().await
    }

    /// Enable remote control mode on the inverter
    ///
    /// CRITICAL: Hybrid inverters default to "Maximize Self-Consumption" mode.
    /// This method writes to the control mode register to enable external power
    /// commands via Modbus. Without this, power commands will be ignored.
    async fn enable_remote_control(&self) -> Result<()> {
        if let Some(register) = self.register_map.control_mode_register() {
            let value = self.register_map.remote_control_value();
            debug!("Enabling remote control mode: writing {} to register {}", value, register);

            self.client
                .write_single_register(register, value)
                .await
                .context("Failed to enable remote control mode")?;

            tracing::info!(
                "Remote control mode enabled (register: {}, value: {})",
                register,
                value
            );
        } else {
            debug!("Device does not support control mode register, skipping");
        }
        Ok(())
    }
}

#[async_trait]
impl Battery for ModbusBattery {
    async fn read_state(&self) -> Result<BatteryState> {
        debug!("Reading battery state via Modbus");

        // Read all registers in parallel for efficiency
        let soc_task = self.read_scaled_register(
            self.register_map.soc_register(),
            self.register_map.soc_scale(),
        );

        let power_task = self.read_scaled_register(
            self.register_map.power_register(),
            self.register_map.power_scale(),
        );

        let voltage_task = self.read_scaled_register(
            self.register_map.voltage_register(),
            self.register_map.voltage_scale(),
        );

        let temp_task = self.read_scaled_register(
            self.register_map.temperature_register(),
            self.register_map.temperature_scale(),
        );

        let health_task = self.read_scaled_register(
            self.register_map.health_register(),
            self.register_map.health_scale(),
        );

        // Wait for all reads to complete
        let (soc, power, voltage, temperature, health) = tokio::try_join!(
            soc_task,
            power_task,
            voltage_task,
            temp_task,
            health_task,
        )?;

        // Determine status based on power
        let status = if power > 10.0 {
            crate::domain::battery::BatteryStatus::Charging
        } else if power < -10.0 {
            crate::domain::battery::BatteryStatus::Discharging
        } else {
            crate::domain::battery::BatteryStatus::Idle
        };

        Ok(BatteryState {
            soc_percent: soc,
            power_w: power,
            voltage_v: voltage,
            temperature_c: temperature,
            health_percent: health,
            status,
        })
    }

    async fn set_power(&self, watts: f64) -> Result<()> {
        // Validate power against capabilities
        let max_charge_w = self.capabilities.max_charge_kw * 1000.0;
        let max_discharge_w = self.capabilities.max_discharge_kw * 1000.0;

        if watts > max_charge_w {
            return Err(anyhow::anyhow!(
                "Charge power {} W exceeds maximum {} W",
                watts,
                max_charge_w
            ));
        }

        if watts < -max_discharge_w {
            return Err(anyhow::anyhow!(
                "Discharge power {} W exceeds maximum {} W",
                -watts,
                max_discharge_w
            ));
        }

        self.write_power_command(watts).await
    }

    fn capabilities(&self) -> BatteryCapabilities {
        self.capabilities.clone()
    }

    /// AUDIT FIX #5: Override default reconnect implementation
    /// Reconnect to the Modbus device if connection is lost
    async fn reconnect(&self) -> Result<()> {
        self.client.reconnect().await
    }

    async fn health_check(&self) -> Result<crate::domain::HealthStatus> {
        match self.client.health_check().await {
            Ok(_) => Ok(crate::domain::HealthStatus::Healthy),
            Err(_) => Ok(crate::domain::HealthStatus::Offline),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_register_map() {
        let map = GenericBatteryRegisterMap;
        assert_eq!(map.soc_register(), 37000);
        assert_eq!(map.soc_scale(), 0.1);
    }

    #[test]
    fn test_huawei_register_map() {
        let map = HuaweiLuna2000RegisterMap;
        assert_eq!(map.soc_register(), 37760);
        assert_eq!(map.soc_scale(), 0.1);
    }

    #[test]
    fn test_solaredge_register_map() {
        let map = SolarEdgeRegisterMap;
        assert_eq!(map.soc_register(), 62852);
        assert_eq!(map.soc_scale(), 1.0);
    }
}

