//! Futurist — open protocol library for the Stark Varg electric motorcycle.
//!
//! See `PROTOCOL.md` at the repository root for the full protocol specification.

#[cfg(feature = "firmware")]
pub mod api;
pub mod ble;
pub mod cli;
pub mod crypto;
pub mod decode;
#[cfg(feature = "firmware")]
pub mod flash;
pub mod presets;
pub mod protocol;
pub mod telemetry;
