use std::{collections::VecDeque, thread::sleep, time::Duration};

use paxos::{Chooser, PaxosRole, Value};
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
    role: PaxosRole,
    // Message to send and who its going to
    log: VecDeque<(Message, Vec<PeerId>)>,
}

impl Data {
    pub fn new(peer_list: PeerList, nexus: Nexus) -> Self {
        let role = peer_list.paxos_role();
        Self {
            peer_list,
            nexus,
            role,
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
        if let PaxosRole::Prop(ref p) = self.role {
            !p.has_begun()
        } else {
            false
        }
    }

    /// Broadcasts a prepare message to all acceptors
    pub async fn propose(&mut self, v: Value) -> io::Result<()> {
        let mut to_send = None;
        if let PaxosRole::Prop(ref mut p) = self.role {
            let msg = p.propose(v);
            msg.paxos_print(self.peer_list.id(), true, &p.current_prop());
            to_send = Some((msg, p.stage));
        }

        if let Some((msg, stage)) = to_send {
            for id in self.peer_list.acceptors(stage) {
                self.send_msg(msg.clone(), id).await?;
            }
        }
        Ok(())
    }

    // checks the mailbox and does according data trickery
    pub fn tick(&mut self) {
        let Some(letter) = self.nexus.check_mailbox() else {
            return;
        };

        let id = self.peer_list.id();
        let recmsg = letter.message();
        match (recmsg, &mut self.role) {
            (Message::Prepare(prop), PaxosRole::Acc(ref mut acc)) => {
                recmsg.paxos_print(id, false, prop);

                let msg = acc.prepare(prop, id);
                self.log.push_back((msg, vec![letter.from()]));
            }

            (Message::PrepareAck(response), PaxosRole::Prop(ref mut prop)) => {
                recmsg.paxos_print(id, false, &prop.current_prop());

                if let Some(msg) = prop.acknowledge_prep(letter.from(), response.clone(), id) {
                    self.log
                        .push_back((msg, self.peer_list.acceptors(prop.stage)));
                }
            }

            (Message::Accept(prop), PaxosRole::Acc(ref mut acceptor)) => {
                recmsg.paxos_print(id, false, prop);
                let msg = acceptor.accept(prop, id);
                self.log.push_back((msg, vec![letter.from()]));
            }

            (Message::AcceptAck { min_proposal }, PaxosRole::Prop(ref mut proposer)) => {
                recmsg.paxos_print(id, false, &proposer.current_prop());
                if let Some(msg) = proposer.acknowledge_accept(letter.from(), *min_proposal, id) {
                    self.log.push_back((
                        msg,
                        self.peer_list.ids_and_names().map(|(id, _)| id).collect(),
                    ));
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
        let Some((msg, to_peers)) = self.log.pop_front() else {
            return Ok(());
        };

        for id in to_peers {
            self.send_msg(msg.clone(), id).await?;
        }

        Ok(())
    }
}
