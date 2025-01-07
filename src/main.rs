use std::io::{Read, Write};

use clap::Parser;

use ipc_tests::moss_service::{self, ServiceConnection, ServiceListener};

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(long)]
    server: bool,
}

fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running server");
    let mut server = ServiceListener::new()?;
    println!("Im running as user: {}", nix::unistd::getuid());

    let mut buf = vec![];
    let n = server.socket.read_to_end(&mut buf)?;
    let client_sz = std::str::from_utf8(&buf[..n])?;
    println!(">> server has Received: {client_sz}",);
    eprintln!("from client:: {client_sz}");
    server.socket.write_all(b"hey jackass\n")?;
    server.socket.flush()?;
    server.socket.shutdown(std::net::Shutdown::Both)?;

    eprintln!(">> Ending server");

    Ok(())
}

fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    let ourselves = std::env::current_exe()?.to_string_lossy().to_string();
    let mut conn = ServiceConnection::new(&ourselves, &["--server"])?;
    conn.socket.write_all(b"hello server!")?;
    conn.socket.flush()?;
    conn.socket.shutdown(std::net::Shutdown::Write)?;
    let mut buf = vec![];
    conn.socket.read_to_end(&mut buf)?;
    println!("client Received: {}", std::str::from_utf8(&buf)?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.server {
        let log_path = "/dev/null";
        let _log_file = moss_service::service_init(log_path)?;
        run_server()
    } else {
        run_client()
    }
}
