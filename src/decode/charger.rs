//! Decoders for service 0x5000 — Charger.

use super::opt_u16;

/// 0x5001 — Charger data (up to 18 bytes).
#[derive(Debug, Clone, Default)]
pub struct ChargerData {
    pub req_current: Option<u16>,
    pub reported_current: Option<u16>,
    pub cell_charge_voltage: Option<u16>,
    pub max_charge_current: Option<u16>,
    pub max_charge_power: Option<u16>,
    pub max_charge_soc: Option<u16>,
    pub req_voltage: Option<u16>,
    pub reported_voltage: Option<u16>,
    pub charger_status: u8,
    pub charger_enabled: bool,
}

impl ChargerData {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 14 {
            return None;
        }
        Some(Self {
            req_current: opt_u16(data, 0),
            reported_current: opt_u16(data, 2),
            cell_charge_voltage: opt_u16(data, 4),
            max_charge_current: opt_u16(data, 6),
            max_charge_power: opt_u16(data, 8),
            max_charge_soc: opt_u16(data, 10),
            req_voltage: opt_u16(data, 12),
            reported_voltage: if data.len() >= 16 {
                opt_u16(data, 14)
            } else {
                None
            },
            charger_status: if data.len() >= 17 { data[16] } else { 0 },
            charger_enabled: data.len() >= 18 && data[17] == 1,
        })
    }
}
