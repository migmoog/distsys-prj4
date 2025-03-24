use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::messaging::{dist_types::PeerId, Message};

/// Passed Between proposors and acceptors. Just a monotonically increasing integer
pub type ProposalNum = u64;
/// Chars that represent accepted values
pub type Value = char;

/// As per the hostsfile, this is a ket referencing
/// which round of the testcases we are in
pub type PaxosStage = u32;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Proposal {
    pub num: ProposalNum,
    pub value: Value,
}

#[derive(Default)]
pub struct Proposing {
    num: ProposalNum,
    value: Option<Value>,
    prep_acks: HashMap<PeerId, Option<Proposal>>,
    broadcasted_accept: bool,
    accept_acks: HashMap<PeerId, ProposalNum>,
    quorum_size: usize,
    chosen: bool,
    pub stage: PaxosStage,
}
impl Proposing {
    pub fn new(quorum_size: usize, stage: PaxosStage) -> Self {
        Self {
            quorum_size,
            broadcasted_accept: false,
            chosen: false,
            stage,
            ..Default::default()
        }
    }
    /// Returns true if we have proposed a value
    pub fn has_begun(&self) -> bool {
        self.value.is_some()
    }

    pub fn current_prop(&self) -> Proposal {
        Proposal {
            num: self.num,
            value: self.value.expect("Dont mess this up"),
        }
    }

    /// Sets proposal value and increments proposal number
    pub fn propose(&mut self, v: Value) -> Message {
        assert!(!self.has_begun());
        self.num += 1;
        self.value = Some(v);

        Message::Prepare(Proposal {
            num: self.num,
            value: v,
        })
    }

    /// Acknowledges a prepare_ack from an acceptor. Will return a message
    /// If it receives a majority of responses and any accepted values,
    /// It will change its value
    pub fn acknowledge_prep(
        &mut self,
        from: PeerId,
        response: Option<Proposal>,
        id: PeerId,
    ) -> Option<Message> {
        self.prep_acks.insert(from, response);

        // if we receive from a majority
        if self.prep_acks.len() > self.quorum_size / 2 && !self.broadcasted_accept {
            if let Some(highest_prop) = self
                .prep_acks
                .values()
                .filter_map(|o| o.as_ref())
                .max_by_key(|p| p.num)
            {
                self.num = highest_prop.num;
                self.value = Some(highest_prop.value);
            }

            self.broadcasted_accept = true;
            let accept_msg = Message::Accept(Proposal {
                num: self.num,
                value: self.value?,
            });
            accept_msg.paxos_print(id, true, &self.current_prop());
            Some(accept_msg)
        } else {
            None
        }
    }

    pub fn acknowledge_accept(
        &mut self,
        from: PeerId,
        min_proposal: ProposalNum,
        id: PeerId,
    ) -> Option<Message> {
        if self.chosen {
            return None;
        }
        self.accept_acks.insert(from, min_proposal);

        // we got a rejection, abort!
        if min_proposal > self.num {
            self.num = min_proposal + 1;
            let redo_prep = Message::Prepare(Proposal {
                num: self.num,
                value: self.value?,
            });
            redo_prep.paxos_print(id, true, &self.current_prop());
            return Some(redo_prep);
        }

        if self.accept_acks.len() > self.quorum_size / 2 && !self.chosen {
            self.chosen = true;
            let chose_msg = Message::Chosen(Proposal {
                num: self.num,
                value: self.value?,
            });
            chose_msg.paxos_print(id, true, &self.current_prop());

            Some(chose_msg)
        } else {
            None
        }
    }
}

pub trait Chooser {
    fn accept_choice(&mut self, prop: &Proposal);
}

#[derive(Default)]
pub struct Accepting {
    min_proposal: ProposalNum,
    accepted_prop: Option<Proposal>,
}
impl Accepting {
    pub fn prepare(&mut self, prop: &Proposal, id: PeerId) -> Message {
        // if n > minProposal then minProposal = n
        self.min_proposal = self.min_proposal.max(prop.num);
        let msg = Message::PrepareAck(self.accepted_prop.clone());
        msg.paxos_print(id, true, prop);
        msg
    }

    pub fn accept(&mut self, prop: &Proposal, id: PeerId) -> Message {
        if prop.num > self.min_proposal {
            self.min_proposal = prop.num;
            self.accepted_prop = Some(prop.clone());
        }
        let msg = Message::AcceptAck {
            min_proposal: self.min_proposal,
        };

        msg.paxos_print(id, true, prop);
        msg
    }
}
impl Chooser for Accepting {
    fn accept_choice(&mut self, _prop: &Proposal) {
        println!("Also chosen");
    }
}

pub struct Learning;
impl Chooser for Learning {
    fn accept_choice(&mut self, _prop: &Proposal) {
        println!("Chosen");
    }
}

pub enum PaxosRole {
    Prop(Proposing),
    Acc(Accepting),
    Learn(Learning),
}
impl Chooser for PaxosRole {
    fn accept_choice(&mut self, prop: &Proposal) {
        match self {
            Self::Acc(a) => a.accept_choice(prop),
            Self::Learn(l) => l.accept_choice(prop),
            _ => {}
        }
    }
}
