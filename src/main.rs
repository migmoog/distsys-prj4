use std::{thread::sleep, time::Duration};

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
    let arguments = Project4::parse();

    // hostsfile reader that can give us information about peers
    let peer_list = PeerList::load(arguments.hostsfile)?;
    // collection of the incoming and outgoing channels to peers
    let nexus = Nexus::new(&peer_list).await;
    // Add this sleep to allow other peers in the system to finish setting up
    sleep(Duration::from_secs(2));

    let mut data = Data::new(peer_list, nexus);
    loop {
        if let Some(value) = arguments.proposal_value {
            if data.can_propose() {
                if let Some(secs) = arguments.proposal_delay {
                    sleep(Duration::from_secs(secs));
                }
                data.propose(value).await?;
            }
        }

        data.tick();

        data.flush_log().await?;
    }
}
