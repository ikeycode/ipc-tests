// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use privileged_ipc::{DirectExecutor, IpcClient, IpcError, PkexecExecutor};
use serde_derive::{Deserialize, Serialize};

/// Basic request types for moss IPC
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Request {
    /// Ping request to test connection
    Ping,
}

/// Basic response types for moss IPC
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Response {
    /// Pong response to ping
    Pong,
    /// Error response
    Error { message: String },
}

/// Client for interacting with moss-ipc daemon
pub struct MossClient {
    client: IpcClient<Request, Response>,
}

impl MossClient {
    /// Creates a new MossClient with privilege escalation
    pub fn new_privileged() -> Result<Self, IpcError> {
        Self::new_privileged_with_path("/usr/bin/moss")
    }

    /// Creates a new MossClient with privilege escalation and custom moss path
    pub fn new_privileged_with_path(moss_path: &str) -> Result<Self, IpcError> {
        Ok(Self {
            client: IpcClient::new::<PkexecExecutor>(moss_path, &["ipc"])?,
        })
    }

    /// Creates a new MossClient without privilege escalation
    pub fn new_direct() -> Result<Self, IpcError> {
        Self::new_direct_with_path("/usr/bin/moss")
    }

    /// Creates a new MossClient without privilege escalation and custom moss path
    pub fn new_direct_with_path(moss_path: &str) -> Result<Self, IpcError> {
        Ok(Self {
            client: IpcClient::new::<DirectExecutor>(moss_path, &["ipc"])?,
        })
    }

    /// Sends a ping request to test the connection
    pub fn ping(&mut self) -> Result<(), IpcError> {
        self.client.send(&Request::Ping)?;

        // Read response
        if let Some(response) = self.client.incoming()?.next() {
            match response? {
                Response::Pong => Ok(()),
                Response::Error { message } => Err(IpcError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    message,
                ))),
            }
        } else {
            Err(IpcError::ConnectionClosed)
        }
    }
}
