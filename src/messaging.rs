// Types as per the distributed system
pub mod dist_types {
    pub type PeerId = usize;
}

use dist_types::PeerId;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
};

use crate::state::paxos::{Proposal, ProposalNum};

// Type of message being sent
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    // Sent by a peer once it establishes a connection
    Alive,
    Prepare(Proposal),
    PrepareAck(Option<Proposal>),
    Accept(Proposal),
    AcceptAck { min_proposal: ProposalNum },
}

// Message with an address
#[derive(Serialize, Deserialize, Debug)]
pub struct Letter {
    from: PeerId,
    contents: Message,
    to: PeerId,
}
impl Letter {
    pub async fn send(&self, sender: &mut TcpStream) -> io::Result<()> {
        let buffer = bincode::serialize(self).expect("Message is serializable");
        sender.write_all(&buffer).await?;
        Ok(())
    }

    pub fn message(&self) -> &Message {
        &self.contents
    }

    pub fn from(&self) -> PeerId {
        self.from
    }

    pub fn to(&self) -> PeerId {
        self.to
    }
}

impl From<(PeerId, Message, PeerId)> for Letter {
    fn from(value: (PeerId, Message, PeerId)) -> Self {
        Self {
            from: value.0,
            contents: value.1,
            to: value.2,
        }
    }
}
