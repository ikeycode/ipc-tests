use clap::Parser;
use ipc_tests::{client, moss_service, server};

/// CLI arguments
#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    /// Run in server mode
    #[clap(long)]
    server: bool,
}

/// Main entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    moss_service::service_init()?;

    let args = Args::parse();

    if args.server {
        server::run()?;
    } else {
        client::run()?;
    }

    Ok(())
}
