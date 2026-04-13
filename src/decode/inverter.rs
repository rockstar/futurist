//! Decoders for service 0x7000 — Inverter.

use super::{opt_i16, opt_u16, read_u16, read_u32, read_version};

/// 0x7001 — Inverter info.
#[derive(Debug, Clone)]
pub struct InverterInfo {
    pub faults: u16,
    pub status: u32,
    pub mcc_data1_bad: u16,
    pub mcc_data2_bad: u16,
    pub logic_fw_version: String,
    pub gate_fw_version: String,
    pub hardware_version: u8,
    pub humidity: u32,
}

impl InverterInfo {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 23 {
            return None;
        }
        Some(Self {
            faults: read_u16(data, 0),
            status: read_u32(data, 2),
            mcc_data1_bad: read_u16(data, 6),
            mcc_data2_bad: read_u16(data, 8),
            logic_fw_version: read_version(data, 10),
            gate_fw_version: read_version(data, 14),
            hardware_version: data[18],
            humidity: read_u32(data, 19),
        })
    }
}

/// 0x7002 — Inverter signals (14 bytes).
#[derive(Debug, Clone, Default)]
pub struct InverterSignals {
    pub dc_bus: Option<u16>,
    pub iq_ref: Option<u16>,
    pub id_ref: Option<u16>,
    pub iq: Option<i16>,
    pub id: Option<i16>,
    pub vq: Option<u16>,
    pub vd: Option<u16>,
}

impl InverterSignals {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 14 {
            return None;
        }
        Some(Self {
            dc_bus: opt_u16(data, 0),
            iq_ref: opt_u16(data, 2),
            id_ref: opt_u16(data, 4),
            iq: opt_i16(data, 6),
            id: opt_i16(data, 8),
            vq: opt_u16(data, 10),
            vd: opt_u16(data, 12),
        })
    }
}

/// Temperature sensor group (8 bytes: 3 sensors + valid + used).
#[derive(Debug, Clone, Default)]
pub struct TempSensors {
    pub sensor1: Option<u16>,
    pub sensor2: Option<u16>,
    pub sensor3: Option<u16>,
    pub valid: u8,
    pub used: u8,
}

impl TempSensors {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        Some(Self {
            sensor1: opt_u16(data, 0),
            sensor2: opt_u16(data, 2),
            sensor3: opt_u16(data, 4),
            valid: data[6],
            used: data[7],
        })
    }
}

/// 0x7003 — Inverter temperatures (16 bytes: motor + IGBT).
#[derive(Debug, Clone, Default)]
pub struct InverterTemperatures {
    pub motor: TempSensors,
    pub igbt: TempSensors,
}

impl InverterTemperatures {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }
        Some(Self {
            motor: TempSensors::parse(&data[0..8])?,
            igbt: TempSensors::parse(&data[8..16])?,
        })
    }
}

/// 0x7004 — Inverter PCB data (18 bytes).
#[derive(Debug, Clone, Default)]
pub struct InverterPcb {
    pub mcu_temp_logic: Option<u16>,
    pub mcu_temp_gate: Option<u16>,
    pub ntc1: Option<u16>,
    pub ntc2: Option<u16>,
    pub ntc3: Option<u16>,
    pub pcb_temp: Option<u16>,
    pub pcb_humidity: Option<u16>,
    pub serial_number: u32,
}

impl InverterPcb {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 18 {
            return None;
        }
        Some(Self {
            mcu_temp_logic: opt_u16(data, 0),
            mcu_temp_gate: opt_u16(data, 2),
            ntc1: opt_u16(data, 4),
            ntc2: opt_u16(data, 6),
            ntc3: opt_u16(data, 8),
            pcb_temp: opt_u16(data, 10),
            pcb_humidity: opt_u16(data, 12),
            serial_number: read_u32(data, 14),
        })
    }
}
