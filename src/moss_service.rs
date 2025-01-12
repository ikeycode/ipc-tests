//! A helper crate for launching Serpent OS tooling as privileged processes
//! while maintaining a secure IPC channel.
//! Specifically we avoid using multiplexing services, ensuring that a client
//! explicitly launches its own helper and is reliant on the locking semantics
//! of the helper tool.

use std::{
    env, io,
    os::{
        fd::{FromRawFd, OwnedFd, RawFd},
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixListener, UnixStream},
    },
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

pub trait SocketExecutor: Default {
    fn child_fd(&self) -> i32;
    fn parent_fd(&self) -> i32;
    fn command(&self, executable: &str, args: &[&str]) -> Command;
}

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

/// A unique identifier for an address.
struct AddressIdentifier(uuid::Uuid);

/// A connection to a privileged service.
pub struct ServiceConnection {
    pub socket: UnixStream,
    _child: Pid,
}

impl ServiceConnection {
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

/// An activated service listener.
pub struct ServiceListener {
    pub listener: UnixListener,
    pub socket: UnixStream,
}

impl ServiceListener {
    pub fn new() -> io::Result<Self> {
        let server_fd: RawFd = match env::var_os("PKEXEC_UID") {
            Some(_) => PkexecExecutor {}.parent_fd(),
            None => DirectExecutor {}.parent_fd(),
        };
        let listener = unsafe { UnixListener::from(OwnedFd::from_raw_fd(server_fd)) };
        let (socket, _) = listener.accept()?;
        log::trace!("🔌 accepted client connection");
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

// Handle service initialization by redirecting stderr to stdout when invoked
// by pkexec.
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
