// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use clap::Parser;

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

    privileged_ipc::service_init()?;

    let args = Args::parse();

    if args.server {
        json_pkexec::server::run()?;
    } else {
        json_pkexec::client::run()?;
    }

    Ok(())
}
