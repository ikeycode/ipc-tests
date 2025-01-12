// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Provides facilities for privilege escalation and service management using Unix domain sockets.
//!
//! This module enables creating privileged services that can be accessed through Unix domain sockets,
//! with support for both direct execution and privilege escalation via pkexec.

use std::{
    env,
    io::{self, Write},
    net::Shutdown,
    ops::DerefMut,
    os::{
        fd::{FromRawFd, OwnedFd, RawFd},
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixListener, UnixStream},
    },
    process::Command,
};

use command_fds::{CommandFdExt, FdMapping, FdMappingCollision};
use nix::unistd::Pid;
use serde_json::de::IoRead;
use std::ops::Deref;
use thiserror::Error;

/// Errors that can occur when working with privileged services
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O error occurred while spawning the privileged worker
    #[error("Failed to spawn privileged worker: {0}")]
    IO(#[from] io::Error),

    /// A file descriptor mapping collision occurred
    #[error("mapping collision@ {0}")]
    MappingCollision(#[from] FdMappingCollision),

    /// The fork operation failed
    #[error("Failed to fork: {0}")]
    Nix(#[from] nix::Error),
}

/// Trait for types that can execute commands with socket file descriptor handling
pub trait SocketExecutor: Default {
    /// Returns the file descriptor to use for the child process
    fn child_fd(&self) -> i32;

    /// Returns the file descriptor to use for the parent process
    fn parent_fd(&self) -> i32;

    /// Creates a command with the given executable and arguments
    fn command(&self, executable: &str, args: &[&str]) -> Command;
}

/// Executor that uses pkexec for privilege escalation
#[derive(Default)]
pub struct PkexecExecutor;

impl SocketExecutor for PkexecExecutor {
    fn child_fd(&self) -> i32 {
        2
    }

    fn parent_fd(&self) -> i32 {
        3
    }

    fn command(&self, executable: &str, args: &[&str]) -> Command {
        let mut command = Command::new("pkexec");
        command.arg(executable);
        command.args(args);
        command
    }
}

/// Executor that runs commands directly without privilege escalation
#[derive(Default)]
pub struct DirectExecutor;

impl SocketExecutor for DirectExecutor {
    fn child_fd(&self) -> i32 {
        3
    }

    fn parent_fd(&self) -> i32 {
        3
    }

    fn command(&self, executable: &str, args: &[&str]) -> Command {
        let mut command = Command::new(executable);
        command.args(args);
        command
    }
}

/// A unique identifier for a socket address using a UUID
struct AddressIdentifier(uuid::Uuid);

/// A connection to a privileged service, maintaining both the socket and child process
pub struct ServiceConnection {
    /// The Unix domain socket connected to the service
    pub socket: UnixStream,
    _child: Pid,
}

impl ServiceConnection {
    /// Creates a new connection to a privileged service using the specified executor
    pub fn new<T: SocketExecutor>(executable: &str, args: &[&str]) -> Result<Self, self::Error> {
        let identity = AddressIdentifier::default();
        let socket_addr = identity.as_unix_address()?;
        let unix_socket = UnixListener::bind_addr(&socket_addr)?;

        log::trace!("🔌 setting server address to: @{:?}", identity.0);

        let exec = T::default();

        let mappings: Vec<FdMapping> = vec![FdMapping {
            parent_fd: unix_socket.into(),
            child_fd: exec.child_fd(),
        }];

        match unsafe { nix::unistd::fork() }? {
            nix::unistd::ForkResult::Parent { child } => {
                let socket = UnixStream::connect_addr(&socket_addr)?;
                Ok(Self {
                    _child: child,
                    socket,
                })
            }
            nix::unistd::ForkResult::Child => {
                // Ensure we don't leak the listener, so failed pkexec
                // will still result in the listener being closed, and the
                // client connection will fail properly.
                let mut command = exec.command(executable, args);
                command.fd_mappings(mappings)?;
                command.env_remove("PKEXEC_UID");
                let st = command.status()?;
                std::process::exit(st.code().unwrap_or(1));
            }
        }
    }
}

/// An activated service listener that accepts connections from clients
pub struct ServiceListener(pub UnixListener);

impl ServiceListener {
    /// Creates a new service listener using the appropriate executor
    pub fn new() -> io::Result<Self> {
        let server_fd: RawFd = match env::var_os("PKEXEC_UID") {
            Some(_) => PkexecExecutor {}.parent_fd(),
            None => DirectExecutor {}.parent_fd(),
        };
        let listener = unsafe { UnixListener::from(OwnedFd::from_raw_fd(server_fd)) };
        Ok(ServiceListener(listener))
    }
}

impl Deref for ServiceListener {
    type Target = UnixListener;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for AddressIdentifier {
    fn default() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl AddressIdentifier {
    #[inline]
    fn as_unix_address(&self) -> io::Result<SocketAddr> {
        SocketAddr::from_abstract_name(self.0.as_bytes())
    }
}

/// Initializes a service by handling file descriptor redirection when running under pkexec
pub fn service_init() -> io::Result<()> {
    match env::var_os("PKEXEC_UID") {
        None => Ok(()),
        Some(_) => {
            // Redirect stderr to stdout
            let exec = PkexecExecutor {};
            nix::unistd::dup2(exec.child_fd(), exec.parent_fd())?;
            nix::unistd::close(exec.child_fd())?;
            nix::unistd::dup2(1, exec.child_fd())?;
            Ok(())
        }
    }
}
/// Error types for IPC operations
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Privileged IPC error: {0}")]
    Privileged(#[from] Error),
    #[error("Connection closed")]
    ConnectionClosed,
}

/// A type-safe IPC connection for sending and receiving messages
pub struct IpcConnection<S, R> {
    connection: ServiceConnection,
    _phantom: std::marker::PhantomData<(S, R)>,
}

impl<S, R> IpcConnection<S, R>
where
    S: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    /// Creates a new IPC connection from an existing ServiceConnection
    pub fn new(connection: ServiceConnection) -> Self {
        Self {
            connection,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sends a message over the connection
    pub fn send(&mut self, message: &S) -> Result<(), IpcError> {
        match serde_json::to_writer(&self.connection.socket, message) {
            Ok(_) => {
                // Try to flush, but handle broken pipe gracefully
                match self.connection.socket.flush() {
                    Ok(_) => Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                        Err(IpcError::ConnectionClosed)
                    }
                    Err(e) => Err(IpcError::Io(e)),
                }
            }
            Err(e) if e.is_io() && e.io_error_kind() == Some(std::io::ErrorKind::BrokenPipe) => {
                Err(IpcError::ConnectionClosed)
            }
            Err(e) => Err(IpcError::Json(e)),
        }
    }

    /// Returns an iterator over incoming messages
    pub fn incoming(&mut self) -> Result<IpcMessageIterator<R>, IpcError> {
        let reader = std::io::BufReader::new(self.connection.socket.try_clone()?);
        Ok(IpcMessageIterator {
            deserializer: serde_json::Deserializer::from_reader(reader),
            eof: false,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Shuts down the connection
    pub fn shutdown(&mut self, how: Shutdown) -> Result<(), IpcError> {
        self.connection.socket.shutdown(how)?;
        Ok(())
    }
}

/// Iterator over incoming IPC messages
pub struct IpcMessageIterator<R> {
    deserializer: serde_json::Deserializer<IoRead<std::io::BufReader<UnixStream>>>,
    eof: bool,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: serde::de::DeserializeOwned> Iterator for IpcMessageIterator<R> {
    type Item = Result<R, IpcError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        match R::deserialize(&mut self.deserializer) {
            Ok(msg) => Some(Ok(msg)),
            Err(e) => {
                // Handle both EOF and broken pipe/connection reset errors
                if e.is_eof()
                    || e.io_error_kind() == Some(std::io::ErrorKind::BrokenPipe)
                    || e.io_error_kind() == Some(std::io::ErrorKind::ConnectionReset)
                    || e.io_error_kind() == Some(std::io::ErrorKind::UnexpectedEof)
                {
                    self.eof = true;
                    None
                } else {
                    Some(Err(IpcError::Json(e)))
                }
            }
        }
    }
}

/// A type-safe IPC server that listens for connections
pub struct IpcServer<S, R> {
    listener: ServiceListener,
    _phantom: std::marker::PhantomData<(S, R)>,
}

impl<S, R> IpcServer<S, R>
where
    S: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    /// Creates a new IPC server
    pub fn new() -> Result<Self, IpcError> {
        Ok(Self {
            listener: ServiceListener::new()?,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Accepts a new client connection
    pub fn accept(&self) -> Result<IpcConnection<S, R>, IpcError> {
        let (socket, _) = self.listener.accept()?;
        let connection = ServiceConnection {
            socket,
            _child: nix::unistd::Pid::from_raw(0), // No child process for server side
        };
        Ok(IpcConnection::new(connection))
    }
}

/// A type-safe IPC client that connects to a server
pub struct IpcClient<S, R> {
    connection: IpcConnection<S, R>,
    _phantom: std::marker::PhantomData<(S, R)>,
}
impl<S, R> IpcClient<S, R>
where
    S: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    /// Creates a new IPC client connection using the specified executor
    pub fn new<T: SocketExecutor>(executable: &str, args: &[&str]) -> Result<Self, IpcError> {
        let connection = ServiceConnection::new::<T>(executable, args)?;
        Ok(Self {
            connection: IpcConnection::new(connection),
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<S, R> DerefMut for IpcClient<S, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

impl<S, R> Deref for IpcClient<S, R> {
    type Target = IpcConnection<S, R>;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}
