//! Byte-level decoders for all Stark Varg GATT characteristics.
//!
//! Each submodule handles one GATT service. All multi-byte values are
//! little-endian. Sentinel values (0xFFFF for u16, 0x7FFF for i16,
//! 0xFFFFFFFF for u32) are treated as "no data" and decoded as `None`.

pub mod battery;
pub mod charger;
pub mod docking;
pub mod inverter;
pub mod vcu;

// Re-export sentinel-aware helpers for use by all decoders.

pub(crate) fn opt_u16(data: &[u8], offset: usize) -> Option<u16> {
    let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
    if v == 0xFFFF { None } else { Some(v) }
}

pub(crate) fn opt_i16(data: &[u8], offset: usize) -> Option<i16> {
    let v = i16::from_le_bytes([data[offset], data[offset + 1]]);
    if v == i16::MAX { None } else { Some(v) }
}

pub(crate) fn opt_u32(data: &[u8], offset: usize) -> Option<u32> {
    let v = u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]);
    if v == 0xFFFF_FFFF { None } else { Some(v) }
}

pub(crate) fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

pub(crate) fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub(crate) fn read_version(data: &[u8], offset: usize) -> String {
    format!("{}.{}.{}", data[offset + 2], data[offset + 1], data[offset])
}
