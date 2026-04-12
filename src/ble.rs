//! BLE scanning, connection, and characteristic access for the Stark Varg.
//!
//! # Pairing
//!
//! The Stark Varg requires BLE SMP Passkey Entry pairing before GATT
//! characteristics are accessible. The passkey is derived from the bike's
//! VIN and sold-on date (see [`crate::crypto::generate_pin`]).
//!
//! How pairing is triggered depends on the OS:
//!
//! - **macOS:** CoreBluetooth shows a system PIN dialog automatically when
//!   the connection accesses an encrypted characteristic. The user must type
//!   the PIN manually. Call [`BikeConnection::pin`] to get the value to enter.
//!
//! - **Linux (BlueZ):** Register an `org.bluez.Agent1` with a `RequestPasskey`
//!   handler before connecting. BlueZ calls the agent during `Device1.Pair()`.
//!   This is not handled by btleplug — use `bluetoothctl` or the `bluer` crate.
//!
//! On both platforms, once paired, subsequent connections reuse the stored
//! bond and no PIN dialog appears.

use std::time::Duration;

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use tokio::time::{sleep, timeout};

use crate::crypto;
use crate::protocol;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no bluetooth adapter found")]
    NoAdapter,
    #[error("bike not found within {0:?}")]
    ScanTimeout(Duration),
    #[error("bike service not found after discovery")]
    ServiceNotFound,
    #[error("bluetooth error: {0}")]
    Ble(#[from] btleplug::Error),
}

/// A live connection to a Stark Varg with resolved characteristics.
pub struct BikeConnection {
    peripheral: Peripheral,
    vin: String,
    pin: String,
    pub security: Option<Characteristic>,
    pub bike_data: Option<Characteristic>,
    pub bike_data_2: Option<Characteristic>,
    pub live_data: Option<Characteristic>,
    pub command: Option<Characteristic>,
    pub vcu_data: Option<Characteristic>,
    pub battery_level: Option<Characteristic>,
}

impl BikeConnection {
    /// The VIN this connection was established with.
    pub fn vin(&self) -> &str {
        &self.vin
    }

    /// The 6-digit pairing PIN for this bike.
    ///
    /// On macOS, display this to the user before the system pairing dialog
    /// appears. On Linux, supply [`pin_as_passkey`](Self::pin_as_passkey)
    /// to the BlueZ agent's `RequestPasskey` handler.
    pub fn pin(&self) -> &str {
        &self.pin
    }

    /// The PIN as a numeric BLE SMP passkey (0–9999).
    pub fn pin_as_passkey(&self) -> u32 {
        crypto::pin_to_passkey(&self.pin)
    }

    /// The underlying btleplug peripheral, for operations not wrapped here.
    pub fn peripheral(&self) -> &Peripheral {
        &self.peripheral
    }

    /// Disconnect from the bike.
    pub async fn disconnect(&self) -> Result<(), Error> {
        self.peripheral.disconnect().await?;
        Ok(())
    }
}

/// Scan for a Stark Varg, connect, discover services, and return a
/// [`BikeConnection`] with all known characteristics resolved.
///
/// The `vin` is used both to match the bike's advertised name and to
/// derive the pairing PIN. The `sold_on` date is used for PIN derivation
/// (pass `"19700101"` for the default/epoch fallback).
///
/// The bike must be keyed on (not just charging) for the GATT server to
/// be active.
///
/// **On macOS**, the first connection to an unpaired bike will trigger a
/// system PIN dialog. Call [`BikeConnection::pin`] on the returned value
/// to get the PIN the user needs to enter — but note the dialog may appear
/// *during* this call (specifically during service discovery). For a better
/// UX, compute the PIN beforehand with [`crate::crypto::generate_pin`] and
/// display it before calling this function.
pub async fn scan_and_connect(
    vin: &str,
    sold_on: &str,
    scan_duration: Duration,
) -> Result<BikeConnection, Error> {
    let sold_on = crypto::normalize_sold_on(sold_on);
    let pin = crypto::generate_pin(vin, &sold_on);

    let manager = Manager::new().await?;
    let adapter = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or(Error::NoAdapter)?;

    adapter.start_scan(ScanFilter::default()).await?;

    let peripheral = timeout(scan_duration, find_bike(&adapter, vin))
        .await
        .map_err(|_| Error::ScanTimeout(scan_duration))??;

    adapter.stop_scan().await.ok();

    peripheral.connect().await?;
    peripheral.discover_services().await?;

    let chars = peripheral.characteristics();
    let find = |uuid| chars.iter().find(|c| c.uuid == uuid).cloned();

    let conn = BikeConnection {
        vin: vin.to_string(),
        pin,
        security: find(protocol::UUID_SECURITY),
        bike_data: find(protocol::UUID_BIKE_DATA),
        bike_data_2: find(protocol::UUID_BIKE_DATA_2),
        live_data: find(protocol::UUID_LIVE_DATA),
        command: find(protocol::UUID_COMMAND),
        vcu_data: find(protocol::UUID_VCU_DATA),
        battery_level: find(protocol::UUID_BATTERY_LEVEL),
        peripheral,
    };

    if conn.security.is_none() && conn.live_data.is_none() {
        return Err(Error::ServiceNotFound);
    }

    Ok(conn)
}

async fn find_bike(
    adapter: &btleplug::platform::Adapter,
    target_vin: &str,
) -> Result<Peripheral, Error> {
    loop {
        for p in adapter.peripherals().await? {
            if let Some(props) = p.properties().await? {
                let has_service = props.services.contains(&protocol::UUID_BIKE_SERVICE);
                let name = props.local_name.as_deref().unwrap_or("");

                if has_service || name.contains(target_vin) {
                    return Ok(p);
                }
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
}
