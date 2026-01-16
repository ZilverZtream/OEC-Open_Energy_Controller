#![allow(dead_code)]
use anyhow::Result;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Device information discovered on the network
#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub ip: IpAddr,
    pub port: u16,
    pub device_type: DeviceType,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceType {
    ModbusBattery,
    ModbusInverter,
    ModbusEvCharger,
    Unknown,
}

/// Network scanner for discovering devices
pub struct NetworkScanner {
    scan_timeout: Duration,
    concurrent_scans: usize,
}

impl Default for NetworkScanner {
    fn default() -> Self {
        Self {
            scan_timeout: Duration::from_millis(200),
            concurrent_scans: 100,
        }
    }
}

impl NetworkScanner {
    pub fn new(scan_timeout: Duration, concurrent_scans: usize) -> Self {
        Self {
            scan_timeout,
            concurrent_scans,
        }
    }

    /// Scan a single IP address for Modbus devices on common ports
    async fn scan_ip(&self, ip: IpAddr) -> Vec<(IpAddr, u16)> {
        let common_ports = vec![502, 1502, 8502]; // Common Modbus TCP ports
        let mut found = Vec::new();

        for port in common_ports {
            if self.is_port_open(ip, port).await {
                found.push((ip, port));
            }
        }

        found
    }

    /// Check if a port is open on a given IP
    async fn is_port_open(&self, ip: IpAddr, port: u16) -> bool {
        let addr = SocketAddr::new(ip, port);

        match timeout(self.scan_timeout, TcpStream::connect(addr)).await {
            Ok(Ok(_)) => {
                debug!("Port {}:{} is open", ip, port);
                true
            }
            Ok(Err(_)) | Err(_) => false,
        }
    }

    /// Scan an IP range for Modbus devices
    /// Example range: "192.168.1.0/24"
    pub async fn scan_range(&self, ip_range: &str) -> Result<Vec<(IpAddr, u16)>> {
        let ips = parse_ip_range(ip_range)?;

        info!("Scanning {} IP addresses for Modbus devices", ips.len());

        let mut handles = Vec::new();
        let mut all_found = Vec::new();

        // Scan in batches to avoid overwhelming the network
        for chunk in ips.chunks(self.concurrent_scans) {
            for ip in chunk {
                let scanner = Self {
                    scan_timeout: self.scan_timeout,
                    concurrent_scans: self.concurrent_scans,
                };
                let ip = *ip;
                let handle = tokio::spawn(async move { scanner.scan_ip(ip).await });
                handles.push(handle);
            }

            // Wait for this batch to complete
            for handle in handles.drain(..) {
                if let Ok(found) = handle.await {
                    all_found.extend(found);
                }
            }
        }

        info!("Found {} potential Modbus devices", all_found.len());
        Ok(all_found)
    }
}

/// Parse an IP range string into a list of IP addresses
/// Supports: "192.168.1.0/24" or "192.168.1.1-192.168.1.254"
fn parse_ip_range(range: &str) -> Result<Vec<IpAddr>> {
    if range.contains('/') {
        // CIDR notation
        parse_cidr(range)
    } else if range.contains('-') {
        // Range notation
        parse_range(range)
    } else {
        // Single IP
        Ok(vec![range.parse()?])
    }
}

/// Parse CIDR notation (e.g., "192.168.1.0/24")
fn parse_cidr(cidr: &str) -> Result<Vec<IpAddr>> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid CIDR notation");
    }

    let base_ip: IpAddr = parts[0].parse()?;
    let prefix_len: u32 = parts[1].parse()?;

    match base_ip {
        IpAddr::V4(ipv4) => {
            let base = u32::from(ipv4);
            let mask = !((1u32 << (32 - prefix_len)) - 1);
            let network = base & mask;
            let broadcast = network | !mask;

            let mut ips = Vec::new();
            // Skip network and broadcast addresses
            for i in (network + 1)..broadcast {
                ips.push(IpAddr::V4(i.into()));
            }
            Ok(ips)
        }
        IpAddr::V6(_) => {
            // IPv6 ranges can be huge, so we'll skip for now
            anyhow::bail!("IPv6 CIDR ranges not supported")
        }
    }
}

/// Parse range notation (e.g., "192.168.1.1-192.168.1.10")
fn parse_range(range: &str) -> Result<Vec<IpAddr>> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid range notation");
    }

    let start_ip: IpAddr = parts[0].trim().parse()?;
    let end_ip: IpAddr = parts[1].trim().parse()?;

    match (start_ip, end_ip) {
        (IpAddr::V4(start), IpAddr::V4(end)) => {
            let start_u32 = u32::from(start);
            let end_u32 = u32::from(end);

            if start_u32 > end_u32 {
                anyhow::bail!("Start IP must be less than or equal to end IP");
            }

            let mut ips = Vec::new();
            for i in start_u32..=end_u32 {
                ips.push(IpAddr::V4(i.into()));
            }
            Ok(ips)
        }
        _ => anyhow::bail!("Only IPv4 ranges are supported"),
    }
}

