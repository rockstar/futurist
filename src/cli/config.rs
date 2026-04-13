use std::time::Duration;

use btleplug::api::{Peripheral as _, WriteType};
use futures::StreamExt;
use tokio::time::timeout;

use crate::protocol;

#[derive(clap::Args)]
pub struct ConfigArgs {
    /// VIN of the target bike.
    #[arg(long, env = "FUTURIST_VIN")]
    vin: String,

    /// Sold-on date (YYYYMMDD). Defaults to the epoch fallback.
    #[arg(long, env = "FUTURIST_SOLD_ON", default_value = protocol::SOLD_DATE_DEFAULT)]
    sold_on: String,

    /// Seconds to scan for the bike before giving up.
    #[arg(long, default_value_t = 30)]
    scan_timeout: u64,

    /// How many map slots to read (default: 5, matching the bike's capacity).
    #[arg(long, default_value_t = 5)]
    slots: u8,

    /// How many throttle curves to read (default: 5).
    #[arg(long, default_value_t = 5)]
    curves: u8,
}

/// VCU Config packet type IDs.
const TYPE_MAP_CONFIG: u8 = 0;
const TYPE_CURVES_CONFIG: u8 = 1;
const TYPE_RACING_CONFIG: u8 = 2;
const TYPE_MISC_CONFIG: u8 = 3;
const TYPE_CHARGER_CONFIG: u8 = 4;
const TYPE_LOCK_CONFIG: u8 = 5;
const TYPE_TOTALS_CONFIG: u8 = 7;

/// Parsed map configuration from the bike.
#[derive(Debug)]
struct MapConfig {
    slot: u8,
    torque_raw: i16,
    regen: i16,
    curve: u8,
}

impl MapConfig {
    fn parse(data: &[u8]) -> Option<Self> {
        // Response payload after the 3-byte VCU header: [slot, torque_lo, torque_hi, regen_lo, regen_hi, curve]
        if data.len() < 6 {
            return None;
        }
        Some(Self {
            slot: data[0],
            torque_raw: i16::from_le_bytes([data[1], data[2]]),
            regen: i16::from_le_bytes([data[3], data[4]]),
            curve: data[5],
        })
    }

    fn power_percent(&self) -> f32 {
        self.torque_raw as f32 / 1.25
    }
}

/// Parsed throttle curve from the bike.
/// 15 points mapping throttle input to torque and regen output.
#[derive(Debug)]
struct CurvesConfig {
    curve_index: u8,
    /// 15 torque output values (u16 each).
    torque: Vec<u16>,
    /// 15 regen output values (u16 each).
    regen: Vec<u16>,
}

impl CurvesConfig {
    fn parse(data: &[u8]) -> Option<Self> {
        // data[0] = curve index
        // Then 15 points, each 4 bytes: torque(u16 LE) + regen(u16 LE)
        // But the parsing starts from data[1] for the first torque value.
        if data.len() < 61 {
            return None;
        }
        let curve_index = data[0];
        let mut torque = Vec::with_capacity(15);
        let mut regen = Vec::with_capacity(15);
        for i in 0..15 {
            let off = i * 4;
            torque.push(u16::from_le_bytes([data[off + 1], data[off + 2]]));
            regen.push(u16::from_le_bytes([data[off + 3], data[off + 4]]));
        }
        Some(Self {
            curve_index,
            torque,
            regen,
        })
    }
}

/// Parsed racing configuration from the bike.
#[derive(Debug)]
struct RacingConfig {
    mode: u8,
    curve: u8,
    neutral_on: u8,
    neutral_off: u8,
    engage: u8,
    category: u8,
    expire_timestamp: u32,
}

impl RacingConfig {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 10 {
            return None;
        }
        Some(Self {
            mode: data[0],
            curve: data[1],
            neutral_on: data[2],
            neutral_off: data[3],
            engage: data[4],
            category: data[5],
            expire_timestamp: u32::from_le_bytes([data[6], data[7], data[8], data[9]]),
        })
    }
}

/// Parsed misc configuration from the bike.
#[derive(Debug)]
struct MiscConfig {
    /// Number of configured power mode slots.
    maps: u8,
    /// Inactivity timeout (likely seconds or minutes).
    inactive_timeout: u16,
    /// Auto power off timeout (likely seconds or minutes).
    auto_power_off: u16,
}

impl MiscConfig {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        Some(Self {
            maps: data[0],
            inactive_timeout: u16::from_le_bytes([data[1], data[2]]),
            auto_power_off: u16::from_le_bytes([data[3], data[4]]),
        })
    }
}

/// Parsed charger configuration from the bike.
#[derive(Debug)]
struct ChargerConfig {
    charge_current: i16,
    charge_power: i16,
    max_soc: i16,
    min_current: i16,
    start_time: i16,
    ramp_time: i16,
}

impl ChargerConfig {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        Some(Self {
            charge_current: i16::from_le_bytes([data[0], data[1]]),
            charge_power: i16::from_le_bytes([data[2], data[3]]),
            max_soc: i16::from_le_bytes([data[4], data[5]]),
            min_current: i16::from_le_bytes([data[6], data[7]]),
            start_time: i16::from_le_bytes([data[8], data[9]]),
            ramp_time: i16::from_le_bytes([data[10], data[11]]),
        })
    }
}

