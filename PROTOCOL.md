# Stark Varg BLE Protocol

This document describes the Bluetooth Low Energy protocol used by the Stark
Varg motorcycle. It is intended as a complete reference for implementing a
third-party client without the official app or Stark's backend services.

Validated against a Gen 1 Stark Varg.

---

## Table of Contents

- [Overview](#overview)
- [BLE Advertising](#ble-advertising)
- [GATT Service and Characteristics](#gatt-service-and-characteristics)
- [Pairing](#pairing)
  - [PIN Derivation Algorithm](#pin-derivation-algorithm)
  - [Pairing Procedure](#pairing-procedure)
  - [Sold-On Date](#sold-on-date)
- [Application-Layer Authentication (V2)](#application-layer-authentication-v2)
  - [When It Applies](#when-it-applies)
  - [Intermediate Key Derivation](#intermediate-key-derivation)
  - [Challenge-Response Handshake](#challenge-response-handshake)
- [Characteristic Map](#characteristic-map)
- [Telemetry (Not Yet Decoded)](#telemetry-not-yet-decoded)
- [Commands (Not Yet Decoded)](#commands-not-yet-decoded)
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
4. **Read/subscribe** to telemetry characteristics for live data.
5. **Write** to the command characteristic to change settings.

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

## GATT Service and Characteristics

All custom characteristics share a common UUID base derived from "StarK Future"
in ASCII:

```
Base: XXXXXXXX-5374-6172-4b20-467574757265
              S t a r K   F u t u r e
```

### Primary Service

```
UUID: 00001000-5374-6172-4b20-467574757265
```

### Characteristics

| UUID | Short Name | Properties | Description |
|------|-----------|------------|-------------|
| `00001001-...-467574757265` | Security | Write, Notify | Authentication handshake (see [Application-Layer Authentication](#application-layer-authentication-v2)). On some firmware versions, the V2 handshake is not used and this characteristic is inert after pairing. |
| `00001002-...-467574757265` | Bike Data | Read, Notify | Primary telemetry frame (18 bytes observed). |
| `00001003-...-467574757265` | Bike Data 2 | Read, Notify | Secondary telemetry frame (23 bytes observed). |
| `00001005-...-467574757265` | Live Data | Read, Notify | Large telemetry struct (100 bytes observed). First byte appears to be bike-on status (`0x01` = on). |
| `00001006-...-467574757265` | Command | Write | Write-only command characteristic for configuration changes (ride mode, settings, etc.). |
| `00001100-...-467574757265` | VCU Data | Read, Notify | VCU (Vehicle Control Unit) data. May require a command write to `00001006` to populate. |
| `00002a19-0000-1000-8000-00805f9b34fb` | Battery Level | Read, Notify | Standard BLE Battery Level characteristic. Single byte, 0-100 (percentage). |

**Note:** Characteristics `00001004` and `00001101` may appear on different
firmware versions or hardware revisions but were not present on the tested
bike's GATT server.

### CCCD (Notifications)

To receive live telemetry updates, write `0x01 0x00` to the Client
Characteristic Configuration Descriptor (UUID `00002902-0000-1000-8000-00805f9b34fb`)
on the desired characteristic. This is standard BLE notification enablement
and is handled automatically by most BLE libraries (`subscribe()` / `notify()`).

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
(`00001001`) determines which flow is used:

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

Performed over the security characteristic (`00001001`).

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

## Characteristic Map

Detailed byte-level decoding of each characteristic is a work in progress.

### 00001005 — Live Data (100 bytes)

The largest telemetry characteristic. Fields are expected to include:

- Byte 0: Bike status (`0x01` = on, `0x00` = off)
- Remaining bytes: TBD (speed, RPM, motor temp, inverter temp, battery SOC,
  current draw, odometer, etc.)

Subscribe to notifications for real-time updates while riding.

### 00001002 — Bike Data (18 bytes)

Primary data frame. Likely contains summary/status information.

### 00001003 — Bike Data 2 (23 bytes)

Secondary data frame.

### 00001100 — VCU Data (variable)

VCU configuration and status. May return empty until a specific request is
written to the command characteristic (`00001006`). Request payloads use a
`(period, targetStructId)` format where both values are 16-bit, selecting what
data the VCU should report and at what interval.

### 00001006 — Command (write-only)

Accepts command payloads. Used for:
- Ride mode changes
- Configuration writes (LED settings, power limits, etc.)
- Firmware update initiation

Command framing and opcodes are not yet decoded.

### 00002a19 — Battery Level (1 byte)

Standard BLE Battery Level characteristic. Value is 0-100 (percentage).

---

## Telemetry (Not Yet Decoded)

The byte-level layout of the telemetry characteristics has not been fully
mapped. Expected data model fields include:

- **Live data** — real-time riding data (speed, RPM, temperatures)
- **BMS signals** — battery management system (cell voltages, SOC, temperatures)
- **Inverter temperatures** — motor controller temperatures
- **Totals** — odometer, energy consumed
- **IMU** — inertial measurement unit (lean angle, acceleration)
- **Estimations** — range estimates
- **Component data** — hardware component status

Cross-referencing with captured telemetry data while the bike is active will
be needed to produce a complete field map.

---

## Commands (Not Yet Decoded)

The command characteristic (`00001006`) accepts write payloads for
configuration changes. Commands are serialized through a queue to prevent
write collisions.

The command format (opcode, length, payload structure) has not been decoded yet.

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
