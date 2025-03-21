use std::sync::{Arc, Mutex};

use tokio::io;

use crate::{
    messaging::Message,
    setup::{hostsfile::PeerList, socketry::Nexus},
};
mod paxos;

pub struct Data {
    peer_list: PeerList,
    nexus: Nexus,
}

impl Data {
    pub fn new(peer_list: PeerList, nexus: Nexus) -> Self {
        Self { peer_list, nexus }
    }

    pub async fn send_msg(&mut self, msg: Message) -> io::Result<()> {
        self.nexus
            .send_letter(
                (
                    self.peer_list.id(),
                    msg,
                    self.peer_list
                        .ids_and_names()
                        .next()
                        .map(|(id, _)| id)
                        .unwrap(),
                )
                    .into(),
            )
            .await?;
        Ok(())
    }

    pub fn tick(&mut self) {
        if let Some(letter) = self.nexus.check_mailbox() {
            println!("GOT ONE! {letter:?}");
        }
    }
}
