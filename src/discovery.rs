use anyhow::Result;

#[derive(Debug, Clone)]
pub struct DeviceDiscovery;

impl DeviceDiscovery {
    pub async fn scan_modbus_devices(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }
    pub async fn listen_mdns(&self) -> Result<()> {
        Ok(())
    }
}
