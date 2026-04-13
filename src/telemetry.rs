//! Live telemetry subscription and frame decoding for the Stark Varg.

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use btleplug::api::Peripheral as _;
use futures::Stream;
use futures::StreamExt;
use uuid::Uuid;

use crate::ble::{BikeConnection, Error};
use crate::protocol;

// ---------------------------------------------------------------------------
// Raw frame stream
// ---------------------------------------------------------------------------

/// A single raw telemetry notification from the bike.
#[derive(Debug, Clone)]
pub struct TelemetryFrame {
    pub characteristic: Uuid,
    pub timestamp: Instant,
    pub data: Vec<u8>,
}

/// A stream of [`TelemetryFrame`]s from a connected bike.
pub struct TelemetryStream {
    inner: Pin<Box<dyn Stream<Item = TelemetryFrame> + Send>>,
}

impl Stream for TelemetryStream {
    type Item = TelemetryFrame;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Subscribe to all notify-capable telemetry characteristics on the bike.
pub async fn subscribe(bike: &BikeConnection) -> Result<TelemetryStream, Error> {
    let peripheral = bike.peripheral();

    // Subscribe to every characteristic that supports notify, across all
    // services. This is dynamic — we don't need to know the UUIDs in
    // advance.
    let mut subscribed = 0;
    for c in bike.characteristics().values() {
        if c.properties.contains(btleplug::api::CharPropFlags::NOTIFY) {
            peripheral.subscribe(c).await?;
            subscribed += 1;
        }
    }
    eprintln!("  subscribed to {} notify characteristics", subscribed);

    let raw_stream = peripheral.notifications().await?;
    let mapped = raw_stream.map(|notif| TelemetryFrame {
        characteristic: notif.uuid,
        timestamp: Instant::now(),
        data: notif.value,
    });

    Ok(TelemetryStream {
        inner: Box::pin(mapped),
    })
}

// ---------------------------------------------------------------------------
// Decoded types — Service 0x1000
// ---------------------------------------------------------------------------

/// Status bits from characteristic 00001002 (18 bytes).
#[derive(Debug, Clone, Default)]
pub struct StatusBits {
    pub misc_bits: u16,
    pub indicator_bits: u16,
    pub alert_bits: u16,
    pub fault_bits: u16,
    pub info_bits: u16,
    pub lock_status: u8,
    pub lock_time: u16,
    pub update_available: bool,
    pub battery_status: u32,
}

impl StatusBits {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 18 {
            return None;
        }
        Some(Self {
            misc_bits: u16::from_le_bytes([data[0], data[1]]),
            indicator_bits: u16::from_le_bytes([data[2], data[3]]),
            alert_bits: u16::from_le_bytes([data[4], data[5]]),
            fault_bits: u16::from_le_bytes([data[6], data[7]]),
            info_bits: u16::from_le_bytes([data[8], data[9]]),
            lock_status: data[10],
            lock_time: u16::from_le_bytes([data[11], data[12]]),
            update_available: data[13] == 1,
            battery_status: u32::from_le_bytes([data[14], data[15], data[16], data[17]]),
        })
    }

    pub fn walking_mode(&self) -> u8 {
        (self.misc_bits & 0x0F) as u8
    }

    pub fn armed_throttle(&self) -> bool {
        (self.indicator_bits & 0x10) != 0
    }

    pub fn drive(&self) -> bool {
        (self.indicator_bits & 0x20) != 0
    }

    pub fn charger_connected(&self) -> bool {
        (self.info_bits & 0x01) != 0
    }

    pub fn is_charging(&self) -> bool {
        (self.info_bits & 0x02) != 0
    }

    pub fn is_docked(&self) -> bool {
        (self.info_bits & 0x04) != 0
    }

    pub fn pump_on(&self) -> bool {
        (self.info_bits & 0x20) != 0
    }

    pub fn fan_on(&self) -> bool {
        (self.info_bits & 0x40) != 0
    }
}

/// VIN and sold-on date from characteristic 00001003 (23 bytes).
#[derive(Debug, Clone)]
pub struct BikeIdentity {
    pub vin: String,
    pub sold_date: String,
}

