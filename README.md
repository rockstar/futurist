# futurist

> "The futurist is here, gentlemen! He sees all. He knows what's best for you,
> whether you like it or not."
>
> -- Clint Barton, Captain America: Civil War

An open reverse-engineering effort for the Stark Varg electric motorcycle's
Bluetooth Low Energy protocol.

## Why

The Stark Varg is a capable electric motorcycle held back by a closed
ecosystem. Ride modes, power limits, and configuration are locked behind an
official app that requires an active account on Stark's backend servers. If the
company shuts down, changes their terms, or simply doesn't recognize your bike
as yours, you lose the ability to configure the machine you own.

Futurist exists so that owners can understand and control their own hardware
without depending on a cloud service that may not always be there.

## What's here

This project is in its early stages. So far we have:

- **[PROTOCOL.md](PROTOCOL.md)** — A detailed specification of the BLE
  protocol used by the Stark Varg, including advertising behavior, GATT
  characteristics, the pairing PIN derivation algorithm, and the
  application-layer authentication handshake. This document is the primary
  output of the reverse-engineering effort and is intended to be a complete
  reference for anyone building their own tools.

More will come as the protocol is further decoded — telemetry structure, command
opcodes, ride mode configuration, and eventually working client implementations.

## Status

- BLE pairing: **fully reverse-engineered and validated on hardware**
- Application-layer auth (V2 handshake): **reverse-engineered, firmware-dependent**
- Telemetry decoding: not yet started
- Command protocol: not yet started
- Wi-Fi firmware flashing: identified but not decoded

## Legal

This project is an independent research effort. It is not affiliated with,
endorsed by, or associated with Stark Future AB. All information was obtained
through analysis of publicly available software. The goal is interoperability
and owner empowerment, not circumvention of access controls for unauthorized
use.

## Contributing

If you own a Stark Varg and want to help map the telemetry and command
protocol, contributions are welcome. The most useful thing you can do right now
is capture BLE traffic from your bike while using the official app and share
the characteristic data alongside what the app was displaying at the time.

See [PROTOCOL.md](PROTOCOL.md) for the current state of knowledge and where
the gaps are.
