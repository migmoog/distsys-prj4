use std::error::Error;

use args::Project4;
use clap::Parser;
use setup::{hostsfile::PeerList, socketry::Nexus};
use state::Data;

mod args;
mod messaging;
mod setup;
mod state;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Project4::parse();

    // hostsfile reader that can give us information about peers
    let peer_list = PeerList::load(args.hostsfile)?;
    // collection of the incoming and outgoing channels to peers
    let nexus = Nexus::new(&peer_list).await;

    let mut data = Data::new(peer_list, nexus);
    loop {}
}
