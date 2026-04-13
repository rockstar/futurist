# futurist

> "The futurist is here, gentlemen! He sees all. He knows what's best for you,
> whether you like it or not."
>
> -- Clint Barton, Captain America: Civil War

An open-source toolkit for the Stark Varg electric motorcycle. Connect to your
bike over Bluetooth, read live telemetry, and configure power modes — without
the official app, without an account, without a cloud service.

## Why

The Stark Varg is a capable electric motorcycle held back by a closed
ecosystem. Ride modes, power limits, and configuration are locked behind an
official app that requires an active account on Stark's backend servers. If the
company shuts down, changes their terms, or simply doesn't recognize your bike
as yours, you lose the ability to configure the machine you own.

Futurist exists so that owners can understand and control their own hardware
without depending on a cloud service that may not always be there.

## Features

### Live telemetry dashboard

A real-time GUI that displays speed, RPM, throttle, battery state, motor and
inverter temperatures, IMU data, ride mode, and more — streamed directly from
the bike over BLE.

```
futurist dash --vin YOUR_VIN
```

### Configuration editor

A graphical editor for the bike's power modes, power curves, and settings.
Adjust power and regen per map slot with sliders, shape RPM-based power curves
with presets or point-by-point editing, and change system timeouts. Writes
directly to the bike's flash.

```
futurist config-ui --vin YOUR_VIN
```

### CLI telemetry dump

Stream decoded telemetry to the terminal for logging, debugging, or piping to
other tools.

```
futurist dump --vin YOUR_VIN
```

### CLI config reader

Read all bike configuration (power modes, throttle curves, charger settings,
racing config, lock state) from the command line.

```
futurist config --vin YOUR_VIN
```

### Protocol documentation

A complete specification of the Stark Varg BLE protocol, covering all seven
GATT services, byte-level data formats, the pairing PIN derivation algorithm,
TLV multiplexing, VCU configuration read/write, and scaling factors.

See **[PROTOCOL.md](PROTOCOL.md)**.

## Supported platforms

The `futurist` binary runs on **macOS** and **Linux**. BLE pairing works
differently on each:

- **macOS**: pairing happens automatically via a system dialog. The tool
  displays the PIN to enter. Once paired, subsequent connections are seamless.
- **Linux (BlueZ)**: pair using `bluetoothctl` before running the tool. The
  pairing PIN is derived from the bike's VIN and can be computed with
  `futurist config --vin YOUR_VIN --dry-run`.

The GUI features (dashboard, config editor) use [egui](https://github.com/emilk/egui)
and work on both platforms.

## Building

Requires Rust 2024 edition (1.85+).

```
cargo build --release
```

To build without the GUI (CLI tools only):

```
cargo build --release --no-default-features
```

## Quick start

1. Find your bike's VIN (17 characters, printed on the frame).
2. Turn the bike on (key to ON position, not just charging).
3. If not yet paired, pair via your OS's Bluetooth settings using the PIN
   from `futurist dump --vin YOUR_VIN --dry-run`.
4. Run the dashboard:

```
futurist dash --vin YOUR_VIN
```

You can set `FUTURIST_VIN` as an environment variable to avoid typing the
VIN every time.

## Status

| Area | Status |
|------|--------|
| BLE pairing | Fully reverse-engineered and validated |
| Application-layer auth (V2) | Reverse-engineered; firmware-dependent (skipped on tested Gen 1) |
| Telemetry (7 GATT services) | All data characteristics decoded with proper units and scaling |
| TLV multiplexing | All seven services' TLV type IDs mapped and decoded |
| VCU configuration (read) | All 7 config types readable (maps, curves, racing, misc, charger, lock, totals) |
| VCU configuration (write) | Power modes, power curves, and misc settings writable |
| Live dashboard | Working (egui) |
| Configuration editor | Working (egui) — power modes, curves, settings |
| Wi-Fi firmware flashing | Identified but not decoded |

## Legal

This project is an independent research effort. It is not affiliated with,
endorsed by, or associated with Stark Future AB. All information was obtained
through analysis of publicly available software. The goal is interoperability
and owner empowerment, not circumvention of access controls for unauthorized
use.

## Contributing

If you own a Stark Varg and want to help, contributions are welcome. Areas
that could use attention:

- Testing on different firmware versions and hardware revisions
- Decoding the remaining config channel data formats
- Wi-Fi firmware flashing protocol
- Battery cell voltage calibration and BMS signal interpretation

See [PROTOCOL.md](PROTOCOL.md) for the full protocol specification.
