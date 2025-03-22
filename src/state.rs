use std::collections::HashMap;

use paxos::{PaxosRole, PaxosStage, Value};
use tokio::io;

use crate::{
    messaging::{dist_types::PeerId, Message},
    setup::{hostsfile::PeerList, socketry::Nexus},
};
pub mod paxos;

pub struct Data {
    // Non paxos
    peer_list: PeerList,
    nexus: Nexus,

    // Paxos stuff
    current_stage: PaxosStage,
    stages: HashMap<PaxosStage, PaxosRole>,
}

impl Data {
    pub fn new(peer_list: PeerList, nexus: Nexus) -> Self {
        let stages = peer_list.establish_stages();
        Self {
            peer_list,
            nexus,
            current_stage: 1,
            stages,
        }
    }

    pub async fn send_msg(&mut self, msg: Message, to: PeerId) -> io::Result<()> {
        self.nexus
            .send_letter((self.peer_list.id(), msg, to).into())
            .await?;
        Ok(())
    }

    /// Returns true if we are the proposer at the current stage of the system
    pub fn can_propose(&self) -> bool {
        if let Some(PaxosRole::Prop(p)) = self.stages.get(&self.current_stage) {
            !p.has_begun()
        } else {
            false
        }
    }

    /// Broadcasts a prepare message to all acceptors
    pub async fn propose(&mut self, v: Value) -> io::Result<()> {
        let mut to_send = None;
        if let Some(PaxosRole::Prop(ref mut p)) = self.stages.get_mut(&self.current_stage) {
            to_send = Some(p.propose(v));
        }

        if let Some(msg) = to_send {
            for id in self.peer_list.acceptors(self.current_stage) {
                self.send_msg(msg.clone(), id).await?;
                println!("sent a {:?} to {id}", msg);
            }
        }
        Ok(())
    }

    // checks the mailbox and does according data trickery
    pub fn tick(&mut self) {
        if let Some(letter) = self.nexus.check_mailbox() {
            println!("{letter:?}");
            // COOL!
        }
    }
}
