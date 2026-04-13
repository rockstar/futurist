//! Power mode presets for the Stark Varg.
//!
//! These are named configurations that can be written to the bike's map
//! slots via the VCU Config characteristic (0x4005). Each preset defines
//! power (as HP), regenerative braking strength, and throttle curve.
//!
//! # Key parameters
//!
//! - **Power (HP)**: 0–80. The bike's absolute max is 80hp. The official
//!   app's default range is 44–60hp across 5 slots. 20hp is the "parental"
//!   mode for beginners.
//!
//! - **Regen**: 0–100. Controls engine braking when you release the
//!   throttle. 100 = maximum engine braking (aggressive, can pitch you
//!   forward). 0 = no engine braking (coasts freely). 50 is moderate.
//!   The factory default is 100; 70 is common after updates.
//!
//! - **Curve**: 0–4. Index into the bike's throttle response curves.
//!   0 = default/standard response. Higher values are custom curves
//!   that can be configured separately.
//!
//! # Torque conversion
//!
//! The bike stores torque as a raw value. To convert:
//! - HP to raw: `torque_raw = hp * 1.25`
//! - Raw to HP: `hp = torque_raw / 1.25`

/// A named power mode configuration.
#[derive(Debug, Clone)]
pub struct PowerPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub power_hp: u8,
    pub regen: u8,
    pub curve: u8,
}

impl PowerPreset {
    /// Convert power HP to the raw torque value the bike expects.
    pub fn torque_raw(&self) -> i16 {
        (self.power_hp as f32 * 1.25) as i16
    }
}

/// The bike's absolute maximum power.
pub const MAX_POWER_HP: u8 = 80;

/// Maximum regen braking value.
pub const MAX_REGEN: u8 = 100;

// ---------------------------------------------------------------------------
// Stark factory defaults (from Const.PowerModes.POWER_MODES_DEFAULT)
// ---------------------------------------------------------------------------

/// Stark's 5 factory default power modes. All use regen 100 and curve 0.
pub const FACTORY_DEFAULTS: [PowerPreset; 5] = [
    PowerPreset {
        name: "Factory 1",
        description: "Stark default slot 0 — 44hp, full regen",
        power_hp: 44,
        regen: 100,
        curve: 0,
    },
    PowerPreset {
        name: "Factory 2",
        description: "Stark default slot 1 — 48hp, full regen",
        power_hp: 48,
        regen: 100,
        curve: 0,
    },
    PowerPreset {
        name: "Factory 3",
        description: "Stark default slot 2 — 52hp, full regen",
        power_hp: 52,
        regen: 100,
        curve: 0,
    },
    PowerPreset {
        name: "Factory 4",
        description: "Stark default slot 3 — 56hp, full regen",
        power_hp: 56,
        regen: 100,
        curve: 0,
    },
    PowerPreset {
        name: "Factory 5",
        description: "Stark default slot 4 — 60hp, full regen",
        power_hp: 60,
        regen: 100,
        curve: 0,
    },
];

// ---------------------------------------------------------------------------
// Curated presets
// ---------------------------------------------------------------------------

pub const PRESET_MELLOW: PowerPreset = PowerPreset {
    name: "Mellow",
    description: "Gentle power, light engine braking. Good for learning or trails.",
    power_hp: 20,
    regen: 30,
    curve: 0,
};

pub const PRESET_TRAIL: PowerPreset = PowerPreset {
    name: "Trail",
    description: "Moderate power with manageable engine braking.",
    power_hp: 35,
    regen: 50,
    curve: 0,
};

pub const PRESET_SPORT: PowerPreset = PowerPreset {
    name: "Sport",
    description: "Punchy power with firm engine braking.",
    power_hp: 52,
    regen: 70,
    curve: 0,
};

pub const PRESET_RACE: PowerPreset = PowerPreset {
    name: "Race",
    description: "Full standard power with strong engine braking.",
    power_hp: 60,
    regen: 85,
    curve: 0,
};

pub const PRESET_MAX: PowerPreset = PowerPreset {
    name: "Max",
    description: "Absolute maximum. 80hp, full regen. For experienced riders only.",
    power_hp: 80,
    regen: 100,
    curve: 0,
};

/// All curated presets, in order from mildest to most aggressive.
pub const PRESETS: [&PowerPreset; 5] = [
    &PRESET_MELLOW,
    &PRESET_TRAIL,
    &PRESET_SPORT,
    &PRESET_RACE,
    &PRESET_MAX,
];

/// Look up a preset by name (case-insensitive).
pub fn preset_by_name(name: &str) -> Option<&'static PowerPreset> {
    let lower = name.to_lowercase();
    PRESETS
        .iter()
        .find(|p| p.name.to_lowercase() == lower)
        .copied()
}
