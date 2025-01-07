//! A helper crate for launching Serpent OS tooling as privileged processes
//! while maintaining a secure IPC channel.
//! Specifically we avoid using multiplexing services, ensuring that a client
//! explicitly launches its own helper and is reliant on the locking semantics
//! of the helper tool.

use std::{
    fs::File,
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixListener, UnixStream},
    },
    path::Path,
    process::{Child, Command},
};

use command_fds::{CommandFdExt, FdMapping, FdMappingCollision};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to spawn privileged worker: {0}")]
    IO(#[from] io::Error),

    #[error("mapping collision@ {0}")]
    MappingCollision(#[from] FdMappingCollision),
}

#[non_exhaustive]
pub struct ServiceFds;

impl ServiceFds {
    pub const CHILD_CONTEXT: i32 = 2;
    pub const PARENT_CONTEXT: i32 = 3;
}

/// A unique identifier for an address.
struct AddressIdentifier(uuid::Uuid);

/// A connection to a privileged service.
pub struct ServiceConnection {
    pub child: Child,
    pub socket: UnixStream,
    _command: Command,
}

impl ServiceConnection {
    pub fn new(executable: &str, args: &[&str]) -> Result<Self, self::Error> {
        let identity = AddressIdentifier::default();
        let socket_addr = identity.as_unix_address()?;
        let unix_socket = UnixListener::bind_addr(&socket_addr)?;

        let mappings: Vec<FdMapping> = vec![FdMapping {
            parent_fd: unix_socket.into(),
            child_fd: ServiceFds::CHILD_CONTEXT,
        }];
        let mut command = Command::new("pkexec");
        command.arg(executable);
        command.args(args);
        command.fd_mappings(mappings)?;
        let child = command.spawn()?;

        let socket = UnixStream::connect_addr(&socket_addr)?;

        Ok(ServiceConnection {
            child,
            socket,
            _command: command,
        })
    }
}

/// An activated service listener.
pub struct ServiceListener {
    pub listener: UnixListener,
    pub socket: UnixStream,
}

impl ServiceListener {
    pub fn new() -> io::Result<Self> {
        let server_fd: RawFd = ServiceFds::PARENT_CONTEXT;
        let listener = unsafe { UnixListener::from(OwnedFd::from_raw_fd(server_fd)) };
        let (socket, _) = listener.accept()?;

        Ok(Self { listener, socket })
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

pub fn service_init(stderr: impl AsRef<Path>) -> io::Result<File> {
    nix::unistd::dup2(ServiceFds::CHILD_CONTEXT, ServiceFds::PARENT_CONTEXT)?;
    nix::unistd::close(ServiceFds::CHILD_CONTEXT)?;
    let file = File::create(stderr.as_ref())?;
    nix::unistd::dup2(file.as_raw_fd(), ServiceFds::CHILD_CONTEXT)?;
    Ok(file)
}
