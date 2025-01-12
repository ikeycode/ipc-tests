// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use serde_derive::{Deserialize, Serialize};

/// Represents a software package with metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct Package {
    /// Name of the package
    pub name: String,
    /// Version string of the package
    pub version: String,
    /// Description of what the package does
    pub description: String,
    /// Download size in bytes
    pub size: u64,
    /// Installed size in bytes
    pub installed_size: u64,
    /// Target architecture (e.g. "x86_64")
    pub arch: String,
    /// Homepage or project URL
    pub url: String,
    /// License identifier (e.g. "GPL-3.0")
    pub license: String,
}

impl Package {
    /// Returns a vector of sample package instances for testing/demo purposes
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

/// Messages that can be sent from the client to the privileged server process.
/// Uses the privileged-ipc crate to handle privilege escalation via polkit.
#[derive(Serialize, Deserialize, Debug)]
pub enum SendyMessage {
    /// Request to perform some operation with the given integer value
    DoThings(i8),
    /// Request a list of all available software packages
    ListThePackages,
    /// Query the server process's user ID to verify privilege escalation
    WhatsYourUID,
}

/// Messages that can be sent from the privileged server process back to the client
#[derive(Serialize, Deserialize, Debug)]
pub enum RecvyMessage {
    /// Response containing the result of DoThings operation
    GotThings(String),
    /// Response containing a single package's metadata
    HereIsOnePackage(Package),
    EndOfPackages,
    /// Response containing the server process's user ID (should be 0/root)
    HereIsYourUID(u32),
}