impl BikeIdentity {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 23 {
            return None;
        }
        Some(Self {
            vin: String::from_utf8_lossy(&data[0..17]).to_string(),
            sold_date: String::from_utf8_lossy(&data[17..23]).to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Decoded types — Service 0x2000 (live riding data)
// ---------------------------------------------------------------------------

use crate::decode::{opt_i16, opt_u16, opt_u32};

/// Speed from characteristic 00002001 (4 bytes).
#[derive(Debug, Clone, Default)]
pub struct Speed {
    /// Speed in km/h (raw value / 10). None if sensor unavailable.
    pub speed_kmh: Option<f32>,
    /// Motor RPM. None if motor not running.
    pub motor_rpm: Option<u16>,
}

impl Speed {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        Some(Self {
            speed_kmh: opt_u16(data, 0).map(|v| v as f32 / 10.0),
            motor_rpm: opt_u16(data, 2),
        })
    }
}

/// Throttle from characteristic 00002002 (6 bytes).
#[derive(Debug, Clone, Default)]
pub struct Throttle {
    pub position: Option<u16>,
    pub iq_fb: Option<i16>,
    pub id_fb: Option<i16>,
}

impl Throttle {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }
        Some(Self {
            position: opt_u16(data, 0),
            iq_fb: opt_i16(data, 2),
            id_fb: opt_i16(data, 4),
        })
    }
}

/// IMU from characteristic 00002003 (12 bytes).
#[derive(Debug, Clone, Default)]
pub struct Imu {
    pub accel_x: Option<i16>,
    pub accel_y: Option<i16>,
    pub accel_z: Option<i16>,
    pub gyro_x: Option<i16>,
    pub gyro_y: Option<i16>,
    pub gyro_z: Option<i16>,
}

impl Imu {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        Some(Self {
            accel_x: opt_i16(data, 0),
            accel_y: opt_i16(data, 2),
            accel_z: opt_i16(data, 4),
            gyro_x: opt_i16(data, 6),
            gyro_y: opt_i16(data, 8),
            gyro_z: opt_i16(data, 10),
        })
    }
}

/// Totals from characteristic 00002005 (16 bytes).
#[derive(Debug, Clone, Default)]
pub struct Totals {
    pub odometer: Option<u32>,
    pub watt_hours: Option<u32>,
    pub airtime_secs: Option<u32>,
    pub total_time_secs: Option<u32>,
}

impl Totals {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }
        Some(Self {
            odometer: opt_u32(data, 0),
            watt_hours: opt_u32(data, 4),
            airtime_secs: opt_u32(data, 8),
            total_time_secs: opt_u32(data, 12),
        })
    }
}

/// Estimations from characteristic 00002006 (6 bytes).
#[derive(Debug, Clone, Default)]
pub struct Estimations {
    pub range_km: Option<u16>,
    pub time_min: Option<u16>,
    pub motor_power_w: Option<i16>,
}

impl Estimations {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }
        Some(Self {
            range_km: opt_u16(data, 0),
            time_min: opt_u16(data, 2),
            motor_power_w: opt_i16(data, 4),
        })
    }
}

/// Racing from characteristic 00002007 (9 bytes).
#[derive(Debug, Clone, Default)]
pub struct Racing {
    pub mode: u8,
    pub curve: u8,
    pub throttle_multiplier: u16,
    pub category: u8,
    pub expire_timestamp: u32,
}

impl Racing {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }
        Some(Self {
            mode: data[0],
            curve: data[1],
            throttle_multiplier: u16::from_le_bytes([data[2], data[3]]),
            category: data[4],
            expire_timestamp: u32::from_le_bytes([data[5], data[6], data[7], data[8]]),
        })
    }
}

/// Firmware version info from characteristic 00001005 (100 bytes).
#[derive(Debug, Clone)]
pub struct BikeVersions {
    pub ble_version: u16,
    pub download_percent: i16,
    pub blob_fs: String,
    pub blob_server: String,
    pub components: Vec<ComponentVersion>,
}

/// A single component's firmware version.
#[derive(Debug, Clone)]
pub struct ComponentVersion {
    pub name: &'static str,
    pub version: String,
    pub available: String,
}