/// Modbus device identifier
pub struct ModbusIdentifier;

impl ModbusIdentifier {
    /// Identify a device at the given IP and port
    #[cfg(feature = "modbus")]
    pub async fn identify(ip: IpAddr, port: u16) -> Result<DiscoveredDevice> {
        use crate::modbus::client::ModbusClient;

        let addr = format!("{}:{}", ip, port);

        // Try different unit IDs
        for unit_id in [1, 2, 3, 126, 247] {
            if let Ok(client) = ModbusClient::connect(&addr, unit_id).await {
                // Try to identify the device by reading identification registers
                if let Ok(device_type) = Self::detect_device_type(&client).await {
                    return Ok(DiscoveredDevice {
                        ip,
                        port,
                        device_type,
                        manufacturer: None, // Could be read from device-specific registers
                        model: None,
                        serial: None,
                    });
                }
            }
        }

        // Couldn't identify specific type, but it's responding to Modbus
        Ok(DiscoveredDevice {
            ip,
            port,
            device_type: DeviceType::Unknown,
            manufacturer: None,
            model: None,
            serial: None,
        })
    }

    #[cfg(feature = "modbus")]
    async fn detect_device_type(client: &crate::modbus::client::ModbusClient) -> Result<DeviceType> {
        // Try battery-specific registers
        if client.read_holding_registers(37000, 1).await.is_ok() {
            return Ok(DeviceType::ModbusBattery);
        }

        // Try inverter-specific registers
        if client.read_holding_registers(40000, 1).await.is_ok() {
            return Ok(DeviceType::ModbusInverter);
        }

        // Try EV charger-specific registers
        if client.read_holding_registers(30000, 1).await.is_ok() {
            return Ok(DeviceType::ModbusEvCharger);
        }

        Ok(DeviceType::Unknown)
    }

    #[cfg(not(feature = "modbus"))]
    pub async fn identify(ip: IpAddr, port: u16) -> Result<DiscoveredDevice> {
        Ok(DiscoveredDevice {
            ip,
            port,
            device_type: DeviceType::Unknown,
            manufacturer: None,
            model: None,
            serial: None,
        })
    }
}

/// Main device discovery orchestrator
#[derive(Debug, Clone)]
pub struct DeviceDiscovery {
    scan_interval_secs: u64,
    ip_ranges: Vec<String>,
}

impl Default for DeviceDiscovery {
    fn default() -> Self {
        Self {
            scan_interval_secs: 300, // 5 minutes
            ip_ranges: vec!["192.168.1.0/24".to_string()],
        }
    }
}

impl DeviceDiscovery {
    pub fn new(scan_interval_secs: u64, ip_ranges: Vec<String>) -> Self {
        Self {
            scan_interval_secs,
            ip_ranges,
        }
    }

    /// Scan for Modbus devices on configured IP ranges
    pub async fn scan_modbus_devices(&self) -> Result<Vec<DiscoveredDevice>> {
        let scanner = NetworkScanner::default();
        let mut all_devices = Vec::new();

        for range in &self.ip_ranges {
            info!("Scanning IP range: {}", range);
            match scanner.scan_range(range).await {
                Ok(found) => {
                    for (ip, port) in found {
                        match ModbusIdentifier::identify(ip, port).await {
                            Ok(device) => {
                                info!("Discovered {:?} at {}:{}", device.device_type, ip, port);
                                all_devices.push(device);
                            }
                            Err(e) => {
                                warn!("Failed to identify device at {}:{}: {}", ip, port, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to scan range {}: {}", range, e);
                }
            }
        }

        Ok(all_devices)
    }

    /// Start continuous discovery (runs in background)
    pub async fn start_continuous_discovery(&self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(self.scan_interval_secs));

        loop {
            interval.tick().await;

            info!("Starting periodic device discovery scan");
            match self.scan_modbus_devices().await {
                Ok(devices) => {
                    info!("Discovery scan complete: found {} devices", devices.len());
                }
                Err(e) => {
                    warn!("Discovery scan failed: {}", e);
                }
            }
        }
    }

    /// Listen for mDNS announcements (placeholder for future implementation)
    pub async fn listen_mdns(&self) -> Result<()> {
        #[cfg(feature = "discovery")]
        {
            info!("mDNS discovery not yet implemented");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cidr() {
        let ips = parse_cidr("192.168.1.0/30").unwrap();
        assert_eq!(ips.len(), 2); // Only .1 and .2 (skip network and broadcast)
    }

    #[test]
    fn test_parse_range() {
        let ips = parse_range("192.168.1.1-192.168.1.5").unwrap();
        assert_eq!(ips.len(), 5);
    }

    #[test]
    fn test_parse_single_ip() {
        let ips = parse_ip_range("192.168.1.100").unwrap();
        assert_eq!(ips.len(), 1);
    }
}

