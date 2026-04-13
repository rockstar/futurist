pub mod dump;

// To add a feature-gated subcommand:
//
//   #[cfg(feature = "dash")]
//   pub mod dash;
//
//   // in Command enum:
//   #[cfg(feature = "dash")]
//   Dash(dash::DashArgs),

#[derive(clap::Subcommand)]
pub enum Command {
    /// Dump raw telemetry frames from the bike to stdout.
    Dump(dump::DumpArgs),
}