impl BikeVersions {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 100 {
            return None;
        }
        let ble_version = u16::from_le_bytes([data[0], data[1]]);
        let download_percent = i16::from_le_bytes([data[2], data[3]]);
        let blob_fs = format!("{}.{}.{}", data[6], data[5], data[4]);
        let blob_server = format!("{}.{}.{}", data[10], data[9], data[8]);

        // 11 component pairs starting at byte 12, 8 bytes per pair
        // (4 bytes version + 4 bytes available version).
        let comp_data = &data[12..];
        let names = [
            "vcu_pic",
            "esp_top",
            "esp_bottom",
            "inverter_logic",
            "inverter_gate",
            "bms_pos",
            "bms_neg",
            "map_switch",
            "light_front",
            "light_rear",
            "docking",
        ];
        let mut components = Vec::with_capacity(names.len());
        for (i, name) in names.iter().enumerate() {
            let off = i * 8;
            if off + 7 < comp_data.len() {
                components.push(ComponentVersion {
                    name,
                    version: format!(
                        "{}.{}.{}",
                        comp_data[off + 2],
                        comp_data[off + 1],
                        comp_data[off]
                    ),
                    available: format!(
                        "{}.{}.{}",
                        comp_data[off + 6],
                        comp_data[off + 5],
                        comp_data[off + 4]
                    ),
                });
            }
        }

        Some(Self {
            ble_version,
            download_percent,
            blob_fs,
            blob_server,
            components,
        })
    }
}

/// A single TLV (Type-Length-Value) entry from extended data characteristics.
#[derive(Debug, Clone)]
pub struct TlvEntry {
    pub entry_type: u8,
    pub data: Vec<u8>,
}

/// Parse TLV-encoded data from characteristics 00001100 / 00002100.
pub fn parse_tlv(raw: &[u8]) -> Vec<TlvEntry> {
    let mut entries = Vec::new();
    let mut offset = 0;
    while offset + 2 <= raw.len() {
        let entry_type = raw[offset];
        let length = raw[offset + 1] as usize;
        let data_start = offset + 2;
        let data_end = data_start + length;
        if data_end > raw.len() {
            break;
        }
        entries.push(TlvEntry {
            entry_type,
            data: raw[data_start..data_end].to_vec(),
        });
        offset = data_end;
    }
    entries
}

/// Battery level from characteristic 00002a19 (1 byte, standard BLE).
pub fn parse_battery_level(data: &[u8]) -> Option<u8> {
    data.first().copied()
}

// ---------------------------------------------------------------------------
// Aggregated decoded state
// ---------------------------------------------------------------------------

use crate::decode::{battery, charger, docking, inverter, vcu};

/// Decoded telemetry snapshot assembled from multiple characteristics.
#[derive(Debug, Clone, Default)]
pub struct DecodedTelemetry {
    // Service 0x1000
    pub status: Option<StatusBits>,
    pub identity: Option<BikeIdentity>,
    pub versions: Option<BikeVersions>,
    pub tlv_entries: Vec<TlvEntry>,
    pub battery_percent: Option<u8>,

    // Service 0x2000
    pub speed: Option<Speed>,
    pub throttle: Option<Throttle>,
    pub imu: Option<Imu>,
    pub ride_mode: Option<u8>,
    pub totals: Option<Totals>,
    pub estimations: Option<Estimations>,
    pub racing: Option<Racing>,

    // Service 0x3000
    pub docking_version: Option<docking::DockingVersion>,
    pub docking_qi: Option<docking::DockingQiStatus>,

    // Service 0x4000
    pub vcu_versions: Option<vcu::VcuVersions>,
    pub vcu_info: Option<vcu::VcuInfo>,

    // Service 0x5000
    pub charger: Option<charger::ChargerData>,

    // Service 0x6000
    pub batt_status: Option<battery::BatteryStatus>,
    pub batt_fw_version: Option<battery::BatteryFirmwareVersion>,
    pub batt_params: Option<battery::BatteryParams>,
    pub batt_soc: Option<battery::BatterySoc>,
    pub batt_temps: Option<battery::BatteryTemperatures>,
    pub batt_cells: Option<battery::BatteryCells>,
    pub batt_signals: Option<battery::BatterySignals>,

