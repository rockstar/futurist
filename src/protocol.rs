//! GATT service and characteristic UUIDs for the Stark Varg.
//!
//! All custom UUIDs share the base `XXXXXXXX-5374-6172-4b20-467574757265`
//! ("StarK Future" in ASCII).

use uuid::Uuid;

// ---------------------------------------------------------------------------
// Service UUIDs
// ---------------------------------------------------------------------------

pub const UUID_SVC_BIKE: Uuid = Uuid::from_u128(0x00001000_5374_6172_4b20_467574757265);
pub const UUID_SVC_LIVE: Uuid = Uuid::from_u128(0x00002000_5374_6172_4b20_467574757265);
pub const UUID_SVC_DOCKING: Uuid = Uuid::from_u128(0x00003000_5374_6172_4b20_467574757265);
pub const UUID_SVC_VCU: Uuid = Uuid::from_u128(0x00004000_5374_6172_4b20_467574757265);
pub const UUID_SVC_CHARGER: Uuid = Uuid::from_u128(0x00005000_5374_6172_4b20_467574757265);
pub const UUID_SVC_BATTERY: Uuid = Uuid::from_u128(0x00006000_5374_6172_4b20_467574757265);
pub const UUID_SVC_INVERTER: Uuid = Uuid::from_u128(0x00007000_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Service 0x1000 — Bike Data
// ---------------------------------------------------------------------------

pub const UUID_SECURITY: Uuid = Uuid::from_u128(0x00001001_5374_6172_4b20_467574757265);
pub const UUID_STATUS_BITS: Uuid = Uuid::from_u128(0x00001002_5374_6172_4b20_467574757265);
pub const UUID_IDENTITY: Uuid = Uuid::from_u128(0x00001003_5374_6172_4b20_467574757265);
pub const UUID_VERSIONS: Uuid = Uuid::from_u128(0x00001005_5374_6172_4b20_467574757265);
pub const UUID_COMMAND: Uuid = Uuid::from_u128(0x00001006_5374_6172_4b20_467574757265);
pub const UUID_EXTENDED_TLV: Uuid = Uuid::from_u128(0x00001100_5374_6172_4b20_467574757265);
pub const UUID_EXTENDED_CONFIG: Uuid = Uuid::from_u128(0x00001101_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Service 0x2000 — Live Data
// ---------------------------------------------------------------------------

pub const UUID_SPEED: Uuid = Uuid::from_u128(0x00002001_5374_6172_4b20_467574757265);
pub const UUID_THROTTLE: Uuid = Uuid::from_u128(0x00002002_5374_6172_4b20_467574757265);
pub const UUID_IMU: Uuid = Uuid::from_u128(0x00002003_5374_6172_4b20_467574757265);
pub const UUID_MAPS: Uuid = Uuid::from_u128(0x00002004_5374_6172_4b20_467574757265);
pub const UUID_TOTALS: Uuid = Uuid::from_u128(0x00002005_5374_6172_4b20_467574757265);
pub const UUID_ESTIMATIONS: Uuid = Uuid::from_u128(0x00002006_5374_6172_4b20_467574757265);
pub const UUID_RACING: Uuid = Uuid::from_u128(0x00002007_5374_6172_4b20_467574757265);
pub const UUID_LIVE_CONFIG: Uuid = Uuid::from_u128(0x00002008_5374_6172_4b20_467574757265);
pub const UUID_LIVE_TLV: Uuid = Uuid::from_u128(0x00002100_5374_6172_4b20_467574757265);
pub const UUID_LIVE_EXT_CONFIG: Uuid = Uuid::from_u128(0x00002101_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Service 0x3000 — Docking
// ---------------------------------------------------------------------------

pub const UUID_DOCKING_DATA_1: Uuid = Uuid::from_u128(0x00003001_5374_6172_4b20_467574757265);
pub const UUID_DOCKING_DATA_2: Uuid = Uuid::from_u128(0x00003002_5374_6172_4b20_467574757265);
pub const UUID_DOCKING_TLV: Uuid = Uuid::from_u128(0x00003100_5374_6172_4b20_467574757265);
pub const UUID_DOCKING_CONFIG: Uuid = Uuid::from_u128(0x00003101_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Service 0x4000 — VCU
// ---------------------------------------------------------------------------

pub const UUID_VCU_VERSIONS: Uuid = Uuid::from_u128(0x00004001_5374_6172_4b20_467574757265);
pub const UUID_VCU_INFO: Uuid = Uuid::from_u128(0x00004002_5374_6172_4b20_467574757265);
pub const UUID_VCU_CONFIG: Uuid = Uuid::from_u128(0x00004005_5374_6172_4b20_467574757265);
pub const UUID_VCU_TLV: Uuid = Uuid::from_u128(0x00004100_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Service 0x5000 — Charger
// ---------------------------------------------------------------------------

pub const UUID_CHARGER_DATA: Uuid = Uuid::from_u128(0x00005001_5374_6172_4b20_467574757265);
pub const UUID_CHARGER_TLV: Uuid = Uuid::from_u128(0x00005100_5374_6172_4b20_467574757265);
pub const UUID_CHARGER_CONFIG: Uuid = Uuid::from_u128(0x00005101_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Service 0x6000 — Battery
// ---------------------------------------------------------------------------

pub const UUID_BATTERY_DATA_1: Uuid = Uuid::from_u128(0x00006001_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_DATA_2: Uuid = Uuid::from_u128(0x00006002_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_DATA_3: Uuid = Uuid::from_u128(0x00006003_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_DATA_4: Uuid = Uuid::from_u128(0x00006004_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_DATA_5: Uuid = Uuid::from_u128(0x00006005_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_DATA_6: Uuid = Uuid::from_u128(0x00006006_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_DATA_7: Uuid = Uuid::from_u128(0x00006007_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_DATA_8: Uuid = Uuid::from_u128(0x00006008_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_DATA_9: Uuid = Uuid::from_u128(0x00006009_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_TLV: Uuid = Uuid::from_u128(0x00006100_5374_6172_4b20_467574757265);
pub const UUID_BATTERY_CONFIG: Uuid = Uuid::from_u128(0x00006101_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Service 0x7000 — Inverter
// ---------------------------------------------------------------------------

pub const UUID_INVERTER_DATA_1: Uuid = Uuid::from_u128(0x00007001_5374_6172_4b20_467574757265);
pub const UUID_INVERTER_DATA_2: Uuid = Uuid::from_u128(0x00007002_5374_6172_4b20_467574757265);
pub const UUID_INVERTER_DATA_3: Uuid = Uuid::from_u128(0x00007003_5374_6172_4b20_467574757265);
pub const UUID_INVERTER_DATA_4: Uuid = Uuid::from_u128(0x00007004_5374_6172_4b20_467574757265);
pub const UUID_INVERTER_TLV: Uuid = Uuid::from_u128(0x00007100_5374_6172_4b20_467574757265);
pub const UUID_INVERTER_CONFIG: Uuid = Uuid::from_u128(0x00007101_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Standard BLE
// ---------------------------------------------------------------------------

pub const UUID_BATTERY_LEVEL: Uuid = Uuid::from_u128(0x00002a19_0000_1000_8000_00805f9b34fb);

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const SOLD_DATE_DEFAULT: &str = "19700101";

/// Look up a human-readable name for any known characteristic UUID.
pub fn characteristic_name(uuid: Uuid) -> Option<&'static str> {
    Some(match uuid {
        // 0x1000 — Bike Data
        UUID_SECURITY => "security",
        UUID_STATUS_BITS => "status",
        UUID_IDENTITY => "identity",
        UUID_VERSIONS => "versions",
        UUID_COMMAND => "command",
        UUID_EXTENDED_TLV => "bike_tlv",
        UUID_EXTENDED_CONFIG => "bike_ext_cfg",

        // 0x2000 — Live Data
        UUID_SPEED => "speed",
        UUID_THROTTLE => "throttle",
        UUID_IMU => "imu",
        UUID_MAPS => "ride_mode",
        UUID_TOTALS => "totals",
        UUID_ESTIMATIONS => "estimates",
        UUID_RACING => "racing",
        UUID_LIVE_CONFIG => "live_cfg",
        UUID_LIVE_TLV => "live_tlv",
        UUID_LIVE_EXT_CONFIG => "live_ext_cfg",

        // 0x3000 — Docking
        UUID_DOCKING_DATA_1 => "dock_1",
        UUID_DOCKING_DATA_2 => "dock_2",
        UUID_DOCKING_TLV => "dock_tlv",
        UUID_DOCKING_CONFIG => "dock_cfg",

        // 0x4000 — VCU
        UUID_VCU_VERSIONS => "vcu_ver",
        UUID_VCU_INFO => "vcu_info",
        UUID_VCU_CONFIG => "vcu_cfg",
        UUID_VCU_TLV => "vcu_tlv",

        // 0x5000 — Charger
        UUID_CHARGER_DATA => "charger",
        UUID_CHARGER_TLV => "chg_tlv",
        UUID_CHARGER_CONFIG => "chg_cfg",

        // 0x6000 — Battery
        UUID_BATTERY_DATA_1 => "batt_1",
        UUID_BATTERY_DATA_2 => "batt_2",
        UUID_BATTERY_DATA_3 => "batt_3",
        UUID_BATTERY_DATA_4 => "batt_4",
        UUID_BATTERY_DATA_5 => "batt_5",
        UUID_BATTERY_DATA_6 => "batt_6",
        UUID_BATTERY_DATA_7 => "batt_7",
        UUID_BATTERY_DATA_8 => "batt_8",
        UUID_BATTERY_DATA_9 => "batt_9",
        UUID_BATTERY_TLV => "batt_tlv",
        UUID_BATTERY_CONFIG => "batt_cfg",

        // 0x7000 — Inverter
        UUID_INVERTER_DATA_1 => "inv_1",
        UUID_INVERTER_DATA_2 => "inv_2",
        UUID_INVERTER_DATA_3 => "inv_3",
        UUID_INVERTER_DATA_4 => "inv_4",
        UUID_INVERTER_TLV => "inv_tlv",
        UUID_INVERTER_CONFIG => "inv_cfg",

        // Standard BLE
        UUID_BATTERY_LEVEL => "battery",

        _ => return None,
    })
}

/// Look up which service a characteristic belongs to.
pub fn service_name(uuid: Uuid) -> Option<&'static str> {
    Some(match uuid {
        UUID_SVC_BIKE => "bike",
        UUID_SVC_LIVE => "live",
        UUID_SVC_DOCKING => "docking",
        UUID_SVC_VCU => "vcu",
        UUID_SVC_CHARGER => "charger",
        UUID_SVC_BATTERY => "battery",
        UUID_SVC_INVERTER => "inverter",
        _ => return None,
    })
}