/// Parsed lock configuration from the bike.
#[derive(Debug)]
struct LockBikeConfig {
    lock_status: u8,
    lock_type: u8,
    lock_timeout: u16,
}

impl LockBikeConfig {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        Some(Self {
            lock_status: data[0],
            lock_type: data[1],
            lock_timeout: u16::from_le_bytes([data[2], data[3]]),
        })
    }
}

/// Parsed totals configuration from the bike.
/// Variable length depending on which flags were requested.
#[derive(Debug)]
struct TotalsConfig {
    flags: u16,
    /// Odometer in meters (present if flags & 1).
    odometer_m: Option<u32>,
    /// Total ride time in seconds (present if flags & 2).
    total_ride_time_s: Option<u32>,
}

impl TotalsConfig {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let flags = u16::from_le_bytes([data[0], data[1]]);
        let mut offset = 2;

        let odometer_m = if flags & 1 != 0 && offset + 4 <= data.len() {
            let v = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            Some(v)
        } else {
            None
        };

        let total_ride_time_s = if flags & 2 != 0 && offset + 4 <= data.len() {
            let v = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            Some(v)
        } else {
            None
        };

        Some(Self {
            flags,
            odometer_m,
            total_ride_time_s,
        })
    }
}

/// Send a config read request and wait for the matching response.
///
/// Filters notifications to match both `readWrite=2` (response) AND the
/// expected config type byte. This prevents consuming stale responses
/// from other config types that arrive out of order.
async fn read_config(
    peripheral: &btleplug::platform::Peripheral,
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    vcu_config_char: &btleplug::api::Characteristic,
    request: &[u8],
    expected_type: u8,
    label: &str,
) -> anyhow::Result<Vec<u8>> {
    use btleplug::api::Peripheral as _;

    peripheral
        .write(vcu_config_char, request, WriteType::WithResponse)
        .await?;

    let response = timeout(Duration::from_secs(5), async {
        while let Some(notif) = notifications.next().await {
            if notif.uuid == protocol::UUID_VCU_CONFIG
                && notif.value.len() >= 2
                && notif.value[0] == 2
                && notif.value[1] == expected_type
            {
                return Some(notif.value);
            }
        }
        None
    })
    .await
    .map_err(|_| anyhow::anyhow!("timed out waiting for {} response", label))?
    .ok_or_else(|| anyhow::anyhow!("notification stream ended"))?;

    // Return just the data after the 3-byte header [readWrite, type, status].
    Ok(response[3..].to_vec())
}

