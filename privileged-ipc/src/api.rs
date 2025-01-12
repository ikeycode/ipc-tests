use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub size: u64,
    pub installed_size: u64,
    pub arch: String,
    pub url: String,
    pub license: String,
}

impl Package {
    pub fn get_sample_packages() -> Vec<Package> {
        vec![
            Package {
                name: String::from("firefox"),
                version: String::from("120.0.1"),
                description: String::from("Mozilla Firefox web browser"),
                size: 50_000_000,
                installed_size: 200_000_000,
                arch: String::from("x86_64"),
                url: String::from("https://www.mozilla.org/firefox"),
                license: String::from("MPL-2.0"),
            },
            Package {
                name: String::from("vlc"),
                version: String::from("3.0.18"),
                description: String::from("Multi-platform multimedia player"),
                size: 40_000_000,
                installed_size: 120_000_000,
                arch: String::from("x86_64"),
                url: String::from("https://www.videolan.org/vlc"),
                license: String::from("GPL-2.0"),
            },
            Package {
                name: String::from("gimp"),
                version: String::from("2.10.34"),
                description: String::from("GNU Image Manipulation Program"),
                size: 80_000_000,
                installed_size: 300_000_000,
                arch: String::from("x86_64"),
                url: String::from("https://www.gimp.org"),
                license: String::from("GPL-3.0"),
            },
        ]
    }
}

/// Messages sent from client to server
#[derive(Serialize, Deserialize, Debug)]
pub enum SendyMessage {
    /// Do some things with the given integer value
    DoThings(i8),
    /// Request list of available packages
    ListThePackages,
    /// Query the server's UID
    WhatsYourUID,
}

/// Messages sent from server to client
#[derive(Serialize, Deserialize, Debug)]
pub enum RecvyMessage {
    /// Response with processed result string
    GotThings(String),
    /// Response with a single package name
    HereIsOnePackage(Package),
    /// Response with the server's UID
    HereIsYourUID(u32),
}
