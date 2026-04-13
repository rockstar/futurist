use clap::Parser;

#[derive(Parser)]
#[command(
    name = "futurist",
    about = "Open tools for the Stark Varg electric motorcycle"
)]
struct Args {
    #[command(subcommand)]
    command: futurist::cli::Command,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        futurist::cli::Command::Config(config_args) => {
            futurist::cli::config::run(config_args).await
        }
        futurist::cli::Command::Dump(dump_args) => futurist::cli::dump::run(dump_args).await,

        #[cfg(feature = "dash")]
        futurist::cli::Command::ConfigUi(args) => futurist::cli::config_ui::run(args),

        #[cfg(feature = "dash")]
        futurist::cli::Command::Dash(dash_args) => futurist::cli::dash::run(dash_args),
    }
}
