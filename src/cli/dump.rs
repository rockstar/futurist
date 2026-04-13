use std::time::{Duration, Instant};

use futures::StreamExt;

use crate::protocol;
use crate::telemetry::{self, DecodedTelemetry, StatusBits, characteristic_name};

#[derive(clap::Args)]
pub struct DumpArgs {
    /// VIN of the target bike.
    #[arg(long, env = "FUTURIST_VIN")]
    vin: String,

    /// Sold-on date (YYYYMMDD). Defaults to the epoch fallback.
    #[arg(long, env = "FUTURIST_SOLD_ON", default_value = protocol::SOLD_DATE_DEFAULT)]
    sold_on: String,

    /// Seconds to scan for the bike before giving up.
    #[arg(long, default_value_t = 30)]
    scan_timeout: u64,

    /// Output raw hex instead of decoded fields.
    #[arg(long, default_value_t = false)]
    no_decode: bool,
}

pub async fn run(args: DumpArgs) -> anyhow::Result<()> {
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
    eprintln!(
        "{} characteristics discovered",
        bike.characteristics().len()
    );

    eprintln!("subscribing to telemetry...");
    let mut stream = telemetry::subscribe(&bike).await?;
    eprintln!("streaming (Ctrl+C to stop):\n");

    let start = Instant::now();
    let mut decoded = DecodedTelemetry::default();

    while let Some(frame) = stream.next().await {
        let elapsed = frame.timestamp.duration_since(start);
        let name = characteristic_name(frame.characteristic);

        if args.no_decode {
            println!(
                "{:>8.3}s  {:12}  ({:>3} bytes)  {}",
                elapsed.as_secs_f64(),
                name,
                frame.data.len(),
                hex::encode(&frame.data),
            );
        } else {
            decoded.update(&frame);
            print_decoded(elapsed.as_secs_f64(), name, &frame, &decoded);
        }
    }

    eprintln!("stream ended (bike disconnected?)");
    Ok(())
}

fn print_decoded(
    elapsed: f64,
    name: &str,
    frame: &telemetry::TelemetryFrame,
    state: &DecodedTelemetry,
) {
    match frame.characteristic {
        protocol::UUID_STATUS_BITS => {
            if let Some(ref s) = state.status {
                println!(
                    "{:>8.3}s  {}  drive={} armed={} charging={} charger={} \
                     fan={} pump={} walking={} battery_status=0x{:08x}",
                    elapsed,
                    name,
                    s.drive(),
                    s.armed_throttle(),
                    s.is_charging(),
                    s.charger_connected(),
                    s.fan_on(),
                    s.pump_on(),
                    s.walking_mode(),
                    s.battery_status,
                );
                print_faults(s);
            }
        }
        protocol::UUID_IDENTITY => {
            if let Some(ref id) = state.identity {
                println!(
                    "{:>8.3}s  {}  vin={} sold_date={}",
                    elapsed, name, id.vin, id.sold_date,
                );
            }
        }
        protocol::UUID_BATTERY_LEVEL => {
            if let Some(pct) = state.battery_percent {
                println!("{:>8.3}s  {}  {}%", elapsed, name, pct);
            }
        }
        protocol::UUID_SPEED => {
            if let Some(ref s) = state.speed {
                println!(
                    "{:>8.3}s  {}  {} km/h  {} RPM",
                    elapsed,
                    name,
                    fmt_f32(s.speed_kmh, 1),
                    fmt_u16(s.motor_rpm),
                );
            }
        }
        protocol::UUID_THROTTLE => {
            if let Some(ref t) = state.throttle {
                println!(
                    "{:>8.3}s  {}  pos={} iq={} id={}",
                    elapsed,
                    name,
                    fmt_u16(t.position),
                    fmt_i16(t.iq_fb),
                    fmt_i16(t.id_fb),
                );
            }
        }
        protocol::UUID_IMU => {
            if let Some(ref i) = state.imu {
                println!(
                    "{:>8.3}s  {}  accel=({},{},{}) gyro=({},{},{})",
                    elapsed,
                    name,
                    fmt_i16(i.accel_x),
                    fmt_i16(i.accel_y),
                    fmt_i16(i.accel_z),
                    fmt_i16(i.gyro_x),
                    fmt_i16(i.gyro_y),
                    fmt_i16(i.gyro_z),
                );
            }
        }
        protocol::UUID_MAPS => {
            if let Some(mode) = state.ride_mode {
                println!(
                    "{:>8.3}s  {}  map {} (active power mode slot — names are user-configured)",
                    elapsed, name, mode,
                );
            }
        }
        protocol::UUID_TOTALS => {
            if let Some(ref t) = state.totals {
                println!(
                    "{:>8.3}s  {}  odo={} wh={} airtime={}s total_time={}s",
                    elapsed,
                    name,
                    fmt_u32(t.odometer),
                    fmt_u32(t.watt_hours),
                    fmt_u32(t.airtime_secs),
                    fmt_u32(t.total_time_secs),
                );
            }
        }
        protocol::UUID_ESTIMATIONS => {
            if let Some(ref e) = state.estimations {
                println!(
                    "{:>8.3}s  {}  range={}km time={}min power={}W",
                    elapsed,
                    name,
                    fmt_u16(e.range_km),
                    fmt_u16(e.time_min),
                    fmt_i16(e.motor_power_w),
                );
            }
        }
        protocol::UUID_RACING => {
            if let Some(ref r) = state.racing {
                println!(
                    "{:>8.3}s  {}  mode={} curve={} throttle_mult={} cat={} expire={}",
                    elapsed,
                    name,
                    r.mode,
                    r.curve,
                    r.throttle_multiplier,
                    r.category,
                    r.expire_timestamp,
                );
            }
        }
        protocol::UUID_VERSIONS => {
            if let Some(ref v) = state.versions {
                println!(
                    "{:>8.3}s  {}  ble_ver={} dl={}% blob_fs={} blob_srv={}",
                    elapsed, name, v.ble_version, v.download_percent, v.blob_fs, v.blob_server,
                );
                for c in &v.components {
                    println!(
                        "           {:20} ver={:10} avail={}",
                        c.name, c.version, c.available,
                    );
                }
            }
        }
        protocol::UUID_EXTENDED_TLV => {
            if state.tlv_entries.is_empty() {
                println!("{:>8.3}s  {}  (empty)", elapsed, name);
            } else {
                for entry in &state.tlv_entries {
                    print_tlv_entry(elapsed, name, entry);
                }
            }
        }
        _ => {
            println!(
                "{:>8.3}s  {:12}  UNKNOWN ({:>3} bytes)  {}",
                elapsed,
                name,
                frame.data.len(),
                hex::encode(&frame.data),
            );
        }
    }
}

