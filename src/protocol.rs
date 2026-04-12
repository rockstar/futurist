//! GATT service and characteristic UUIDs for the Stark Varg.
//!
//! All custom UUIDs share the base `XXXXXXXX-5374-6172-4b20-467574757265`
//! ("StarK Future" in ASCII).

use uuid::Uuid;

/// Primary GATT service UUID advertised by every Stark Varg.
pub const UUID_BIKE_SERVICE: Uuid = Uuid::from_u128(0x00001000_5374_6172_4b20_467574757265);

/// Security characteristic (write + notify).
///
/// Used for the V2 challenge-response auth handshake on firmware that
/// requires it. On firmware where this characteristic is not readable,
/// auth is skipped and the BLE bond alone is sufficient.
pub const UUID_SECURITY: Uuid = Uuid::from_u128(0x00001001_5374_6172_4b20_467574757265);

/// Bike Data — primary telemetry frame (read + notify, 18 bytes observed).
pub const UUID_BIKE_DATA: Uuid = Uuid::from_u128(0x00001002_5374_6172_4b20_467574757265);

/// Bike Data 2 — secondary telemetry frame (read + notify, 23 bytes observed).
pub const UUID_BIKE_DATA_2: Uuid = Uuid::from_u128(0x00001003_5374_6172_4b20_467574757265);

/// Live Data — large telemetry struct (read + notify, 100 bytes observed).
pub const UUID_LIVE_DATA: Uuid = Uuid::from_u128(0x00001005_5374_6172_4b20_467574757265);

/// Command — write-only characteristic for configuration changes.
pub const UUID_COMMAND: Uuid = Uuid::from_u128(0x00001006_5374_6172_4b20_467574757265);

/// VCU Data — vehicle control unit data (read + notify).
pub const UUID_VCU_DATA: Uuid = Uuid::from_u128(0x00001100_5374_6172_4b20_467574757265);

/// Standard BLE Battery Level characteristic.
pub const UUID_BATTERY_LEVEL: Uuid = Uuid::from_u128(0x00002a19_0000_1000_8000_00805f9b34fb);

/// Default sold-on date used for bikes that were never officially sold
/// or activated. Also the fallback after failed pairing attempts.
pub const SOLD_DATE_DEFAULT: &str = "19700101";
