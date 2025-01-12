// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use std::io::{BufReader, Write};

use privileged_ipc::{PkexecExecutor, ServiceConnection};

use crate::api::{RecvyMessage, SendyMessage};

/// Run the client component
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let ourselves = std::env::current_exe()?.to_string_lossy().to_string();
    let mut conn = ServiceConnection::new::<PkexecExecutor>(&ourselves, &["--server"])?;

    log::info!("🚀 Sending messages to server...");
    let message = SendyMessage::DoThings(42);
    serde_json::to_writer(&conn.socket, &message)?;
    serde_json::to_writer(&conn.socket, &SendyMessage::ListThePackages)?;
    serde_json::to_writer(&conn.socket, &SendyMessage::WhatsYourUID)?;
    conn.socket.flush()?;
    conn.socket.shutdown(std::net::Shutdown::Write)?;

    log::info!("⏳ Waiting for server responses...");
    let mut buf = BufReader::new(&conn.socket);
    for message in serde_json::Deserializer::from_reader(&mut buf).into_iter::<RecvyMessage>() {
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
            Ok(RecvyMessage::HereIsYourUID(uid)) => {
                log::info!("🎫 Received UID: {}", uid);
            }
            Err(e) => {
                log::error!("💥 Error: {:?}", e);
            }
        }
    }

    conn.socket.shutdown(std::net::Shutdown::Read)?;

    Ok(())
}
