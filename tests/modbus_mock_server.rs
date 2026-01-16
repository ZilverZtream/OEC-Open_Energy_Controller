#![allow(dead_code)]
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

/// Modbus function codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FunctionCode {
    ReadHoldingRegisters = 0x03,
    ReadInputRegisters = 0x04,
    WriteSingleRegister = 0x06,
    WriteMultipleRegisters = 0x10,
}

/// Modbus exception codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExceptionCode {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    ServerDeviceFailure = 0x04,
}

/// Mock Modbus server for testing
pub struct MockModbusServer {
    /// Server address
    addr: SocketAddr,
    /// Holding registers storage
    holding_registers: Arc<RwLock<HashMap<u16, u16>>>,
    /// Input registers storage
    input_registers: Arc<RwLock<HashMap<u16, u16>>>,
    /// Simulate response delay (milliseconds)
    response_delay_ms: u64,
    /// Simulate connection failures
    simulate_connection_error: Arc<RwLock<bool>>,
    /// Simulate timeout
    simulate_timeout: Arc<RwLock<bool>>,
}

impl MockModbusServer {
    /// Create a new mock Modbus server
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            holding_registers: Arc::new(RwLock::new(HashMap::new())),
            input_registers: Arc::new(RwLock::new(HashMap::new())),
            response_delay_ms: 0,
            simulate_connection_error: Arc::new(RwLock::new(false)),
            simulate_timeout: Arc::new(RwLock::new(false)),
        }
    }

    /// Set response delay for realistic simulation
    pub fn set_response_delay(&mut self, delay_ms: u64) {
        self.response_delay_ms = delay_ms;
    }

    /// Enable connection error simulation
    pub fn set_connection_error(&self, enable: bool) {
        let simulate = self.simulate_connection_error.clone();
        tokio::spawn(async move {
            *simulate.write().await = enable;
        });
    }

    /// Enable timeout simulation
    pub fn set_timeout(&self, enable: bool) {
        let simulate = self.simulate_timeout.clone();
        tokio::spawn(async move {
            *simulate.write().await = enable;
        });
    }

    /// Set a holding register value
    pub async fn set_holding_register(&self, address: u16, value: u16) {
        let mut registers = self.holding_registers.write().await;
        registers.insert(address, value);
    }

    /// Set multiple holding registers
    pub async fn set_holding_registers(&self, start_address: u16, values: &[u16]) {
        let mut registers = self.holding_registers.write().await;
        for (i, &value) in values.iter().enumerate() {
            registers.insert(start_address + i as u16, value);
        }
    }

    /// Set an input register value
    pub async fn set_input_register(&self, address: u16, value: u16) {
        let mut registers = self.input_registers.write().await;
        registers.insert(address, value);
    }

    /// Get a holding register value
    pub async fn get_holding_register(&self, address: u16) -> Option<u16> {
        let registers = self.holding_registers.read().await;
        registers.get(&address).copied()
    }

    /// Start the mock server
    pub async fn start(self: Arc<Self>) -> std::io::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        println!("Mock Modbus server listening on {}", self.addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(stream).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }
    }

    /// Handle a client connection
    async fn handle_connection(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let mut buffer = vec![0u8; 256];

        loop {
            // Check if we should simulate connection error
            if *self.simulate_connection_error.read().await {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "Simulated connection error",
                ));
            }

            // Check if we should simulate timeout
            if *self.simulate_timeout.read().await {
                sleep(Duration::from_secs(60)).await;
                return Ok(());
            }

            // Read request
            let n = match stream.read(&mut buffer).await {
                Ok(0) => return Ok(()), // Connection closed
                Ok(n) => n,
                Err(e) => return Err(e),
            };

            let request = &buffer[..n];

            // Simulate response delay
            if self.response_delay_ms > 0 {
                sleep(Duration::from_millis(self.response_delay_ms)).await;
            }

            // Process request and generate response
            let response = self.process_request(request).await;

            // Send response
            stream.write_all(&response).await?;
        }
    }

    /// Process a Modbus request and generate a response
    async fn process_request(&self, request: &[u8]) -> Vec<u8> {
        if request.len() < 8 {
            return self.error_response(0, 0, ExceptionCode::IllegalDataValue);
        }

        // Parse Modbus TCP header
        let _transaction_id = u16::from_be_bytes([request[0], request[1]]);
        let _protocol_id = u16::from_be_bytes([request[2], request[3]]);
        let _length = u16::from_be_bytes([request[4], request[5]]);
        let unit_id = request[6];
        let function_code = request[7];

        match function_code {
            0x03 => self.read_holding_registers(unit_id, &request[8..]).await,
            0x04 => self.read_input_registers(unit_id, &request[8..]).await,
            0x06 => self.write_single_register(unit_id, &request[8..]).await,
            0x10 => self.write_multiple_registers(unit_id, &request[8..]).await,
            _ => self.error_response(unit_id, function_code, ExceptionCode::IllegalFunction),
        }
    }

    /// Read holding registers (function code 0x03)
    async fn read_holding_registers(&self, unit_id: u8, data: &[u8]) -> Vec<u8> {
        if data.len() < 4 {
            return self.error_response(unit_id, 0x03, ExceptionCode::IllegalDataValue);
        }

        let start_address = u16::from_be_bytes([data[0], data[1]]);
        let quantity = u16::from_be_bytes([data[2], data[3]]);

        if quantity == 0 || quantity > 125 {
            return self.error_response(unit_id, 0x03, ExceptionCode::IllegalDataValue);
        }

        let registers = self.holding_registers.read().await;
        let mut values = Vec::new();

        for i in 0..quantity {
            let address = start_address + i;
            let value = registers.get(&address).copied().unwrap_or(0);
            values.push(value);
        }

        self.read_response(unit_id, 0x03, &values)
    }

    /// Read input registers (function code 0x04)
    async fn read_input_registers(&self, unit_id: u8, data: &[u8]) -> Vec<u8> {
        if data.len() < 4 {
            return self.error_response(unit_id, 0x04, ExceptionCode::IllegalDataValue);
        }

        let start_address = u16::from_be_bytes([data[0], data[1]]);
        let quantity = u16::from_be_bytes([data[2], data[3]]);

        if quantity == 0 || quantity > 125 {
            return self.error_response(unit_id, 0x04, ExceptionCode::IllegalDataValue);
        }

        let registers = self.input_registers.read().await;
        let mut values = Vec::new();

        for i in 0..quantity {
            let address = start_address + i;
            let value = registers.get(&address).copied().unwrap_or(0);
            values.push(value);
        }

        self.read_response(unit_id, 0x04, &values)
    }

    /// Write single register (function code 0x06)
    async fn write_single_register(&self, unit_id: u8, data: &[u8]) -> Vec<u8> {
        if data.len() < 4 {
            return self.error_response(unit_id, 0x06, ExceptionCode::IllegalDataValue);
        }

        let address = u16::from_be_bytes([data[0], data[1]]);
        let value = u16::from_be_bytes([data[2], data[3]]);

        let mut registers = self.holding_registers.write().await;
        registers.insert(address, value);

        self.write_single_response(unit_id, address, value)
    }

    /// Write multiple registers (function code 0x10)
    async fn write_multiple_registers(&self, unit_id: u8, data: &[u8]) -> Vec<u8> {
        if data.len() < 5 {
            return self.error_response(unit_id, 0x10, ExceptionCode::IllegalDataValue);
        }

        let start_address = u16::from_be_bytes([data[0], data[1]]);
        let quantity = u16::from_be_bytes([data[2], data[3]]);
        let byte_count = data[4];

        if byte_count as usize != quantity as usize * 2 {
            return self.error_response(unit_id, 0x10, ExceptionCode::IllegalDataValue);
        }

        let mut registers = self.holding_registers.write().await;
        for i in 0..quantity {
            let offset = 5 + (i * 2) as usize;
            if offset + 1 >= data.len() {
                return self.error_response(unit_id, 0x10, ExceptionCode::IllegalDataValue);
            }
            let value = u16::from_be_bytes([data[offset], data[offset + 1]]);
            registers.insert(start_address + i, value);
        }

        self.write_multiple_response(unit_id, start_address, quantity)
    }

    /// Generate read response
    fn read_response(&self, unit_id: u8, function_code: u8, values: &[u16]) -> Vec<u8> {
        let byte_count = (values.len() * 2) as u8;
        let mut response = vec![
            0, 0, // Transaction ID (will be filled by client)
            0, 0, // Protocol ID
            0, 0, // Length (will be calculated)
            unit_id,
            function_code,
            byte_count,
        ];

        for &value in values {
            response.extend_from_slice(&value.to_be_bytes());
        }

        // Set length field
        let length = (response.len() - 6) as u16;
        response[4..6].copy_from_slice(&length.to_be_bytes());

        response
    }

    /// Generate write single register response
    fn write_single_response(&self, unit_id: u8, address: u16, value: u16) -> Vec<u8> {
        let mut response = vec![
            0, 0, // Transaction ID
            0, 0, // Protocol ID
            0, 6, // Length
            unit_id,
            0x06, // Function code
        ];
        response.extend_from_slice(&address.to_be_bytes());
        response.extend_from_slice(&value.to_be_bytes());
        response
    }

    /// Generate write multiple registers response
    fn write_multiple_response(&self, unit_id: u8, start_address: u16, quantity: u16) -> Vec<u8> {
        let mut response = vec![
            0, 0, // Transaction ID
            0, 0, // Protocol ID
            0, 6, // Length
            unit_id,
            0x10, // Function code
        ];
        response.extend_from_slice(&start_address.to_be_bytes());
        response.extend_from_slice(&quantity.to_be_bytes());
        response
    }

    /// Generate error response
    fn error_response(&self, unit_id: u8, function_code: u8, exception: ExceptionCode) -> Vec<u8> {
        vec![
            0,
            0, // Transaction ID
            0,
            0,                    // Protocol ID
            0,
            3,                    // Length
            unit_id,              // Unit ID
            function_code | 0x80, // Function code with error bit
            exception as u8,      // Exception code
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_mock_server_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 15502);
        let server = MockModbusServer::new(addr);

        assert_eq!(server.addr, addr);
        assert_eq!(server.response_delay_ms, 0);
    }

    #[tokio::test]
    async fn test_set_get_holding_register() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 15503);
        let server = MockModbusServer::new(addr);

        server.set_holding_register(100, 1234).await;

        let value = server.get_holding_register(100).await;
        assert_eq!(value, Some(1234));
    }

    #[tokio::test]
    async fn test_set_multiple_registers() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 15504);
        let server = MockModbusServer::new(addr);

        let values = vec![100, 200, 300];
        server.set_holding_registers(1000, &values).await;

        assert_eq!(server.get_holding_register(1000).await, Some(100));
        assert_eq!(server.get_holding_register(1001).await, Some(200));
        assert_eq!(server.get_holding_register(1002).await, Some(300));
    }

    #[tokio::test]
    async fn test_response_delay() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 15505);
        let mut server = MockModbusServer::new(addr);

        server.set_response_delay(50);
        assert_eq!(server.response_delay_ms, 50);
    }
}
