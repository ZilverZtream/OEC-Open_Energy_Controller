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

    /// Listen for mDNS announcements
    pub async fn listen_mdns(&self) -> Result<()> {
        #[cfg(feature = "discovery")]
        {
            let mdns = MdnsListener::new()?;
            mdns.start_listening().await?;
        }
        #[cfg(not(feature = "discovery"))]
        {
            info!("mDNS discovery not enabled (requires 'discovery' feature)");
        }
        Ok(())
    }
}

#[cfg(feature = "discovery")]
pub mod mdns {
    use super::*;
    use mdns_sd::{ServiceDaemon, ServiceEvent};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// mDNS service types to discover
    const MODBUS_SERVICE: &str = "_modbus._tcp.local.";
    const HTTP_SERVICE: &str = "_http._tcp.local.";
    const OCPP_SERVICE: &str = "_ocpp._tcp.local.";

    /// Discovered service information from mDNS
    #[derive(Debug, Clone)]
    pub struct MdnsService {
        pub service_name: String,
        pub service_type: String,
        pub hostname: String,
        pub ip_addresses: Vec<IpAddr>,
        pub port: u16,
        pub txt_properties: HashMap<String, String>,
    }

    impl MdnsService {
        /// Try to convert mDNS service to DiscoveredDevice
        pub fn to_discovered_device(&self) -> Option<DiscoveredDevice> {
            let ip = self.ip_addresses.first()?;

            // Determine device type from service type and TXT records
            let device_type = if self.service_type.contains("modbus") {
                // Check TXT records for device type hint
                if let Some(dtype) = self.txt_properties.get("type") {
                    match dtype.as_str() {
                        "battery" => DeviceType::ModbusBattery,
                        "inverter" => DeviceType::ModbusInverter,
                        "ev_charger" | "evcharger" => DeviceType::ModbusEvCharger,
                        _ => DeviceType::Unknown,
                    }
                } else {
                    DeviceType::Unknown
                }
            } else {
                DeviceType::Unknown
            };

            Some(DiscoveredDevice {
                ip: *ip,
                port: self.port,
                device_type,
                manufacturer: self.txt_properties.get("manufacturer").cloned(),
                model: self.txt_properties.get("model").cloned(),
                serial: self.txt_properties.get("serial").cloned(),
            })
        }
    }

    /// mDNS service discovery listener
    pub struct MdnsListener {
        daemon: ServiceDaemon,
        discovered_services: Arc<RwLock<HashMap<String, MdnsService>>>,
    }

    impl MdnsListener {
        /// Create a new mDNS listener
        pub fn new() -> Result<Self> {
            let daemon = ServiceDaemon::new()
                .map_err(|e| anyhow::anyhow!("Failed to create mDNS daemon: {}", e))?;

            Ok(Self {
                daemon,
                discovered_services: Arc::new(RwLock::new(HashMap::new())),
            })
        }

        /// Start listening for mDNS services
        pub async fn start_listening(&self) -> Result<()> {
            info!("Starting mDNS service discovery");

            // Browse for each service type
            let service_types = vec![MODBUS_SERVICE, HTTP_SERVICE, OCPP_SERVICE];

            for service_type in service_types {
                let receiver = self
                    .daemon
                    .browse(service_type)
                    .map_err(|e| anyhow::anyhow!("Failed to browse {}: {}", service_type, e))?;

                let services = Arc::clone(&self.discovered_services);
                let service_type_str = service_type.to_string();

                // Spawn background task to handle service events
                tokio::spawn(async move {
                    Self::handle_service_events(receiver, services, service_type_str).await;
                });
            }

            info!("mDNS listener started for {} service types", service_types.len());
            Ok(())
        }

