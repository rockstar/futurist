pub mod config;
pub mod dump;

#[cfg(feature = "dash")]
pub mod dash;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Read bike configuration (power modes, etc.).
    Config(config::ConfigArgs),

    /// Dump raw telemetry frames from the bike to stdout.
    Dump(dump::DumpArgs),

    /// Live telemetry dashboard.
    #[cfg(feature = "dash")]
    Dash(dash::DashArgs),
}