    // Service 0x7000
    pub inv_info: Option<inverter::InverterInfo>,
    pub inv_signals: Option<inverter::InverterSignals>,
    pub inv_temps: Option<inverter::InverterTemperatures>,
    pub inv_pcb: Option<inverter::InverterPcb>,
}

impl DecodedTelemetry {
    /// Update this snapshot with a new raw frame.
    pub fn update(&mut self, frame: &TelemetryFrame) {
        match frame.characteristic {
            // Service 0x1000 — dedicated
            protocol::UUID_STATUS_BITS => self.status = StatusBits::parse(&frame.data),
            protocol::UUID_IDENTITY => self.identity = BikeIdentity::parse(&frame.data),
            protocol::UUID_BATTERY_LEVEL => self.battery_percent = parse_battery_level(&frame.data),
            protocol::UUID_VERSIONS => self.versions = BikeVersions::parse(&frame.data),

            // Service 0x1000 — TLV
            protocol::UUID_EXTENDED_TLV => {
                let entries = parse_tlv(&frame.data);
                for entry in &entries {
                    self.decode_bike_tlv(entry);
                }
                self.tlv_entries = entries;
            }

            // Service 0x2000 — dedicated
            protocol::UUID_SPEED => self.speed = Speed::parse(&frame.data),
            protocol::UUID_THROTTLE => self.throttle = Throttle::parse(&frame.data),
            protocol::UUID_IMU => self.imu = Imu::parse(&frame.data),
            protocol::UUID_MAPS => self.ride_mode = frame.data.first().copied(),
            protocol::UUID_TOTALS => self.totals = Totals::parse(&frame.data),
            protocol::UUID_ESTIMATIONS => self.estimations = Estimations::parse(&frame.data),
            protocol::UUID_RACING => self.racing = Racing::parse(&frame.data),

            // Service 0x2000 — TLV
            protocol::UUID_LIVE_TLV => {
                for entry in parse_tlv(&frame.data) {
                    self.decode_live_tlv(&entry);
                }
            }

            // Service 0x3000 — dedicated
            protocol::UUID_DOCKING_DATA_1 => {
                self.docking_version = docking::DockingVersion::parse(&frame.data)
            }
            protocol::UUID_DOCKING_DATA_2 => {
                self.docking_qi = docking::DockingQiStatus::parse(&frame.data)
            }
            // Service 0x3000 — TLV
            protocol::UUID_DOCKING_TLV => {
                for entry in parse_tlv(&frame.data) {
                    match entry.entry_type {
                        1 => self.docking_version = docking::DockingVersion::parse(&entry.data),
                        2 => self.docking_qi = docking::DockingQiStatus::parse(&entry.data),
                        _ => {}
                    }
                }
            }

            // Service 0x4000 — dedicated
            protocol::UUID_VCU_VERSIONS => self.vcu_versions = vcu::VcuVersions::parse(&frame.data),
            protocol::UUID_VCU_INFO => self.vcu_info = vcu::VcuInfo::parse(&frame.data),
            // Service 0x4000 — TLV
            protocol::UUID_VCU_TLV => {
                for entry in parse_tlv(&frame.data) {
                    match entry.entry_type {
                        1 => self.vcu_versions = vcu::VcuVersions::parse(&entry.data),
                        2 => self.vcu_info = vcu::VcuInfo::parse(&entry.data),
                        _ => {}
                    }
                }
            }

            // Service 0x5000 — dedicated
            protocol::UUID_CHARGER_DATA => self.charger = charger::ChargerData::parse(&frame.data),
            // Service 0x5000 — TLV
            protocol::UUID_CHARGER_TLV => {
                for entry in parse_tlv(&frame.data) {
                    if entry.entry_type == 1 {
                        self.charger = charger::ChargerData::parse(&entry.data);
                    }
                }
            }

            // Service 0x6000 — dedicated
            protocol::UUID_BATT_STATUS => {
                self.batt_status = battery::BatteryStatus::parse(&frame.data)
            }
            protocol::UUID_BATT_FW_VERSION => {
                self.batt_fw_version = battery::BatteryFirmwareVersion::parse(&frame.data)
            }
            protocol::UUID_BATT_PARAMS => {
                self.batt_params = battery::BatteryParams::parse(&frame.data)
            }
            protocol::UUID_BATT_SOC => self.batt_soc = battery::BatterySoc::parse(&frame.data),
            protocol::UUID_BATT_TEMPS => {
                self.batt_temps = battery::BatteryTemperatures::parse(&frame.data)
            }
            protocol::UUID_BATT_CELLS => {
                self.batt_cells = battery::BatteryCells::parse(&frame.data)
            }
            protocol::UUID_BATT_SIGNALS => {
                self.batt_signals = battery::BatterySignals::parse(&frame.data)
            }
            // Service 0x6000 — TLV
            protocol::UUID_BATT_TLV => {
                for entry in parse_tlv(&frame.data) {
                    self.decode_battery_tlv(&entry);
                }
            }

            // Service 0x7000 — dedicated
            protocol::UUID_INV_INFO => self.inv_info = inverter::InverterInfo::parse(&frame.data),
            protocol::UUID_INV_SIGNALS => {
                self.inv_signals = inverter::InverterSignals::parse(&frame.data)
            }
            protocol::UUID_INV_TEMPS => {
                self.inv_temps = inverter::InverterTemperatures::parse(&frame.data)
            }
            protocol::UUID_INV_PCB => self.inv_pcb = inverter::InverterPcb::parse(&frame.data),
            // Service 0x7000 — TLV
            protocol::UUID_INV_TLV => {
                for entry in parse_tlv(&frame.data) {
                    self.decode_inverter_tlv(&entry);
                }
            }

            _ => {}
        }
    }

