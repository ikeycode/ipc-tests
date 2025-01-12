// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use std::io::{BufReader, Write};

use nix::unistd::getuid;
use privileged_ipc::ServiceListener;

use crate::api::{Package, RecvyMessage, SendyMessage};

/// Example server implementation showcasing privileged IPC communication
///
/// This server demonstrates the core functionality of the crate by:
///
/// - Setting up a privileged service listener
/// - Processing incoming JSON messages
/// - Responding to various message types:
///   - Basic string messages (`DoThings`)
///   - Package listing requests (`ListThePackages`)
///   - System information queries (`WhatsYourUID`)
///
/// # Errors
///
/// Returns an error if:
/// - Service listener creation fails
/// - Socket operations fail
/// - JSON serialization/deserialization fails
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🚀 Starting server...");
    let svs = ServiceListener::new()?;

    let (mut socket, _) = svs.accept()?;
    log::trace!("🔌 accepted client connection");

    let mut buf = BufReader::new(socket.try_clone()?);

    for message in serde_json::Deserializer::from_reader(&mut buf).into_iter::<SendyMessage>() {
        match message {
            Ok(SendyMessage::DoThings(i)) => {
                log::info!("📬 Received: {:?}", i);
                let reply = RecvyMessage::GotThings(format!("I got your message: {}", i));
                serde_json::to_writer(&socket, &reply)?;
            }
            Ok(SendyMessage::ListThePackages) => {
                log::info!("📦 Received: ListThePackages");
                Package::get_sample_packages()
                    .into_iter()
                    .map(RecvyMessage::HereIsOnePackage)
                    .for_each(|reply| {
                        serde_json::to_writer(&socket, &reply).unwrap();
                    });
            }
            Ok(SendyMessage::WhatsYourUID) => {
                log::info!("🔑 Received: WhatsYourUID");
                let reply = RecvyMessage::HereIsYourUID(getuid().into());
                serde_json::to_writer(&socket, &reply)?;
            }
            Err(e) => {
                log::error!("💥 Error: {:?}", e);
            }
        }
        socket.flush()?;
    }

    socket.shutdown(std::net::Shutdown::Both)?;

    Ok(())
}