fn print_faults(s: &StatusBits) {
    if s.fault_bits == 0 {
        return;
    }
    let mut faults = Vec::new();
    if s.fault_bits & 0x01 != 0 {
        faults.push("general");
    }
    if s.fault_bits & 0x02 != 0 {
        faults.push("battery_temp");
    }
    if s.fault_bits & 0x04 != 0 {
        faults.push("motor_temp");
    }
    if s.fault_bits & 0x08 != 0 {
        faults.push("coolant_pump");
    }
    if s.fault_bits & 0x10 != 0 {
        faults.push("cooling_fan");
    }
    if s.fault_bits & 0x20 != 0 {
        faults.push("insulation");
    }
    if s.fault_bits & 0x40 != 0 {
        faults.push("derating_battery");
    }
    if s.fault_bits & 0x80 != 0 {
        faults.push("derating_igbt");
    }
    eprintln!("  FAULTS: {}", faults.join(", "));
}

fn print_tlv_entry(elapsed: f64, name: &str, entry: &telemetry::TlvEntry) {
    match entry.entry_type {
        // Type 1: FAST_BITS — compact status update (8 bytes)
        1 if entry.data.len() >= 8 => {
            let misc = u16::from_le_bytes([entry.data[0], entry.data[1]]);
            let indicators = u16::from_le_bytes([entry.data[2], entry.data[3]]);
            let alerts = u16::from_le_bytes([entry.data[4], entry.data[5]]);
            let info = u16::from_le_bytes([entry.data[6], entry.data[7]]);

            let armed = (indicators & 0x10) != 0;
            let drive = (indicators & 0x20) != 0;
            let charger = (info & 0x01) != 0;
            let charging = (info & 0x02) != 0;
            let pump = (info & 0x20) != 0;
            let fan = (info & 0x40) != 0;

            println!(
                "{:>8.3}s  {}  fast_bits: drive={drive} armed={armed} \
                 charging={charging} charger={charger} fan={fan} pump={pump} \
                 walking={} alerts=0x{alerts:04x} misc=0x{misc:04x}",
                elapsed,
                name,
                misc & 0x0F,
            );
        }
        // Type 2: LOCK_STATUS (3 bytes)
        2 if entry.data.len() >= 3 => {
            let status = entry.data[0];
            let time = u16::from_le_bytes([entry.data[1], entry.data[2]]);
            println!(
                "{:>8.3}s  {}  lock_status: locked={} time={}",
                elapsed,
                name,
                status != 0,
                time,
            );
        }
        // Type 3: UPDATE_AVAILABLE (1 byte)
        3 if !entry.data.is_empty() => {
            println!(
                "{:>8.3}s  {}  update_available: {}",
                elapsed,
                name,
                entry.data[0] == 1,
            );
        }
        _ => {
            println!(
                "{:>8.3}s  {}  tlv_type={} ({} bytes): {}",
                elapsed,
                name,
                entry.entry_type,
                entry.data.len(),
                hex::encode(&entry.data),
            );
        }
    }
}

// -- Formatting helpers for Option fields --

fn fmt_u16(v: Option<u16>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "-".to_string(),
    }
}

fn fmt_i16(v: Option<i16>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "-".to_string(),
    }
}

fn fmt_u32(v: Option<u32>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "-".to_string(),
    }
}

fn fmt_f32(v: Option<f32>, decimals: usize) -> String {
    match v {
        Some(n) => format!("{n:.decimals$}"),
        None => "-".to_string(),
    }
}
