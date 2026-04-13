pub mod config;
#[cfg(feature = "dash")]
pub mod config_ui;
pub mod dump;

#[cfg(feature = "dash")]
pub mod dash;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Read bike configuration (power modes, etc.) via CLI.
    Config(config::ConfigArgs),

    /// Graphical configuration editor.
    #[cfg(feature = "dash")]
    ConfigUi(config_ui::ConfigUiArgs),

    /// Dump raw telemetry frames from the bike to stdout.
    Dump(dump::DumpArgs),

    /// Live telemetry dashboard.
    #[cfg(feature = "dash")]
    Dash(dash::DashArgs),
}
