use std::collections::{HashMap, VecDeque};

use paxos::{Chooser, PaxosRole, PaxosStage, Value};
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
    log: VecDeque<Message>,
}

impl Data {
    pub fn new(peer_list: PeerList, nexus: Nexus) -> Self {
        let stages = peer_list.establish_stages();
        Self {
            peer_list,
            nexus,
            current_stage: 1,
            stages,
            log: VecDeque::new(),
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
            let msg = p.propose(v);
            msg.paxos_print(self.peer_list.id(), true, &p.current_prop());
            to_send = Some(msg);
        }

        if let Some(msg) = to_send {
            for id in self.peer_list.acceptors(self.current_stage) {
                self.send_msg(msg.clone(), id).await?;
            }
        }
        Ok(())
    }

    // checks the mailbox and does according data trickery
    pub fn tick(&mut self) {
        let (Some(letter), Some(paxos_role)) = (
            self.nexus.check_mailbox(),
            self.stages.get_mut(&self.current_stage),
        ) else {
            return;
        };

        let id = self.peer_list.id();
        let recmsg = letter.message();
        match (recmsg, paxos_role) {
            (Message::Prepare(prop), PaxosRole::Acc(ref mut acc)) => {
                recmsg.paxos_print(id, false, prop);

                let msg = acc.prepare(prop, id);
                self.log.push_back(msg);
            }

            (Message::PrepareAck(response), PaxosRole::Prop(ref mut prop)) => {
                recmsg.paxos_print(id, false, &prop.current_prop());

                if let Some(msg) = prop.acknowledge_prep(letter.from(), response.clone(), id) {
                    self.log.push_back(msg);
                }
            }

            (Message::Accept(prop), PaxosRole::Acc(ref mut acceptor)) => {
                recmsg.paxos_print(id, false, prop);
                let msg = acceptor.accept(prop, id);
                self.log.push_back(msg);
            }

            (Message::AcceptAck { min_proposal }, PaxosRole::Prop(ref mut proposer)) => {
                recmsg.paxos_print(id, false, &proposer.current_prop());
                if let Some(msg) = proposer.acknowledge_accept(letter.from(), *min_proposal, id) {
                    self.log.push_back(msg);
                }
            }

            (Message::Chosen(prop), role) => {
                recmsg.paxos_print(id, false, prop);
                role.accept_choice(prop);
            }
            _ => unreachable!("These messages should only be sent by their accompanying roles"),
        }
    }

    pub async fn flush_log(&mut self) -> io::Result<()> {
        let Some(msg) = self.log.pop_front() else {
            return Ok(());
        };

        match &msg {
            Message::PrepareAck(_) | Message::AcceptAck { .. } => {
                self.send_msg(msg, self.peer_list.proposer(self.current_stage))
                    .await?
            }

            Message::Accept(_) => {
                for id in self.peer_list.acceptors(self.current_stage) {
                    self.send_msg(msg.clone(), id).await?;
                }
            }

            Message::Chosen(_) => {
                for id in self.peer_list.acceptors_and_learners(self.current_stage) {
                    self.send_msg(msg.clone(), id).await?;
                }
            }

            Message::Prepare(_) => {}
            _ => {}
        }
        Ok(())
    }
}
