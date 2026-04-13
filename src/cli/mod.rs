pub mod dump;

#[cfg(feature = "dash")]
pub mod dash;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Dump raw telemetry frames from the bike to stdout.
    Dump(dump::DumpArgs),

    /// Live telemetry dashboard.
    #[cfg(feature = "dash")]
    Dash(dash::DashArgs),
}
