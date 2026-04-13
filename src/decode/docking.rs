//! Decoders for service 0x3000 — Docking.

use super::read_version;

/// 0x3001 — Docking firmware version (3 bytes).
#[derive(Debug, Clone)]
pub struct DockingVersion {
    pub version: String,
}

impl DockingVersion {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 3 {
            return None;
        }
        Some(Self {
            version: read_version(data, 0),
        })
    }
}

/// 0x3002 — Docking QI charging status (1 byte).
#[derive(Debug, Clone)]
pub struct DockingQiStatus {
    pub status: u8,
}

impl DockingQiStatus {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self { status: data[0] })
    }
}