    fn decode_bike_tlv(&mut self, entry: &TlvEntry) {
        match entry.entry_type {
            // Type 1: Fast Bits — compact status update (8 bytes)
            1 if entry.data.len() >= 8 => {
                if let Some(ref mut s) = self.status {
                    s.misc_bits = u16::from_le_bytes([entry.data[0], entry.data[1]]);
                    s.indicator_bits = u16::from_le_bytes([entry.data[2], entry.data[3]]);
                    s.alert_bits = u16::from_le_bytes([entry.data[4], entry.data[5]]);
                    s.info_bits = u16::from_le_bytes([entry.data[6], entry.data[7]]);
                }
            }
            // Type 2: Lock Status (3 bytes)
            2 if entry.data.len() >= 3 => {
                if let Some(ref mut s) = self.status {
                    s.lock_status = entry.data[0];
                    s.lock_time = u16::from_le_bytes([entry.data[1], entry.data[2]]);
                }
            }
            // Type 3: Update Available (1 byte)
            3 if !entry.data.is_empty() => {
                if let Some(ref mut s) = self.status {
                    s.update_available = entry.data[0] == 1;
                }
            }
            _ => {}
        }
    }

    fn decode_live_tlv(&mut self, entry: &TlvEntry) {
        match entry.entry_type {
            1 => self.speed = Speed::parse(&entry.data),
            2 => self.throttle = Throttle::parse(&entry.data),
            3 => self.imu = Imu::parse(&entry.data),
            4 => self.ride_mode = entry.data.first().copied(),
            5 => self.estimations = Estimations::parse(&entry.data),
            6 => self.totals = Totals::parse(&entry.data),
            7 => self.racing = Racing::parse(&entry.data),
            _ => {}
        }
    }

    fn decode_battery_tlv(&mut self, entry: &TlvEntry) {
        match entry.entry_type {
            2 => self.batt_signals = battery::BatterySignals::parse(&entry.data),
            3 => self.batt_cells = battery::BatteryCells::parse(&entry.data),
            // 4 => balancing (raw bytes, skipped)
            5 => self.batt_temps = battery::BatteryTemperatures::parse(&entry.data),
            6 => self.batt_params = battery::BatteryParams::parse(&entry.data),
            7 => self.batt_fw_version = battery::BatteryFirmwareVersion::parse(&entry.data),
            8 => self.batt_soc = battery::BatterySoc::parse(&entry.data),
            // 9 => BatteryInfo (not yet decoded)
            _ => {}
        }
    }

