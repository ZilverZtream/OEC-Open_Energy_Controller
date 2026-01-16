#[cfg(feature = "modbus")]
pub mod battery;

#[cfg(feature = "modbus")]
pub use battery::ModbusBattery;
