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
    process::Command,
};

use command_fds::{CommandFdExt, FdMapping, FdMappingCollision};
use nix::unistd::Pid;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to spawn privileged worker: {0}")]
    IO(#[from] io::Error),

    #[error("mapping collision@ {0}")]
    MappingCollision(#[from] FdMappingCollision),

    #[error("Failed to fork: {0}")]
    Nix(#[from] nix::Error),
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
    pub socket: UnixStream,
    _child: Pid,
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

        match unsafe { nix::unistd::fork() }? {
            nix::unistd::ForkResult::Parent { child } => Ok(Self {
                _child: child,
                socket: UnixStream::connect_addr(&socket_addr)?,
            }),
            nix::unistd::ForkResult::Child => {
                // Ensure we don't leak the listener, so failed pkexec
                // will still result in the listener being closed, and the
                // client connection will fail properly.
                let mut command = Command::new("pkexec");
                command.arg(executable);
                command.args(args);
                command.fd_mappings(mappings)?;
                let st = command.status()?;
                std::process::exit(st.code().unwrap_or(1));
            }
        }
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
        let (socket, client) = listener.accept()?;
        println!("Got client connection: {client:?}");

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
    // TODO: Handle *non* pkexec redirections
    nix::unistd::dup2(ServiceFds::CHILD_CONTEXT, ServiceFds::PARENT_CONTEXT)?;
    nix::unistd::close(ServiceFds::CHILD_CONTEXT)?;
    let file = File::create(stderr.as_ref())?;
    nix::unistd::dup2(file.as_raw_fd(), ServiceFds::CHILD_CONTEXT)?;
    Ok(file)
}
