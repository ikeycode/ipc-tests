// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use nix::unistd::getuid;
use privileged_ipc::{IpcError, IpcServer};

use crate::api::{Package, RecvyMessage, SendyMessage};

/// Example server implementation showcasing privileged IPC communication
///
/// This server demonstrates the core functionality of the crate by:
///
/// - Setting up a privileged IPC server
/// - Processing incoming JSON messages
/// - Responding to various message types:
///   - Basic string messages (`DoThings`)
///   - Package listing requests (`ListThePackages`)
///   - System information queries (`WhatsYourUID`)
///
/// # Errors
///
/// Returns an error if:
/// - Server creation fails
/// - Connection handling fails
/// - Message processing fails
pub fn run() -> Result<(), IpcError> {
    log::info!("🚀 Starting server...");
    let server = IpcServer::<RecvyMessage, SendyMessage>::new()?;

    let mut connection = server.accept()?;
    log::trace!("🔌 accepted client connection");

    let mut incoming = connection.incoming()?;

    while let Some(message) = incoming.next() {
        match message? {
            SendyMessage::DoThings(i) => {
                log::info!("📬 Received: {:?}", i);
                let reply = RecvyMessage::GotThings(format!("I got your message: {}", i));
                connection.send(&reply)?;
            }
            SendyMessage::ListThePackages => {
                log::info!("📦 Received: ListThePackages");
                for package in Package::get_sample_packages() {
                    connection.send(&RecvyMessage::HereIsOnePackage(package))?;
                }
                connection.send(&RecvyMessage::EndOfPackages)?;
            }
            SendyMessage::WhatsYourUID => {
                log::info!("🔑 Received: WhatsYourUID");
                let reply = RecvyMessage::HereIsYourUID(getuid().into());
                connection.send(&reply)?;
            }
        }
    }

    connection.shutdown(std::net::Shutdown::Both)?;

    Ok(())
}
