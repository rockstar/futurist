# Stark Varg BLE Protocol

This document describes the Bluetooth Low Energy protocol used by the Stark
Varg motorcycle. It is intended as a complete reference for implementing a
third-party client without the official app or Stark's backend services.

Validated against a Gen 1 Stark Varg.

---

## Table of Contents

- [Overview](#overview)
- [BLE Advertising](#ble-advertising)
- [GATT Services](#gatt-services)
- [Pairing](#pairing)
  - [PIN Derivation Algorithm](#pin-derivation-algorithm)
  - [Pairing Procedure](#pairing-procedure)
  - [Sold-On Date](#sold-on-date)
- [Application-Layer Authentication (V2)](#application-layer-authentication-v2)
  - [When It Applies](#when-it-applies)
  - [Intermediate Key Derivation](#intermediate-key-derivation)
  - [Challenge-Response Handshake](#challenge-response-handshake)
- [Decoded Data Types](#decoded-data-types)
- [Battery Data Types](#battery-data-types)
- [Inverter Data Types](#inverter-data-types)
- [Charger Data](#charger-data-0x5001--18-bytes)
- [Docking Data](#docking-data)
- [VCU Data Types](#vcu-data-types)
- [VCU Configuration](#vcu-configuration)
- [TLV Format](#tlv-format)
- [Data Redundancy](#data-redundancy)
- [Scaling Factors and Units](#scaling-factors-and-units)
- [Firmware Variants](#firmware-variants)
- [Wi-Fi Firmware Flashing](#wi-fi-firmware-flashing)

---

## Overview

The Stark Varg communicates over Bluetooth Low Energy (BLE). The bike is a
dual-mode Bluetooth device — it has both a BLE GATT server (for telemetry and
control) and a Bluetooth Classic SPP profile for legacy data transfer.

Normal operation uses BLE exclusively. The connection flow is:

1. **Scan** for the bike's BLE advertisement.
2. **Pair** using a VIN-derived PIN via BLE SMP Passkey Entry.
3. **Optionally authenticate** at the application layer via a challenge-response
   handshake on the security characteristic (firmware-dependent; some versions
   skip this).
4. **Subscribe** to notify-capable characteristics across all services.
5. **Read/write** characteristics for telemetry and configuration.

---

## BLE Advertising

The bike advertises under two modes depending on its power state:

| State | Advertised Name | Service UUID in Advertisement | GATT Server Active |
|-------|----------------|-------------------------------|-------------------|
| **Charging (key off)** | VIN (e.g. `UDUMX1AB2CD012345`) | No | No |
| **Key on** | VIN (e.g. `UDUMX1AB2CD012345`) | Yes | Yes |

The GATT server and its characteristics are only available when the bike is
keyed on. Scanning for the service UUID is the reliable way to detect a
connectable bike.

The bike has a short idle timeout and will power off BLE after a period of
inactivity with no active connection. The exact timeout is not known but is
on the order of minutes.

---

## GATT Services

All custom UUIDs share a common base derived from "StarK Future" in ASCII:

```
Base: XXXXXXXX-5374-6172-4b20-467574757265
              S t a r K   F u t u r e
```

The bike exposes up to seven GATT services. Not all may be present on every
firmware version. Clients should discover all services dynamically and
subscribe to every notify-capable characteristic.

### Service 0x1000 — Bike Data

Primary bike state, identity, firmware versions, and authentication.

| UUID | Name | Properties | Description |
|------|------|-----------|-------------|
| `0x1001` | Security | W, N | Auth handshake (see [V2 Auth](#application-layer-authentication-v2)). |
| `0x1002` | Status Bits | R, N | 18 bytes — bike state flags. See [Status Bits](#status-bits-0x1002--18-bytes). |
| `0x1003` | Identity | R, N | 23 bytes — VIN (17 chars) + sold date (6 chars). |
| `0x1005` | Versions | R, N | 100 bytes — BLE version, firmware versions for all components. |
| `0x1006` | Command | W | Write-only command channel. |
| `0x1100` | Extended TLV | R, N | [TLV-encoded](#tlv-format) status updates (fast bits, lock, update available). |
| `0x1101` | Extended Config | R, N | Extended configuration data. |

### Service 0x2000 — Live Data

Real-time riding telemetry. Updated at high frequency while the bike is active.

| UUID | Name | Properties | Description |
|------|------|-----------|-------------|
| `0x2001` | Speed | R, N | 4 bytes. See [Speed](#speed-0x2001--4-bytes). |
| `0x2002` | Throttle | R, N | 6 bytes. See [Throttle](#throttle-0x2002--6-bytes). |
| `0x2003` | IMU | R, N | 12 bytes. See [IMU](#imu-0x2003--12-bytes). |
| `0x2004` | Maps | R, N | 1 byte — active power mode slot number. |
| `0x2005` | Totals | R, N | 16 bytes. See [Totals](#totals-0x2005--16-bytes). |
| `0x2006` | Estimations | R, N | 6 bytes. See [Estimations](#estimations-0x2006--6-bytes). |
| `0x2007` | Racing | R, N | 9 bytes. See [Racing](#racing-0x2007--9-bytes). |
| `0x2008` | Config | R, N | Live data configuration. |
| `0x2100` | Live TLV | R, N | [TLV-encoded](#tlv-format) multiplexed live data. |
| `0x2101` | Live Ext Config | R, N | Extended live configuration. |

### Service 0x3000 — Docking

Docking station data (for the Stark charging dock).

| UUID | Name | Properties | Description |
|------|------|-----------|-------------|
| `0x3001` | Docking Data 1 | R, N | Docking state. |
| `0x3002` | Docking Data 2 | R, N | Additional docking data. |
| `0x3100` | Docking TLV | R, N | TLV-encoded docking data. |
| `0x3101` | Docking Config | R, N | Docking configuration. |

### Service 0x4000 — VCU

Vehicle Control Unit data, versions, and configuration.

| UUID | Name | Properties | Description |
|------|------|-----------|-------------|
| `0x4001` | VCU Versions | R, N | VCU firmware version info. |
| `0x4002` | VCU Info | R, N | VCU status information. |
| `0x4005` | VCU Config | R, W, N | Read/write bike configuration. See [VCU Configuration](#vcu-configuration). |
| `0x4100` | VCU TLV | R, N | TLV-encoded VCU data. |

### Service 0x5000 — Charger

Charger status and configuration.

| UUID | Name | Properties | Description |
|------|------|-----------|-------------|
| `0x5001` | Charger Data | R, N | Charger state. |
| `0x5100` | Charger TLV | R, N | TLV-encoded charger data. |
| `0x5101` | Charger Config | R, N | Charger configuration. |

### Service 0x6000 — Battery

Detailed battery management system data.

| UUID | Name | Properties | Description |
|------|------|-----------|-------------|
| `0x6001`–`0x6009` | Battery Data 1–9 | R, N | Nine battery data characteristics (cell voltages, temperatures, BMS signals, etc.). |
| `0x6100` | Battery TLV | R, N | TLV-encoded battery data. |
| `0x6101` | Battery Config | R, N | Battery configuration. |

### Service 0x7000 — Inverter

Motor controller (inverter) data.

| UUID | Name | Properties | Description |
|------|------|-----------|-------------|
| `0x7001`–`0x7004` | Inverter Data 1–4 | R, N | Inverter telemetry (temperatures, currents, etc.). |
| `0x7100` | Inverter TLV | R, N | TLV-encoded inverter data. |
| `0x7101` | Inverter Config | R, N | Inverter configuration. |

### Standard BLE

| UUID | Name | Properties | Description |
|------|------|-----------|-------------|
| `0x2a19` | Battery Level | R, N | Standard BLE characteristic. Single byte, 0–100 (percentage). |

### Notifications (CCCD)

To receive live updates, enable notifications by writing `0x01 0x00` to the
Client Characteristic Configuration Descriptor (UUID `0x2902`) on the desired
characteristic. This is standard BLE and is handled automatically by most
libraries (`subscribe()` / `notify()`).

Clients should subscribe to **all** notify-capable characteristics across all
discovered services.

---

## Pairing

The bike uses BLE SMP (Security Manager Protocol) Passkey Entry for pairing.
The passkey is a 4-digit number (0000-9999) derived deterministically from the
bike's VIN and a "sold-on" date.

### PIN Derivation Algorithm

**Input:** A string of the form `"VIN-SOLDDATE"`, where:
- `VIN` is the bike's 17-character Vehicle Identification Number (uppercase ASCII).
- `SOLDDATE` is an 8-digit date string in `YYYYMMDD` format.

**Algorithm:**

```
hash = SHA-256("VIN-SOLDDATE")    // e.g. SHA-256("UDUMX1AB2CD012345-19700101")

d1 = ((hash[15] ^ 0xB3) as u32 + 0xEF) % 10
d2 = ((hash[ 9] ^ 0x9E) as u32 + 0x64) % 10
d3 = ((hash[14] ^ 0xEC) as u32 + 0xD1) % 10
d4 = ((hash[ 3] ^ 0xC5) as u32 + 0xE4) % 10

pin = d1 * 1000 + d2 * 100 + d3 * 10 + d4
```

**Critical implementation note:** The XOR produces a byte (0-255), but the
addition of the constant MUST be performed in 32-bit (or wider) arithmetic
before the modulo. Performing the addition in 8-bit arithmetic with wrapping
produces incorrect PINs for most inputs.

**Output:** An integer in the range 0-9999. The BLE SMP passkey is this
numeric value directly.

**Byte index and constant table:**

| Digit | Hash Index | XOR Mask | Add Constant |
|-------|-----------|----------|-------------|
| d1 (thousands) | 15 | 0xB3 | 0xEF |
| d2 (hundreds) | 9 | 0x9E | 0x64 |
| d3 (tens) | 14 | 0xEC | 0xD1 |
| d4 (units) | 3 | 0xC5 | 0xE4 |

### Pairing Procedure

**On Linux (BlueZ):**

1. Register an `org.bluez.Agent1` with a `RequestPasskey` handler that returns
   the computed PIN as a `u32`.
2. Connect to the bike via BLE (`Device1.Connect()`).
3. Pair over the active LE link (`Device1.Pair()`). BlueZ will call the agent's
   `RequestPasskey` method.
4. After successful pairing, GATT services become available.

**Important:** Call `Connect()` before `Pair()`. If `Pair()` is called without
an active BLE connection, BlueZ may attempt BR/EDR (Classic Bluetooth) paging,
which fails with "Page Timeout".

**On macOS:**

CoreBluetooth handles pairing automatically when accessing an encrypted
characteristic. The system shows a PIN dialog; there is no programmatic API
to supply the PIN. macOS is unsuitable for automated pairing.

### Sold-On Date

The sold-on date is the date the bike was "sold" or activated in Stark's
system.

**Normalization:**
- Strip all non-digit characters.
- If fewer than 8 digits remain, use the default: `"19700101"` (Unix epoch).
- Otherwise, take the first 8 digits.

**Default / fallback:** `"19700101"`. This is used for:
- Bikes that were never officially sold or activated.
- Demo/prototype units.
- The official app's automatic fallback after 2 failed pairing attempts.

---

## Application-Layer Authentication (V2)

### When It Applies

The V2 auth handshake is **firmware-dependent**. The security characteristic
(`0x1001`) determines which flow is used:

| Security Char Properties | Auth Required | Notes |
|-------------------------|--------------|-------|
| **Read + Write + Notify** | Yes | The characteristic is readable; the bike serves a 32-byte nonce on read. The full V2 handshake must be completed before telemetry is accessible. |
| **Write + Notify** (no Read) | No | The characteristic is not readable. The nonce read times out and auth is skipped. The BLE bond alone is sufficient. |

On the tested Gen 1 Varg, the security characteristic is **write+notify only**,
so V2 auth is not required. The following section documents the V2 handshake
for firmware versions that do require it.

### Intermediate Key Derivation

Before the handshake, a 16-byte intermediate key `K` is derived from the VIN
and sold-on date using variant-specific lookup tables.

```
hash = SHA-256("VIN-SOLDDATE")    // same hash as PIN derivation

HASH_IDX_V2 = [0x19, 0x05, 0x07, 0x16, 0x13, 0x0c, 0x16, 0x0b,
               0x0c, 0x05, 0x18, 0x0d, 0x17, 0x17, 0x12, 0x30]

MIX_V2 = [0xc356, 0x9b94, 0x0dbd, 0xc561, 0x3139, 0x300f, 0xee5a, 0xe883,
          0x4638, 0x89ca, 0x8b30, 0x593d, 0xe47d, 0x2ea2, 0x89bc, 0xaafe]

for i in 0..16:
    idx = HASH_IDX_V2[i] XOR (i + 0x11)           // always in range 0..31
    mix = (i * 0x1111) XOR MIX_V2[i] XOR 0xFFFFA5A5
    mix_hi = (mix >> 8) & 0xFF
    mix_lo = mix & 0xFF
    K[i] = (hash[idx] XOR mix_hi) wrapping_add mix_lo   // u8 arithmetic is correct here
```

The byte-level wrapping (`wrapping_add`) is correct for K derivation because
only the low byte of the result is stored. This is different from the PIN
derivation where 32-bit arithmetic is required.

### Challenge-Response Handshake

Performed over the security characteristic (`0x1001`).

**Step 1 — Read nonce:** Client reads the security characteristic. The bike
returns a 32-byte random nonce.

**Step 2 — Build payload:**
```
header = [0x02, 0x01]                          // 2 bytes: type=V2, version=1
message = K || header || nonce                  // 16 + 2 + 32 = 50 bytes
digest = SHA-256(message)                       // 32 bytes
payload = header || digest                      // 2 + 32 = 34 bytes
```

**Step 3 — Subscribe and write:** Enable notifications on the security
characteristic (CCCD write), then write the 34-byte payload.

**Step 4 — Read verdict:** The bike sends a notification on the security
characteristic. First byte:
- `0x01` = success.
- Anything else = failure.

On success, the client disables notifications on the security characteristic
and proceeds to read telemetry.

### V1 Variant

A V1 auth path also exists with a different set of lookup tables. The V1
header would be `[0x01, 0x01]`. V1 is not used by the current official app
(variant=2 is always used) and has not been tested.

**V1 hash index table:**
```
HASH_IDX_V1 = [0x0b, 0x1f, 0x15, 0x03, 0x04, 0x02, 0x07, 0x12,
               0x0a, 0x18, 0x18, 0x09, 0x14, 0x16, 0x1e, 0x3f]
```

**V1 mix table:**
```
MIX_V1 = [0x2fbf, 0x9b94, 0xeca2, 0xf03d, 0x31e9, 0x308f, 0xee5a, 0x5f9e,
          0xa910, 0x88f6, 0x483c, 0x4cc1, 0x53c0, 0x2cde, 0x899c, 0xb486]
```

---

## Decoded Data Types

All multi-byte values are **little-endian**.

### Status Bits (0x1002 — 18 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | `misc_bits` |
| 2–3 | u16 | `indicator_bits` |
| 4–5 | u16 | `alert_bits` |
| 6–7 | u16 | `fault_bits` |
| 8–9 | u16 | `info_bits` |
| 10 | u8 | `lock_status` |
| 11–12 | u16 | `lock_time` |
| 13 | u8 | `update_available` (0 or 1) |
| 14–17 | u32 | `battery_status` |

**`misc_bits` fields:**

| Bits | Field |
|------|-------|
| 0–3 | Walking mode (0–15) |
| 8–15 | Vibration amplitude |

**`indicator_bits` fields:**

| Bit | Field |
|-----|-------|
| 0 | Blinker left |
| 1 | Blinker right |
| 2 | Beam long (high beam) |
| 3 | Horn |
| 4 | Throttle armed |
| 5 | Drive engaged |
| 6 | Throttle cable damaged |
| 8 | Throttle not closed on turning on |
| 9 | Throttle not closed on engage |
| 10 | Bike not stopped |
| 11 | MIL error |

**`info_bits` fields:**

| Bit | Field |
|-----|-------|
| 0 | Charger connected |
| 1 | Is charging |
| 2 | Is docked |
| 4 | Is hibernating (inverted: 0 = hibernating) |
| 5 | Pump on |
| 6 | Fan on |

**`fault_bits` fields:**

| Bit | Field |
|-----|-------|
| 0 | General fault |
| 1 | Battery temperature |
| 2 | Motor temperature |
| 3 | Coolant pump |
| 4 | Cooling fan |
| 5 | Insulation |
| 6 | Derating (battery) |
| 7 | Derating (IGBT) |

### Identity (0x1003 — 23 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–16 | ASCII | VIN (17 characters) |
| 17–22 | ASCII | Sold date (6 characters, YYMMDD) |

### Versions (0x1005 — 100 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | BLE version |
| 2–3 | i16 | Download percent |
| 4–6 | 3 × u8 | Blob FS version (`[6].[5].[4]`) |
| 8–10 | 3 × u8 | Blob server version (`[10].[9].[8]`) |
| 12–99 | 11 × 8 bytes | Component versions (see below) |

Each component occupies 8 bytes: 3 bytes for the current version, 1 byte
padding, 3 bytes for the available version, 1 byte padding. Version bytes
are formatted as `[+2].[+1].[+0]`.

Components in order: VCU PIC, ESP Top, ESP Bottom, Inverter Logic,
Inverter Gate, BMS Positive, BMS Negative, Map Switch, Light Front,
Light Rear, Docking.

### Speed (0x2001 — 4 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | Speed (raw value / 10 = km/h) |
| 2–3 | u16 | Motor RPM |

### Throttle (0x2002 — 6 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | Throttle position |
| 2–3 | i16 | Iq feedback (current) |
| 4–5 | i16 | Id feedback (current) |

### IMU (0x2003 — 12 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | i16 | Accelerometer X |
| 2–3 | i16 | Accelerometer Y |
| 4–5 | i16 | Accelerometer Z |
| 6–7 | i16 | Gyroscope X |
| 8–9 | i16 | Gyroscope Y |
| 10–11 | i16 | Gyroscope Z |

### Totals (0x2005 — 16 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–3 | u32 | Odometer |
| 4–7 | u32 | Total watt-hours |
| 8–11 | u32 | Total airtime (seconds) |
| 12–15 | u32 | Total ride time (seconds) |

### Estimations (0x2006 — 6 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | Estimated range (km) |
| 2–3 | u16 | Estimated time (minutes) |
| 4–5 | i16 | Motor power (watts, negative = regen) |

### Racing (0x2007 — 9 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0 | u8 | Racing mode |
| 1 | u8 | Racing curve |
| 2–3 | u16 | Throttle multiplier |
| 4 | u8 | Category |
| 5–8 | u32 | Expiry timestamp (Unix seconds) |

### Battery Level (0x2a19 — 1 byte)

Standard BLE Battery Level characteristic. Value is 0–100 (percentage).

---

## Battery Data Types

### Battery Status (0x6001 — 8 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–3 | u32 | `fault_bits_pos` — positive side fault bits |
| 4–7 | u32 | `fault_bits_neg` — negative side fault bits |

### Battery Firmware Version (0x6002 — 12 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–2 | 3 × u8 | Positive BMS firmware version (`[2].[1].[0]`) |
| 4–6 | 3 × u8 | Negative BMS firmware version (`[6].[5].[4]`) |
| 8–11 | u32 | Serial number |

### Battery Parameters (0x6003 — 4 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0 | u8 | Cells in series |
| 1 | u8 | Parallel strings |
| 2–3 | u16 | Capacity (mAh) |

### Battery SOC (0x6004 — 6 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | State of charge |
| 2–3 | u16 | State of health |
| 4–5 | u16 | DC bus voltage |

### Battery Temperatures (0x6005 — 27 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–23 | 12 × u16 | Temperature sensor readings |
| 24–25 | u16 | Valid sensor mask |
| 26 | u8 | Number of sensors in use |

### Battery Cells (0x6007 — 200 bytes)

100 cell voltages, each as u16 LE (2 bytes per cell). Sentinel value
`0xFFFF` indicates an inactive cell.

### Battery Signals (0x6009 — 18 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | Positive DC bus |
| 2–3 | u16 | Positive temperature |
| 4–5 | u16 | Positive humidity |
| 6–7 | u16 | Positive control |
| 8–9 | u16 | Negative DC bus |
| 10–11 | u16 | Negative temperature |
| 12–13 | u16 | Negative humidity |
| 14–15 | u16 | Negative control |
| 16–17 | i16 | Battery current (signed) |

---

## Inverter Data Types

### Inverter Info (0x7001 — 23 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | Fault codes |
| 2–5 | u32 | Status |
| 6–7 | u16 | MCC data 1 error count |
| 8–9 | u16 | MCC data 2 error count |
| 10–12 | 3 × u8 | Logic firmware version (`[12].[11].[10]`) |
| 14–16 | 3 × u8 | Gate driver firmware version (`[16].[15].[14]`) |
| 18 | u8 | Hardware version |
| 19–22 | u32 | Humidity (divide by 100 for %) |

### Inverter Signals (0x7002 — 14 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | DC bus voltage |
| 2–3 | u16 | Iq reference |
| 4–5 | u16 | Id reference |
| 6–7 | i16 | Iq actual (signed) |
| 8–9 | i16 | Id actual (signed) |
| 10–11 | u16 | Vq |
| 12–13 | u16 | Vd |

### Inverter Temperatures (0x7003 — 16 bytes)

Two groups of 8 bytes: motor sensors (0–7) and IGBT sensors (8–15).
Each group:

| Offset | Type | Field |
|--------|------|-------|
| +0/+1 | u16 | Sensor 1 (divide by 10 for °C) |
| +2/+3 | u16 | Sensor 2 |
| +4/+5 | u16 | Sensor 3 |
| +6 | u8 | Valid flags |
| +7 | u8 | Used flags |

### Inverter PCB (0x7004 — 18 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | MCU logic temperature |
| 2–3 | u16 | MCU gate temperature |
| 4–5 | u16 | NTC 1 |
| 6–7 | u16 | NTC 2 |
| 8–9 | u16 | NTC 3 |
| 10–11 | u16 | PCB temperature |
| 12–13 | u16 | PCB humidity (divide by 100 for %) |
| 14–17 | u32 | Serial number |

---

## Charger Data (0x5001 — 18 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | Requested current |
| 2–3 | u16 | Reported current |
| 4–5 | u16 | Cell charge voltage |
| 6–7 | u16 | Max charge current |
| 8–9 | u16 | Max charge power |
| 10–11 | u16 | Max charge SOC |
| 12–13 | u16 | Requested voltage |
| 14–15 | u16 | Reported voltage (optional) |
| 16 | u8 | Charger status (optional) |
| 17 | bool | Charger enabled (optional) |

---

## Docking Data

### Docking Version (0x3001 — 3 bytes)

Firmware version: `[2].[1].[0]`.

### Docking QI Status (0x3002 — 1 byte)

QI wireless charging status code.

---

## VCU Data Types

### VCU Versions (0x4001 — 20 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–2 | 3 × u8 | PIC VCU version |
| 4–6 | 3 × u8 | Top ESP VCU version |
| 8–10 | 3 × u8 | Bottom ESP VCU version |
| 12–14 | 3 × u8 | FWFS version |
| 16–19 | u32 | Serial number |

### VCU Info (0x4002 — 10 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0–1 | u16 | Fan current |
| 2–3 | u16 | Pump current |
| 4–5 | u16 | Pump OK counter |
| 6–7 | u16 | Humidity (divide by 100 for %) |
| 8–9 | u16 | Temperature (divide by 100 for °C) |

---

## VCU Configuration

Service `0x4000`, characteristic `0x4005`. This is a request/response channel
for reading and writing bike configuration. All interactions follow the same
pattern:

**Read request:** Write `[0x00, type, ...read_payload]` to `0x4005`.
The bike responds via notification with `[0x02, type, status, ...data]`.

**Write request:** Write `[0x01, type, ...write_payload]` to `0x4005`.

Note: the request has **no status byte** (2-byte header), but the response
has a 3-byte header (readWrite=2, type, status). Some BLE stacks echo the
write back as a notification — filter to both `readWrite=2` AND the expected
type byte to avoid consuming stale responses from other config types.
Responses may arrive out of order.

**Config types:**

| Type | Name | Writable | Read Payload | Description |
|------|------|----------|-------------|-------------|
| 0 | Map Config | Yes | 1 byte (slot) | Power mode settings per slot |
| 1 | Curves Config | Yes | 1 byte (curve index) | 15-point power curves across RPM range |
| 2 | Racing Config | Yes | (none) | Racing mode settings |
| 3 | Misc Config | Yes | (none) | Map count, timeouts |
| 4 | Charger Config | Yes | (none) | Charging parameters |
| 5 | Lock Bike Config | Yes | (none) | Lock/security settings |
| 7 | Totals Config | Read-only | 2 bytes (flags) | Odometer/ride time. May return empty on some firmware. |

### Map Config (type 0)

Defines a power mode slot. The bike has 5 slots (0–4) selectable via the
handlebar map switch. The number of configured slots is reported by
[Misc Config](#misc-config-type-3). All slots may be configured
identically from the factory.

**Read payload:** 1 byte — slot number.

**Response data (6 bytes):**

| Offset | Type | Field | Notes |
|--------|------|-------|-------|
| 0 | u8 | Slot number | |
| 1–2 | i16 | Torque | Divide by 1.25 for power as HP. Max 80hp = torque 100. |
| 3–4 | i16 | Regen | 0 = no engine braking, 100 = maximum. |
| 5 | u8 | Curve | Index into power curves (0 = built-in default, 1–4 = custom). |

**Write payload (7 bytes):**

| Offset | Type | Field |
|--------|------|-------|
| 0 | u8 | Slot number |
| 1 | u8 | Save flag (1 = persist to flash) |
| 2–3 | i16 | Torque |
| 4–5 | i16 | Regen |
| 6 | u8 | Curve |

**App defaults** (from app constants):
- `MAX_POWER_HP = 80` — the bike's absolute maximum.
- `STANDARD_POWER_HP = 60` — what the app considers "standard."
- `PARENTAL_DEFAULT_POWER_HP = 20` — the parental/beginner lock.
- `POWER_MODES_DEFAULT = [44, 48, 52, 56, 60]` — HP for slots 0–4,
  all with regen 100 and curve 0.

Note: the bike's actual configuration may differ — a tested Gen 1 Varg
had all 5 slots set to 40 HP / regen 70 / curve 0 (likely set by a
firmware update, not the factory defaults).

### Curves Config (type 1)

A power curve defines how much of the available torque and regen to
deliver at each RPM. The bike stores up to 5 curves (indices 0–4).
Curve 0 is the built-in default and **returns an empty response when
read** — it cannot be read or modified. Curves 1–4 are user-configurable
and default to all-1000 values (flat/linear).

**The 15 points correspond to RPM breakpoints** from 0 to 14,000 RPM
in 1,000 RPM steps:

```
Point:  0     1     2     3     4     5     6     7     8     9    10    11    12    13    14
RPM:    0  1000  2000  3000  4000  5000  6000  7000  8000  9000 10000 11000 12000 13000 14000
```

Each value is a **scale factor from 0 to 1000** (where 1000 = 100% of
available power at that RPM). For example:
- All 1000 = flat power delivery across the full RPM range (default).
- `[300, 400, 500, ..., 1000]` = reduced power at low RPM, full at
  high RPM (gentler off the line).
- `[500, 750, 1000, 1000, ..., 800, 700]` = peaks in the mid-range.

**Read payload:** 1 byte — curve index.

**Response data (61 bytes, or 0 bytes for curve 0):**

| Offset | Type | Field |
|--------|------|-------|
| 0 | u8 | Curve index |
| 1–60 | 15 × 4 bytes | 15 RPM points, each: torque (u16 LE) + regen (u16 LE) |

**Write payload (66 bytes):** save (u8), curve (u8), 2 × i16 padding
(0x7FFF), then 15 × torque (i16 LE), then 15 × regen (i16 LE).

### Racing Config (type 2)

**Read payload:** (none — 0 bytes).

**Response data (10 bytes):**

| Offset | Type | Field |
|--------|------|-------|
| 0 | u8 | Racing mode |
| 1 | u8 | Curve |
| 2 | u8 | Neutral on threshold |
| 3 | u8 | Neutral off threshold |
| 4 | u8 | Engage threshold |
| 5 | u8 | Category |
| 6–9 | u32 | Expiry timestamp (Unix seconds) |

**Write payload (6 bytes):** mode (u8), category (u8), expire timestamp
(u32 LE).

### Misc Config (type 3)

General bike settings including the number of configured power mode slots.

**Read payload:** (none — 0 bytes).

**Response data (5 bytes):**

| Offset | Type | Field | Notes |
|--------|------|-------|-------|
| 0 | u8 | Maps | Number of configured power mode slots. Observed: 5. |
| 1–2 | u16 | Inactive timeout | Inactivity timeout. Observed: 15 (likely seconds). |
| 3–4 | u16 | Auto power off | Auto power-off timeout. Observed: 600 (likely seconds = 10 minutes). |

**Write payload (6 bytes):** save (u8), maps (u8), inactive timeout
(i16 LE), auto power off (i16 LE).

### Charger Config (type 4)

Charging parameters.

**Read payload:** (none — 0 bytes).

**Response data (12 bytes):**

| Offset | Type | Field | Observed |
|--------|------|-------|----------|
| 0–1 | i16 | Charge current | 100 |
| 2–3 | i16 | Charge power | 3300 (likely watts = 3.3 kW onboard charger) |
| 4–5 | i16 | Max SOC | 1000 (likely 100.0% × 10 = charge to full) |
| 6–7 | i16 | Min current | 20 |
| 8–9 | i16 | Start time | 2 |
| 10–11 | i16 | Ramp time | 16 |

**Write payload (7 bytes):** save (u8), charge current (i16 LE), charge
power (i16 LE), max SOC (i16 LE). Note: write is shorter than read — only
the first three fields are writable.

### Lock Bike Config (type 5)

Bike lock and security settings.

**Read payload:** (none — 0 bytes).

**Response data (4 bytes):**

| Offset | Type | Field |
|--------|------|-------|
| 0 | u8 | Lock status |
| 1 | u8 | Lock type |
| 2–3 | u16 | Lock timeout |

**Write payload (5 bytes):** action (u8), lock status (u8), lock type
(u8), lock timeout (i16 LE).

### Totals Config (type 7)

Odometer and total ride time. Treat as **read-only**. May return an
empty response on some firmware versions — in that case, use the live
Totals notification (0x2005) instead.

**Read payload:** 2 bytes — flags (u16 LE). Bit 0 = include odometer,
bit 1 = include total ride time. Send `0x03 0x00` for both.

**Response data (variable):**

| Offset | Type | Field | Condition |
|--------|------|-------|-----------|
| 0–1 | u16 | Flags | Always present |
| 2–5 | u32 | Odometer (meters) | If flags & 1 |
| next 4 | u32 | Total ride time (seconds) | If flags & 2 |

The odometer value is in **meters** (divide by 1000 for km).

---

## TLV Format

Several characteristics use TLV (Type-Length-Value) encoding for multiplexed
data. The format is:

```
While data remains:
    type   = data[offset]        // u8
    length = data[offset + 1]    // u8
    value  = data[offset + 2 .. offset + 2 + length]
    offset += 2 + length
```

### Extended TLV (0x1100) Type IDs

| Type | Name | Data |
|------|------|------|
| 1 | Fast Bits | 8 bytes: misc_bits (u16), indicator_bits (u16), alert_bits (u16), info_bits (u16). Compact status update — same fields as Status Bits bytes 0–7 but with `info_bits` replacing `fault_bits`. |
| 2 | Lock Status | 3 bytes: lock_status (u8), lock_time (u16). |
| 3 | Update Available | 1 byte: 0 = no update, 1 = update available. |

### Live TLV (0x2100) Type IDs

| Type | Name | Data |
|------|------|------|
| 1 | Speed | 4 bytes (same format as [Speed](#speed-0x2001--4-bytes)). |
| 2 | Throttle | 6 bytes (same format as [Throttle](#throttle-0x2002--6-bytes)). |
| 3 | IMU | 12 bytes (same format as [IMU](#imu-0x2003--12-bytes)). |
| 4 | Maps | 1 byte — active power mode slot. |
| 5 | Estimations | 6 bytes (same format as [Estimations](#estimations-0x2006--6-bytes)). |
| 6 | Totals | 16 bytes (same format as [Totals](#totals-0x2005--16-bytes)). |
| 7 | Racing | 9 bytes (same format as [Racing](#racing-0x2007--9-bytes)). |

### Docking TLV (0x3100) Type IDs

| Type | Name | Data |
|------|------|------|
| 1 | Version | 3 bytes — docking firmware version (same as 0x3001). |
| 2 | QI Status | 1 byte — QI charging status (same as 0x3002). |

### VCU TLV (0x4100) Type IDs

| Type | Name | Data |
|------|------|------|
| 1 | Versions | 20 bytes — VCU firmware versions (same as 0x4001). |
| 2 | Info | 10 bytes — VCU info (same as 0x4002). |

### Charger TLV (0x5100) Type IDs

| Type | Name | Data |
|------|------|------|
| 1 | Charger Data | Up to 18 bytes — all charger fields (same as 0x5001). |

### Battery TLV (0x6100) Type IDs

| Type | Name | Data |
|------|------|------|
| 2 | Signals | 18 bytes — BMS signals (same as 0x6009). |
| 3 | Cells | 200 bytes — cell voltages (same as 0x6007). |
| 4 | Balancing | ~13 bytes — raw balancing data (same as 0x6008). |
| 5 | Temperatures | 27 bytes — temperature sensors (same as 0x6005). |
| 6 | Params | 4 bytes — battery parameters (same as 0x6003). |
| 7 | Firmware Version | 12 bytes — BMS firmware versions (same as 0x6002). |
| 8 | SOC | 6 bytes — state of charge/health (same as 0x6004). |
| 9 | Info | 26 bytes — serial info and manufacturer code. |

### Inverter TLV (0x7100) Type IDs

| Type | Name | Data |
|------|------|------|
| 1 | Signals | 14 bytes — inverter signals (same as 0x7002). |
| 2 | IGBT Temperatures | 8 bytes — IGBT temperature sensors only (half of 0x7003). |
| 3 | Motor Temperatures | 8 bytes — motor temperature sensors only (half of 0x7003). |
| 4 | PCB | 18 bytes — PCB data (same as 0x7004). |
| 5 | Info | ~23 bytes — inverter info (same as 0x7001). |

---

## Data Redundancy

Several pieces of information are available from multiple sources. This
table documents the overlaps to help implementations choose the most
appropriate source for each use case.

| Data | Notification Source | Config Request | TLV Source | Notes |
|------|-------------------|----------------|------------|-------|
| **Odometer** | Totals (0x2005) | Totals Config (type 7) | Live TLV type 6 | Notification gives meters in real-time; config gives stored value. |
| **Total ride time** | Totals (0x2005) | Totals Config (type 7) | Live TLV type 6 | Same as odometer. |
| **Active map slot** | Maps (0x2004) | — | Live TLV type 4 | Notification only; to read the slot's *contents*, use Map Config. |
| **Speed / RPM** | Speed (0x2001) | — | Live TLV type 1 | Dedicated characteristic or TLV; same data. |
| **Throttle** | Throttle (0x2002) | — | Live TLV type 2 | Same. |
| **IMU** | IMU (0x2003) | — | Live TLV type 3 | Same. |
| **Status bits** | Status Bits (0x1002) | — | Bike TLV type 1 | TLV sends a compact 8-byte "fast bits" update (misc, indicators, alerts, info). The dedicated characteristic sends the full 18-byte struct including faults and lock. |
| **Lock status** | Status Bits (0x1002) | Lock Config (type 5) | Bike TLV type 2 | Three sources: embedded in the full status struct, as a config response, or as a TLV entry. |
| **Update available** | Status Bits (0x1002) | — | Bike TLV type 3 | Both the dedicated byte in status bits and the TLV entry. |
| **Battery SOC** | Battery SOC (0x6004) | — | Battery TLV type 8 | Same data via dedicated characteristic or TLV. |
| **Battery temps** | Battery Temps (0x6005) | — | Battery TLV type 5 | Same. |
| **Inverter temps** | Inverter Temps (0x7003) | — | Inverter TLV types 2+3 | Dedicated char sends motor+IGBT together; TLV sends them as separate entries. |
| **VCU versions** | VCU Versions (0x4001) | — | VCU TLV type 1 | Same. |

**General guidance:** Use notifications for real-time telemetry (they push
data continuously while the bike is on). Use config requests for one-shot
reads of settings that don't change during a ride. Use TLV as a fallback
or when a single multiplexed stream is preferred over many subscriptions.

---

## Scaling Factors and Units

Raw BLE values often need scaling before display. Sentinel value `0xFFFF`
(u16) or `0x7FFF` (i16) means "no data available."

| Value | Raw Unit | Display Unit | Conversion |
|-------|----------|-------------|------------|
| Speed | u16 | km/h | ÷ 10 |
| Motor RPM | u16 | RPM | Direct (filter ≥ 30000 as invalid) |
| Torque (MapConfig) | i16 | HP | ÷ 1.25 |
| Motor temperature | u16 | °C | ÷ 10 |
| Battery temperature | u16 | °C | ÷ 100 |
| VCU temperature | u16 | °C | ÷ 100 |
| Inverter PCB humidity | u16 | % | ÷ 100 |
| VCU humidity | u16 | % | ÷ 100 |
| Inverter humidity | u32 | % | ÷ 100 |
| Battery DC bus voltage | u16 | V | ÷ 10 |
| Odometer | u32 | m | Direct (÷ 1000 for km) |
| Airtime / ride time | u32 | seconds | Direct |
| Watt-hours | u32 | Wh | Direct |
| Battery current | i16 | A | Direct |
| Motor power | i16 | W | Direct (negative = consuming) |
| Throttle position | u16 | (raw) | Direct |

---

## Firmware Variants

The protocol behavior varies across firmware versions:

| Aspect | Firmware A (Readable Security) | Firmware B (Write+Notify Security) |
|--------|-------------------------------|----------------------------------|
| Security char properties | Read + Write + Notify | Write + Notify |
| V2 auth handshake | Required | Skipped (bond-only) |
| Nonce delivery | Via GATT read | N/A |
| Tested on | (untested on hardware) | Gen 1 Varg (confirmed) |

Implementations should check the security characteristic's property flags
after service discovery and adapt accordingly.

---

## Wi-Fi Firmware Flashing

The bike also has a Wi-Fi access point for firmware updates. This is a
separate transport from BLE.

- **SSID format:** `VARG-<VIN>` (e.g. `VARG-UDUMX1AB2CD012345`)
- **Purpose:** Firmware binary transfer to the ESP-based module on the bike
- **Operations:** Binary upload, console commands, programming sequence

The Wi-Fi flashing protocol has not been decoded beyond identifying its
existence. It may serve as an emergency recovery path if BLE access is ever
bricked.
