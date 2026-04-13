//! Decoders for service 0x4000 — VCU.

use super::{opt_u16, read_u32, read_version};

/// 0x4001 — VCU firmware versions (20 bytes).
#[derive(Debug, Clone)]
pub struct VcuVersions {
    pub pic_vcu: String,
    pub top_vcu: String,
    pub bottom_vcu: String,
    pub fwfs: String,
    pub serial_number: u32,
}

impl VcuVersions {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }
        Some(Self {
            pic_vcu: read_version(data, 0),
            top_vcu: read_version(data, 4),
            bottom_vcu: read_version(data, 8),
            fwfs: read_version(data, 12),
            serial_number: read_u32(data, 16),
        })
    }
}

/// 0x4002 — VCU info (10 bytes).
#[derive(Debug, Clone, Default)]
pub struct VcuInfo {
    pub fan_current: Option<u16>,
    pub pump_current: Option<u16>,
    pub pump_ok_counter: Option<u16>,
    /// Humidity in percent.
    pub humidity_pct: Option<f32>,
    /// Temperature in °C.
    pub temperature_c: Option<f32>,
}

impl VcuInfo {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 10 {
            return None;
        }
        Some(Self {
            fan_current: opt_u16(data, 0),
            pump_current: opt_u16(data, 2),
            pump_ok_counter: opt_u16(data, 4),
            humidity_pct: opt_u16(data, 6).map(|v| v as f32 / 100.0),
            temperature_c: opt_u16(data, 8).map(|v| v as f32 / 100.0),
        })
    }
}
