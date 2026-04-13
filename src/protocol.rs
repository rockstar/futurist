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

/// Battery fault bits — 8 bytes: fault_bits_pos (u32), fault_bits_neg (u32).
pub const UUID_BATT_STATUS: Uuid = Uuid::from_u128(0x00006001_5374_6172_4b20_467574757265);
/// Battery firmware versions — 12 bytes: pos version, neg version, serial.
pub const UUID_BATT_FW_VERSION: Uuid = Uuid::from_u128(0x00006002_5374_6172_4b20_467574757265);
/// Battery params — 4 bytes: series (u8), parallel (u8), capacity (u16).
pub const UUID_BATT_PARAMS: Uuid = Uuid::from_u128(0x00006003_5374_6172_4b20_467574757265);
/// Battery SOC — 6 bytes: soc (u16), soh (u16), dc_bus (u16).
pub const UUID_BATT_SOC: Uuid = Uuid::from_u128(0x00006004_5374_6172_4b20_467574757265);
/// Battery temperatures — 27 bytes: 12x u16 temps + valid (u16) + used (u8).
pub const UUID_BATT_TEMPS: Uuid = Uuid::from_u128(0x00006005_5374_6172_4b20_467574757265);
/// Battery DC bus — not actively parsed by the app.
pub const UUID_BATT_DC_BUS: Uuid = Uuid::from_u128(0x00006006_5374_6172_4b20_467574757265);
/// Battery cell voltages — 200 bytes: 100x u16 cell voltages.
pub const UUID_BATT_CELLS: Uuid = Uuid::from_u128(0x00006007_5374_6172_4b20_467574757265);
/// Battery balancing — raw bytes (typically 13).
pub const UUID_BATT_BALANCING: Uuid = Uuid::from_u128(0x00006008_5374_6172_4b20_467574757265);
/// Battery BMS signals — 18 bytes: 2x BMSSignals (8 bytes each) + current (i16).
pub const UUID_BATT_SIGNALS: Uuid = Uuid::from_u128(0x00006009_5374_6172_4b20_467574757265);
/// Battery config (0x600A).
pub const UUID_BATT_CFG: Uuid = Uuid::from_u128(0x0000600a_5374_6172_4b20_467574757265);
pub const UUID_BATT_TLV: Uuid = Uuid::from_u128(0x00006100_5374_6172_4b20_467574757265);
pub const UUID_BATT_EXT_CONFIG: Uuid = Uuid::from_u128(0x00006101_5374_6172_4b20_467574757265);

// ---------------------------------------------------------------------------
// Service 0x7000 — Inverter
// ---------------------------------------------------------------------------

/// Inverter info — faults, status, firmware versions, humidity.
pub const UUID_INV_INFO: Uuid = Uuid::from_u128(0x00007001_5374_6172_4b20_467574757265);
/// Inverter signals — 14 bytes: dc_bus, iq/id ref/actual, vq, vd.
pub const UUID_INV_SIGNALS: Uuid = Uuid::from_u128(0x00007002_5374_6172_4b20_467574757265);
/// Inverter temperatures — 16 bytes: motor (3 sensors) + IGBT (3 sensors).
pub const UUID_INV_TEMPS: Uuid = Uuid::from_u128(0x00007003_5374_6172_4b20_467574757265);
/// Inverter PCB — 18 bytes: MCU temps, NTCs, PCB temp/humidity, serial.
pub const UUID_INV_PCB: Uuid = Uuid::from_u128(0x00007004_5374_6172_4b20_467574757265);
pub const UUID_INV_TLV: Uuid = Uuid::from_u128(0x00007100_5374_6172_4b20_467574757265);
pub const UUID_INV_CONFIG: Uuid = Uuid::from_u128(0x00007101_5374_6172_4b20_467574757265);

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
        UUID_DOCKING_DATA_1 => "dock_ver",
        UUID_DOCKING_DATA_2 => "dock_qi",
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
        UUID_BATT_STATUS => "batt_status",
        UUID_BATT_FW_VERSION => "batt_fw",
        UUID_BATT_PARAMS => "batt_params",
        UUID_BATT_SOC => "batt_soc",
        UUID_BATT_TEMPS => "batt_temps",
        UUID_BATT_DC_BUS => "batt_dc_bus",
        UUID_BATT_CELLS => "batt_cells",
        UUID_BATT_BALANCING => "batt_bal",
        UUID_BATT_SIGNALS => "batt_signals",
        UUID_BATT_CFG => "batt_cfg",
        UUID_BATT_TLV => "batt_tlv",
        UUID_BATT_EXT_CONFIG => "batt_ext_cfg",

        // 0x7000 — Inverter
        UUID_INV_INFO => "inv_info",
        UUID_INV_SIGNALS => "inv_signals",
        UUID_INV_TEMPS => "inv_temps",
        UUID_INV_PCB => "inv_pcb",
        UUID_INV_TLV => "inv_tlv",
        UUID_INV_CONFIG => "inv_cfg",

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