    fn decode_inverter_tlv(&mut self, entry: &TlvEntry) {
        match entry.entry_type {
            1 => self.inv_signals = inverter::InverterSignals::parse(&entry.data),
            2 => {
                // IGBT temperatures only — update igbt half
                if let Some(sensors) = inverter::TempSensors::parse(&entry.data) {
                    if let Some(ref mut t) = self.inv_temps {
                        t.igbt = sensors;
                    } else {
                        self.inv_temps = Some(inverter::InverterTemperatures {
                            igbt: sensors,
                            ..Default::default()
                        });
                    }
                }
            }
            3 => {
                // Motor temperatures only — update motor half
                if let Some(sensors) = inverter::TempSensors::parse(&entry.data) {
                    if let Some(ref mut t) = self.inv_temps {
                        t.motor = sensors;
                    } else {
                        self.inv_temps = Some(inverter::InverterTemperatures {
                            motor: sensors,
                            ..Default::default()
                        });
                    }
                }
            }
            4 => self.inv_pcb = inverter::InverterPcb::parse(&entry.data),
            5 => self.inv_info = inverter::InverterInfo::parse(&entry.data),
            _ => {}
        }
    }
}

/// Human-readable name for a characteristic UUID.
pub fn characteristic_name(uuid: Uuid) -> &'static str {
    protocol::characteristic_name(uuid).unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_bits_basic() {
        let mut data = [0u8; 18];
        data[0] = 0x03; // misc_bits = walking mode 3
        data[2] = 0x30; // indicator_bits = armed + drive
        let status = StatusBits::parse(&data).unwrap();
        assert_eq!(status.walking_mode(), 3);
        assert!(status.armed_throttle());
        assert!(status.drive());
        assert!(!status.is_charging());
    }

    #[test]
    fn parse_status_bits_too_short() {
        assert!(StatusBits::parse(&[0u8; 17]).is_none());
    }

    #[test]
    fn parse_identity() {
        let mut data = [0u8; 23];
        data[..17].copy_from_slice(b"UDUMX1AB2CD012345");
        data[17..23].copy_from_slice(b"240615");
        let id = BikeIdentity::parse(&data).unwrap();
        assert_eq!(id.vin, "UDUMX1AB2CD012345");
        assert_eq!(id.sold_date, "240615");
    }

    #[test]
    fn parse_battery_level_works() {
        assert_eq!(parse_battery_level(&[42]), Some(42));
        assert_eq!(parse_battery_level(&[]), None);
    }

    #[test]
    fn parse_speed() {
        // 350 = 35.0 km/h, 4500 RPM
        let data = [0x5E, 0x01, 0x94, 0x11];
        let s = Speed::parse(&data).unwrap();
        assert!((s.speed_kmh.unwrap() - 35.0).abs() < 0.1);
        assert_eq!(s.motor_rpm, Some(4500));
    }

    #[test]
    fn parse_speed_sentinel() {
        // 0xFFFF = no data
        let data = [0xFF, 0xFF, 0xFF, 0xFF];
        let s = Speed::parse(&data).unwrap();
        assert_eq!(s.speed_kmh, None);
        assert_eq!(s.motor_rpm, None);
    }

    #[test]
    fn parse_totals() {
        let mut data = [0u8; 16];
        data[0..4].copy_from_slice(&1234u32.to_le_bytes());
        data[4..8].copy_from_slice(&56789u32.to_le_bytes());
        let t = Totals::parse(&data).unwrap();
        assert_eq!(t.odometer, Some(1234));
        assert_eq!(t.watt_hours, Some(56789));
    }

    #[test]
    fn parse_estimations() {
        let mut data = [0u8; 6];
        data[0..2].copy_from_slice(&42u16.to_le_bytes());
        data[2..4].copy_from_slice(&90u16.to_le_bytes());
        data[4..6].copy_from_slice(&(-500i16).to_le_bytes());
        let e = Estimations::parse(&data).unwrap();
        assert_eq!(e.range_km, Some(42));
        assert_eq!(e.time_min, Some(90));
        assert_eq!(e.motor_power_w, Some(-500));
    }
}
