// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use privileged_ipc::{IpcClient, PkexecExecutor};

use crate::api::{RecvyMessage, SendyMessage};

/// Example client implementation demonstrating communication with a privileged server
///
/// This function shows how to:
/// - Establish a privileged connection using `IpcClient`
/// - Send multiple serialized messages to the server
/// - Handle responses asynchronously using type-safe message types
/// - Proper error handling with the IpcError type
///
/// # Errors
///
/// Returns a boxed error if any IPC operations fail
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let ourselves = std::env::current_exe()?.to_string_lossy().to_string();
    let mut conn =
        IpcClient::<SendyMessage, RecvyMessage>::new::<PkexecExecutor>(&ourselves, &["--server"])?;

    log::info!("🚀 Sending messages to server...");
    conn.send(&SendyMessage::DoThings(42))?;
    conn.send(&SendyMessage::ListThePackages)?;
    conn.send(&SendyMessage::WhatsYourUID)?;

    conn.shutdown(std::net::Shutdown::Write)?;

    log::info!("⏳ Waiting for server responses...");
    for message in conn.incoming()? {
        match message {
            Ok(RecvyMessage::GotThings(s)) => {
                log::info!("📬 Received: {}", s);
            }
            Ok(RecvyMessage::HereIsOnePackage(s)) => {
                log::info!("📦 Received package: {}", s.name);
                colored_json::to_colored_json_auto(&s)
                    .map(|v| log::trace!("{}", v))
                    .unwrap_or_else(|e| log::error!("JSON error: {}", e));
            }
            Ok(RecvyMessage::EndOfPackages) => break,
            Ok(RecvyMessage::HereIsYourUID(uid)) => {
                log::info!("🎫 Received UID: {}", uid);
            }
            Err(e) => {
                log::error!("💥 Error: {:?}", e);
            }
        }
    }

    Ok(())
}
