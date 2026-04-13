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

use std::collections::HashMap;
use std::time::Duration;

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use crate::crypto;
use crate::protocol;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no bluetooth adapter found")]
    NoAdapter,
    #[error("bike not found within {0:?}")]
    ScanTimeout(Duration),
    #[error("no Stark services found after discovery")]
    NoServicesFound,
    #[error("bluetooth error: {0}")]
    Ble(#[from] btleplug::Error),
}

/// A live connection to a Stark Varg with all discovered characteristics.
pub struct BikeConnection {
    peripheral: Peripheral,
    vin: String,
    pin: String,
    /// All discovered characteristics, keyed by UUID.
    chars: HashMap<Uuid, Characteristic>,
}

impl BikeConnection {
    /// The VIN this connection was established with.
    pub fn vin(&self) -> &str {
        &self.vin
    }

    /// The 6-digit pairing PIN for this bike.
    pub fn pin(&self) -> &str {
        &self.pin
    }

    /// The PIN as a numeric BLE SMP passkey (0–9999).
    pub fn pin_as_passkey(&self) -> u32 {
        crypto::pin_to_passkey(&self.pin)
    }

    /// The underlying btleplug peripheral.
    pub fn peripheral(&self) -> &Peripheral {
        &self.peripheral
    }

    /// Look up a characteristic by UUID. Returns `None` if the bike
    /// doesn't expose it (varies by firmware/hardware).
    pub fn characteristic(&self, uuid: Uuid) -> Option<&Characteristic> {
        self.chars.get(&uuid)
    }

    /// All discovered characteristics as a map.
    pub fn characteristics(&self) -> &HashMap<Uuid, Characteristic> {
        &self.chars
    }

    /// All discovered characteristic UUIDs, sorted for display.
    pub fn characteristic_uuids(&self) -> Vec<Uuid> {
        let mut uuids: Vec<Uuid> = self.chars.keys().copied().collect();
        uuids.sort();
        uuids
    }

    /// Disconnect from the bike.
    pub async fn disconnect(&self) -> Result<(), Error> {
        self.peripheral.disconnect().await?;
        Ok(())
    }
}

/// Scan for a Stark Varg, connect, discover ALL services and
/// characteristics, and return a [`BikeConnection`].
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

    // Collect ALL characteristics from ALL services.
    let all_chars = peripheral.characteristics();
    let chars: HashMap<Uuid, Characteristic> = all_chars.into_iter().map(|c| (c.uuid, c)).collect();

    if chars.is_empty() {
        return Err(Error::NoServicesFound);
    }

    Ok(BikeConnection {
        vin: vin.to_string(),
        pin,
        chars,
        peripheral,
    })
}

async fn find_bike(
    adapter: &btleplug::platform::Adapter,
    target_vin: &str,
) -> Result<Peripheral, Error> {
    loop {
        for p in adapter.peripherals().await? {
            if let Some(props) = p.properties().await? {
                let has_service = props.services.contains(&protocol::UUID_SVC_BIKE);
                let name = props.local_name.as_deref().unwrap_or("");

                if has_service || name.contains(target_vin) {
                    return Ok(p);
                }
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
}
