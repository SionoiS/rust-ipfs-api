// Copyright 2017 rust-ipfs-api Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use ipfs_api::IpfsClient;
use std::io::Cursor;

// Creates an Ipfs client, and replaces the config file with the default one.
//
#[cfg_attr(feature = "with-actix", actix_rt::main)]
#[cfg_attr(any(feature = "with-hyper", feature = "with-reqwest"), tokio::main)]
async fn main() {
    tracing_subscriber::fmt::init();

    eprintln!("note: this must be run in the root of the project repository");
    eprintln!("connecting to localhost:5001...");

    let client = IpfsClient::default();
    let default_config = include_str!("default_config.json");

    match client.config_replace(Cursor::new(default_config)).await {
        Ok(_) => eprintln!("replaced config file"),
        Err(e) => eprintln!("error replacing config file: {}", e),
    }
}
