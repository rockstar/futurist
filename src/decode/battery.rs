//! Decoders for service 0x6000 — Battery.

use super::{opt_u16, read_u16, read_u32, read_version};

/// 0x6001 — Battery fault bits (8 bytes).
#[derive(Debug, Clone, Default)]
pub struct BatteryStatus {
    pub fault_bits_pos: u32,
    pub fault_bits_neg: u32,
}

impl BatteryStatus {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        Some(Self {
            fault_bits_pos: read_u32(data, 0),
            fault_bits_neg: read_u32(data, 4),
        })
    }

    pub fn has_faults(&self) -> bool {
        self.fault_bits_pos != 0 || self.fault_bits_neg != 0
    }
}

/// 0x6002 — Battery firmware versions (12 bytes).
#[derive(Debug, Clone)]
pub struct BatteryFirmwareVersion {
    pub pos_version: String,
    pub neg_version: String,
    pub serial: u32,
}

impl BatteryFirmwareVersion {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        Some(Self {
            pos_version: read_version(data, 0),
            neg_version: read_version(data, 4),
            serial: read_u32(data, 8),
        })
    }
}

/// 0x6003 — Battery parameters (4 bytes).
#[derive(Debug, Clone, Default)]
pub struct BatteryParams {
    pub series: u8,
    pub parallel: u8,
    pub capacity: u16,
}

impl BatteryParams {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        Some(Self {
            series: data[0],
            parallel: data[1],
            capacity: read_u16(data, 2),
        })
    }
}

/// 0x6004 — Battery state of charge (6 bytes).
#[derive(Debug, Clone, Default)]
pub struct BatterySoc {
    pub soc: Option<u16>,
    pub soh: Option<u16>,
    pub dc_bus: Option<u16>,
}

impl BatterySoc {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }
        Some(Self {
            soc: opt_u16(data, 0),
            soh: opt_u16(data, 2),
            dc_bus: opt_u16(data, 4),
        })
    }
}

/// 0x6005 — Battery temperatures (27 bytes).
#[derive(Debug, Clone)]
pub struct BatteryTemperatures {
    /// 12 temperature sensor readings (u16 each).
    pub sensors: Vec<Option<u16>>,
    pub valid: u16,
    pub used: u8,
}

impl BatteryTemperatures {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 27 {
            return None;
        }
        let sensors = (0..12).map(|i| opt_u16(data, i * 2)).collect();
        Some(Self {
            sensors,
            valid: read_u16(data, 24),
            used: data[26],
        })
    }
}

/// 0x6007 — Battery cell voltages (200 bytes: 100 x u16).
#[derive(Debug, Clone)]
pub struct BatteryCells {
    pub voltages: Vec<Option<u16>>,
}

impl BatteryCells {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 200 {
            return None;
        }
        let voltages = (0..100).map(|i| opt_u16(data, i * 2)).collect();
        Some(Self { voltages })
    }

    /// Number of cells reporting real (non-sentinel) values.
    pub fn active_count(&self) -> usize {
        self.voltages.iter().filter(|v| v.is_some()).count()
    }
}

/// 0x6009 — Battery BMS signals (18 bytes).
#[derive(Debug, Clone, Default)]
pub struct BatterySignals {
    pub pos_dc_bus: Option<u16>,
    pub pos_temp: Option<u16>,
    pub pos_humidity: Option<u16>,
    pub pos_control: Option<u16>,
    pub neg_dc_bus: Option<u16>,
    pub neg_temp: Option<u16>,
    pub neg_humidity: Option<u16>,
    pub neg_control: Option<u16>,
    pub current: Option<i16>,
}

impl BatterySignals {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 18 {
            return None;
        }
        Some(Self {
            pos_dc_bus: opt_u16(data, 0),
            pos_temp: opt_u16(data, 2),
            pos_humidity: opt_u16(data, 4),
            pos_control: opt_u16(data, 6),
            neg_dc_bus: opt_u16(data, 8),
            neg_temp: opt_u16(data, 10),
            neg_humidity: opt_u16(data, 12),
            neg_control: opt_u16(data, 14),
            current: super::opt_i16(data, 16),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_battery_params() {
        let data = [10, 2, 0xE8, 0x03]; // 10s2p, 1000 mAh
        let p = BatteryParams::parse(&data).unwrap();
        assert_eq!(p.series, 10);
        assert_eq!(p.parallel, 2);
        assert_eq!(p.capacity, 1000);
    }

    #[test]
    fn parse_battery_soc() {
        let mut data = [0u8; 6];
        data[0..2].copy_from_slice(&85u16.to_le_bytes());
        data[2..4].copy_from_slice(&97u16.to_le_bytes());
        data[4..6].copy_from_slice(&380u16.to_le_bytes());
        let s = BatterySoc::parse(&data).unwrap();
        assert_eq!(s.soc, Some(85));
        assert_eq!(s.soh, Some(97));
        assert_eq!(s.dc_bus, Some(380));
    }

    #[test]
    fn parse_battery_status_no_faults() {
        let data = [0u8; 8];
        let s = BatteryStatus::parse(&data).unwrap();
        assert!(!s.has_faults());
    }
}