pub async fn run(args: ConfigArgs) -> anyhow::Result<()> {
    let pin = crate::crypto::generate_pin(&args.vin, &args.sold_on);
    eprintln!("VIN:  {}", args.vin);
    eprintln!("PIN:  {} (enter this if a pairing dialog appears)", pin);
    eprintln!();

    eprintln!("scanning for bike...");
    let bike = crate::ble::scan_and_connect(
        &args.vin,
        &args.sold_on,
        Duration::from_secs(args.scan_timeout),
    )
    .await?;
    eprintln!("connected to {}", bike.vin());

    let peripheral = bike.peripheral();

    // We need the VCU Config characteristic (0x4005) for read requests.
    let vcu_config_char = bike
        .characteristic(protocol::UUID_VCU_CONFIG)
        .ok_or_else(|| anyhow::anyhow!("VCU Config characteristic (0x4005) not found"))?
        .clone();

    // Subscribe to notifications on it — responses come back here.
    peripheral.subscribe(&vcu_config_char).await?;
    let mut notifications = peripheral.notifications().await?;

    // --- Read map configs ---
    println!("Reading map configurations...\n");
    for slot in 0..args.slots {
        let data = read_config(
            peripheral,
            &mut notifications,
            &vcu_config_char,
            &[0x00, TYPE_MAP_CONFIG, slot],
            TYPE_MAP_CONFIG,
            &format!("map slot {}", slot),
        )
        .await?;
        match MapConfig::parse(&data) {
            Some(cfg) => {
                println!("  Map slot {}:", cfg.slot);
                println!(
                    "    Power:    {:.0}%  (torque raw: {})",
                    cfg.power_percent(),
                    cfg.torque_raw
                );
                println!("    Regen:    {}", cfg.regen);
                println!("    Curve:    {}", cfg.curve);
                println!();
            }
            None => println!(
                "  Slot {}: could not parse ({} bytes): {}",
                slot,
                data.len(),
                hex::encode(&data)
            ),
        }
    }

    // --- Read throttle curves ---
    println!("Reading throttle curves...\n");
    for curve_idx in 0..args.curves {
        let data = read_config(
            peripheral,
            &mut notifications,
            &vcu_config_char,
            &[0x00, TYPE_CURVES_CONFIG, curve_idx],
            TYPE_CURVES_CONFIG,
            &format!("curve {}", curve_idx),
        )
        .await?;
        if data.is_empty() {
            println!("  Curve {}: (built-in default, not readable)\n", curve_idx);
        } else {
            match CurvesConfig::parse(&data) {
                Some(cfg) => {
                    println!("  Curve {}:", cfg.curve_index);
                    println!("    Torque points: {:?}", cfg.torque);
                    println!("    Regen points:  {:?}", cfg.regen);
                    println!();
                }
                None => println!(
                    "  Curve {}: could not parse ({} bytes): {}",
                    curve_idx,
                    data.len(),
                    hex::encode(&data)
                ),
            }
        }
    }

    // --- Read racing config ---
    println!("Reading racing config...\n");
    let data = read_config(
        peripheral,
        &mut notifications,
        &vcu_config_char,
        &[0x00, TYPE_RACING_CONFIG],
        TYPE_RACING_CONFIG,
        "racing config",
    )
    .await?;
    match RacingConfig::parse(&data) {
        Some(cfg) => {
            println!("  Mode:          {}", cfg.mode);
            println!("  Curve:         {}", cfg.curve);
            println!("  Neutral on:    {}", cfg.neutral_on);
            println!("  Neutral off:   {}", cfg.neutral_off);
            println!("  Engage:        {}", cfg.engage);
            println!("  Category:      {}", cfg.category);
            println!("  Expire:        {}", cfg.expire_timestamp);
            println!();
        }
        None => println!(
            "  Could not parse ({} bytes): {}",
            data.len(),
            hex::encode(&data)
        ),
    }

    // --- Read misc config ---
    println!("Reading misc config...\n");
    let data = read_config(
        peripheral,
        &mut notifications,
        &vcu_config_char,
        &[0x00, TYPE_MISC_CONFIG],
        TYPE_MISC_CONFIG,
        "misc config",
    )
    .await?;
    match MiscConfig::parse(&data) {
        Some(cfg) => {
            println!("  Maps configured:   {}", cfg.maps);
            println!("  Inactive timeout:  {}", cfg.inactive_timeout);
            println!("  Auto power off:    {}", cfg.auto_power_off);
            println!();
        }
        None => println!(
            "  Could not parse ({} bytes): {}",
            data.len(),
            hex::encode(&data)
        ),
    }

    // --- Read charger config ---
    println!("Reading charger config...\n");
    let data = read_config(
        peripheral,
        &mut notifications,
        &vcu_config_char,
        &[0x00, TYPE_CHARGER_CONFIG],
        TYPE_CHARGER_CONFIG,
        "charger config",
    )
    .await?;
    match ChargerConfig::parse(&data) {
        Some(cfg) => {
            println!("  Charge current:  {}", cfg.charge_current);
            println!("  Charge power:    {}", cfg.charge_power);
            println!("  Max SOC:         {}", cfg.max_soc);
            println!("  Min current:     {}", cfg.min_current);
            println!("  Start time:      {}", cfg.start_time);
            println!("  Ramp time:       {}", cfg.ramp_time);
            println!();
        }
        None => println!(
            "  Could not parse ({} bytes): {}",
            data.len(),
            hex::encode(&data)
        ),
    }

    // --- Read lock config ---
    println!("Reading lock config...\n");
    let data = read_config(
        peripheral,
        &mut notifications,
        &vcu_config_char,
        &[0x00, TYPE_LOCK_CONFIG],
        TYPE_LOCK_CONFIG,
        "lock config",
    )
    .await?;
    match LockBikeConfig::parse(&data) {
        Some(cfg) => {
            println!("  Lock status:   {}", cfg.lock_status);
            println!("  Lock type:     {}", cfg.lock_type);
            println!("  Lock timeout:  {}", cfg.lock_timeout);
            println!();
        }
        None => println!(
            "  Could not parse ({} bytes): {}",
            data.len(),
            hex::encode(&data)
        ),
    }

    // --- Read totals config ---
    println!("Reading totals config...\n");
    let flags: u16 = 3; // bit 0 = odometer, bit 1 = total ride time
    let data = read_config(
        peripheral,
        &mut notifications,
        &vcu_config_char,
        &[0x00, TYPE_TOTALS_CONFIG, flags as u8, (flags >> 8) as u8],
        TYPE_TOTALS_CONFIG,
        "totals config",
    )
    .await?;
    if data.is_empty() {
        println!("  (not supported on this firmware — use live Totals notification instead)");
        println!();
    } else {
        match TotalsConfig::parse(&data) {
            Some(cfg) => {
                println!("  Flags: 0x{:04x}", cfg.flags);
                if let Some(odo) = cfg.odometer_m {
                    println!(
                        "  Odometer:         {} m ({:.1} km)",
                        odo,
                        odo as f64 / 1000.0
                    );
                }
                if let Some(time) = cfg.total_ride_time_s {
                    let hours = time / 3600;
                    let mins = (time % 3600) / 60;
                    println!("  Total ride time:  {} s ({}h {}m)", time, hours, mins);
                }
                println!();
            }
            None => println!(
                "  Could not parse ({} bytes): {}",
                data.len(),
                hex::encode(&data)
            ),
        }
    }

    bike.disconnect().await?;
    Ok(())
}