        /// Handle mDNS service events
        async fn handle_service_events(
            receiver: mdns_sd::Receiver<ServiceEvent>,
            services: Arc<RwLock<HashMap<String, MdnsService>>>,
            service_type: String,
        ) {
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        info!("mDNS service resolved: {} ({})", info.get_fullname(), service_type);

                        let ip_addresses: Vec<IpAddr> = info
                            .get_addresses()
                            .iter()
                            .map(|addr| IpAddr::from(*addr))
                            .collect();

                        let txt_properties: HashMap<String, String> = info
                            .get_properties()
                            .iter()
                            .filter_map(|prop| {
                                let key = prop.key();
                                let val = prop.val_str();
                                Some((key.to_string(), val.to_string()))
                            })
                            .collect();

                        let service = MdnsService {
                            service_name: info.get_fullname().to_string(),
                            service_type: service_type.clone(),
                            hostname: info.get_hostname().to_string(),
                            ip_addresses,
                            port: info.get_port(),
                            txt_properties,
                        };

                        // Store the service
                        let mut services_map = services.write().await;
                        services_map.insert(info.get_fullname().to_string(), service.clone());

                        // Log discovered device info
                        if let Some(device) = service.to_discovered_device() {
                            info!(
                                "Discovered {:?} via mDNS at {}:{}",
                                device.device_type, device.ip, device.port
                            );
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        info!("mDNS service removed: {}", fullname);
                        let mut services_map = services.write().await;
                        services_map.remove(&fullname);
                    }
                    ServiceEvent::SearchStarted(_) => {
                        debug!("mDNS search started for {}", service_type);
                    }
                    ServiceEvent::SearchStopped(_) => {
                        warn!("mDNS search stopped for {}", service_type);
                    }
                }
            }
        }

        /// Get all currently discovered services
        pub async fn get_discovered_services(&self) -> Vec<MdnsService> {
            let services = self.discovered_services.read().await;
            services.values().cloned().collect()
        }

        /// Get all discovered devices (converted from services)
        pub async fn get_discovered_devices(&self) -> Vec<DiscoveredDevice> {
            let services = self.get_discovered_services().await;
            services
                .iter()
                .filter_map(|s| s.to_discovered_device())
                .collect()
        }

        /// Stop the mDNS daemon
        pub fn shutdown(&self) -> Result<()> {
            self.daemon
                .shutdown()
                .map_err(|e| anyhow::anyhow!("Failed to shutdown mDNS daemon: {}", e))?;
            info!("mDNS listener shut down");
            Ok(())
        }
    }

    impl Drop for MdnsListener {
        fn drop(&mut self) {
            let _ = self.shutdown();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mdns_service_to_device_battery() {
            let mut txt_props = HashMap::new();
            txt_props.insert("type".to_string(), "battery".to_string());
            txt_props.insert("manufacturer".to_string(), "Huawei".to_string());
            txt_props.insert("model".to_string(), "Luna2000".to_string());

            let service = MdnsService {
                service_name: "battery-1._modbus._tcp.local.".to_string(),
                service_type: "_modbus._tcp.local.".to_string(),
                hostname: "battery-1.local.".to_string(),
                ip_addresses: vec!["192.168.1.100".parse().unwrap()],
                port: 502,
                txt_properties: txt_props,
            };

            let device = service.to_discovered_device().unwrap();
            assert_eq!(device.device_type, DeviceType::ModbusBattery);
            assert_eq!(device.manufacturer, Some("Huawei".to_string()));
            assert_eq!(device.model, Some("Luna2000".to_string()));
            assert_eq!(device.port, 502);
        }

        #[test]
        fn test_mdns_service_to_device_no_type() {
            let service = MdnsService {
                service_name: "device-1._modbus._tcp.local.".to_string(),
                service_type: "_modbus._tcp.local.".to_string(),
                hostname: "device-1.local.".to_string(),
                ip_addresses: vec!["192.168.1.101".parse().unwrap()],
                port: 502,
                txt_properties: HashMap::new(),
            };

            let device = service.to_discovered_device().unwrap();
            assert_eq!(device.device_type, DeviceType::Unknown);
        }
    }
}

#[cfg(feature = "discovery")]
pub use mdns::MdnsListener;

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

