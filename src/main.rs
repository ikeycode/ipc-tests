use std::io::{BufReader, Write};

use clap::Parser;

use ipc_tests::moss_service::{self, ServiceConnection, ServiceListener};
use nix::unistd::getuid;
use serde_derive::{Deserialize, Serialize};

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(long)]
    server: bool,
}

#[derive(Serialize, Deserialize, Debug)]
enum SendyMessage {
    DoThings(i8),
    ListThePackages,
    WhatsYourUID,
}

#[derive(Serialize, Deserialize, Debug)]
enum RecvyMessage {
    GotThings(String),
    HereIsOnePackage(String),
    HereIsYourUID(u32),
}

fn server_runner() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running server");
    let mut svs = ServiceListener::new()?;

    let mut buf = BufReader::new(svs.socket.try_clone()?);

    for message in serde_json::Deserializer::from_reader(&mut buf).into_iter::<SendyMessage>() {
        match message {
            Ok(SendyMessage::DoThings(i)) => {
                println!(">>> Server received: {:?}", i);
                let reply = RecvyMessage::GotThings(format!("I got your message: {}", i));
                serde_json::to_writer(&svs.socket, &reply)?;
            }
            Ok(SendyMessage::ListThePackages) => {
                println!(">>> Server received: ListThePackages");
                for package in &["firefox", "chromium", "libreoffice"] {
                    let reply = RecvyMessage::HereIsOnePackage(package.to_string());
                    serde_json::to_writer(&svs.socket, &reply)?;
                }
            }
            Ok(SendyMessage::WhatsYourUID) => {
                println!(">>> Server received: WhatsYourUID");
                let reply = RecvyMessage::HereIsYourUID(getuid().into());
                serde_json::to_writer(&svs.socket, &reply)?;
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
        svs.socket.flush()?;
    }

    svs.socket.shutdown(std::net::Shutdown::Both)?;

    Ok(())
}

fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    let ourselves = std::env::current_exe()?.to_string_lossy().to_string();
    let mut conn = ServiceConnection::new(&ourselves, &["--server"])?;

    let message = SendyMessage::DoThings(42);
    serde_json::to_writer(&conn.socket, &message)?;
    serde_json::to_writer(&conn.socket, &SendyMessage::ListThePackages)?;
    serde_json::to_writer(&conn.socket, &SendyMessage::WhatsYourUID)?;
    conn.socket.flush()?;
    conn.socket.shutdown(std::net::Shutdown::Write)?;

    let mut buf = BufReader::new(&conn.socket);
    for message in serde_json::Deserializer::from_reader(&mut buf).into_iter::<RecvyMessage>() {
        match message {
            Ok(RecvyMessage::GotThings(s)) => {
                println!("<<< Client received: `{:?}`", s);
            }
            Ok(RecvyMessage::HereIsOnePackage(s)) => {
                println!("<<< Client received package: `{:?}`", s);
            }
            Ok(RecvyMessage::HereIsYourUID(uid)) => {
                println!("<<< Client received UID: `{:?}`", uid);
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
    }

    conn.socket.shutdown(std::net::Shutdown::Read)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.server {
        let log_path = "/dev/null";
        let _log_file = moss_service::service_init(log_path)?;
        server_runner()?;
    } else {
        run_client()?;
    }

    Ok(())
}
