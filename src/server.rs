use std::io::{BufReader, Write};

use nix::unistd::getuid;

use crate::{
    api::{Package, RecvyMessage, SendyMessage},
    moss_service::ServiceListener,
};

/// Run the server component
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🚀 Starting server...");
    let mut svs = ServiceListener::new()?;

    let mut buf = BufReader::new(svs.socket.try_clone()?);

    for message in serde_json::Deserializer::from_reader(&mut buf).into_iter::<SendyMessage>() {
        match message {
            Ok(SendyMessage::DoThings(i)) => {
                log::info!("📬 Received: {:?}", i);
                let reply = RecvyMessage::GotThings(format!("I got your message: {}", i));
                serde_json::to_writer(&svs.socket, &reply)?;
            }
            Ok(SendyMessage::ListThePackages) => {
                log::info!("📦 Received: ListThePackages");
                Package::get_sample_packages()
                    .into_iter()
                    .map(RecvyMessage::HereIsOnePackage)
                    .for_each(|reply| {
                        serde_json::to_writer(&svs.socket, &reply).unwrap();
                    });
            }
            Ok(SendyMessage::WhatsYourUID) => {
                log::info!("🔑 Received: WhatsYourUID");
                let reply = RecvyMessage::HereIsYourUID(getuid().into());
                serde_json::to_writer(&svs.socket, &reply)?;
            }
            Err(e) => {
                log::error!("💥 Error: {:?}", e);
            }
        }
        svs.socket.flush()?;
    }

    svs.socket.shutdown(std::net::Shutdown::Both)?;

    Ok(())
}
