#[cfg(feature = "modbus")]
pub mod client {
    use anyhow::Result;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio_modbus::client::tcp;
    use tokio_modbus::prelude::*;

    pub struct ModbusClient {
        context: Arc<Mutex<tokio_modbus::client::Context>>,
        unit_id: u8,
    }

    impl ModbusClient {
        pub async fn connect(addr: &str, unit_id: u8) -> Result<Self> {
            let socket_addr = addr.parse()?;
            let ctx = tcp::connect(socket_addr).await?;
            Ok(Self {
                context: Arc::new(Mutex::new(ctx)),
                unit_id,
            })
        }

        pub async fn read_holding_registers(&self, start: u16, count: u16) -> Result<Vec<u16>> {
            let mut ctx = self.context.lock().await;
            ctx.set_slave(Slave(self.unit_id));
            Ok(ctx.read_holding_registers(start, count).await?)
        }

        pub async fn write_multiple_registers(&self, start: u16, values: &[u16]) -> Result<()> {
            let mut ctx = self.context.lock().await;
            ctx.set_slave(Slave(self.unit_id));
            ctx.write_multiple_registers(start, values).await?;
            Ok(())
        }
    }
}
#[cfg(not(feature = "modbus"))]
pub mod client {}
