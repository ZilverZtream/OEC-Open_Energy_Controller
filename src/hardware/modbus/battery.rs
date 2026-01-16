use crate::domain::battery::{Battery, BatteryState, BatteryCapabilities};
use crate::modbus::client::ModbusClient;
use crate::modbus::register_map::{RegisterMap, GenericBatteryRegisterMap, HuaweiLuna2000RegisterMap, SolarEdgeRegisterMap};
use crate::modbus::parser;
use anyhow::{Result, Context};
use async_trait::async_trait;
use tracing::debug;

/// Modbus-based battery implementation
pub struct ModbusBattery {
    client: ModbusClient,
    register_map: Box<dyn RegisterMap>,
    capabilities: BatteryCapabilities,
}

impl ModbusBattery {
    /// Create a new ModbusBattery with generic register map
    pub async fn new(addr: &str, unit_id: u8) -> Result<Self> {
        let client = ModbusClient::connect(addr, unit_id)
            .await
            .context("Failed to connect to Modbus battery")?;

        let register_map = Box::new(GenericBatteryRegisterMap);

        // Read capabilities from device
        let capabilities = Self::read_capabilities(&client, &*register_map).await?;

        Ok(Self {
            client,
            register_map,
            capabilities,
        })
    }

    /// Create a new ModbusBattery with Huawei Luna2000 register map
    pub async fn new_huawei(addr: &str, unit_id: u8) -> Result<Self> {
        let client = ModbusClient::connect(addr, unit_id)
            .await
            .context("Failed to connect to Huawei battery")?;

        let register_map = Box::new(HuaweiLuna2000RegisterMap);
        let capabilities = Self::read_capabilities(&client, &*register_map).await?;

        Ok(Self {
            client,
            register_map,
            capabilities,
        })
    }

    /// Create a new ModbusBattery with SolarEdge register map
    pub async fn new_solaredge(addr: &str, unit_id: u8) -> Result<Self> {
        let client = ModbusClient::connect(addr, unit_id)
            .await
            .context("Failed to connect to SolarEdge battery")?;

        let register_map = Box::new(SolarEdgeRegisterMap);
        let capabilities = Self::read_capabilities(&client, &*register_map).await?;

        Ok(Self {
            client,
            register_map,
            capabilities,
        })
    }

    /// Read battery capabilities from device
    async fn read_capabilities(
        client: &ModbusClient,
        register_map: &dyn RegisterMap,
    ) -> Result<BatteryCapabilities> {
        // Read max charge power
        let max_charge_regs = client
            .read_holding_registers(register_map.max_charge_power_register(), 1)
            .await
            .unwrap_or_else(|_| vec![5000]); // Default 5kW if read fails

        // Read max discharge power
        let max_discharge_regs = client
            .read_holding_registers(register_map.max_discharge_power_register(), 1)
            .await
            .unwrap_or_else(|_| vec![5000]); // Default 5kW if read fails

        // Read capacity
        let capacity_regs = client
            .read_holding_registers(register_map.capacity_register(), 1)
            .await
            .unwrap_or_else(|_| vec![10000]); // Default 10kWh if read fails

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
        let register_value = (watts / scale) as i16;

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

        Ok(BatteryState {
            soc_percent: soc,
            power_w: power,
            voltage_v: voltage,
            temperature_c: temperature,
            health_percent: health,
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

