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
        futurist::cli::Command::Dump(dump_args) => futurist::cli::dump::run(dump_args).await,
    }
}
