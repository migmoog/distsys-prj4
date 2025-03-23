use std::collections::{HashMap, HashSet};

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
    num: ProposalNum,
    value: Value,
}

#[derive(Default)]
pub struct Proposing {
    num: ProposalNum,
    value: Option<Value>,
    prep_acks: HashMap<PeerId, Option<Proposal>>,
    broadcasted_accept: bool,
    accept_acks: HashSet<PeerId>,
    quorum_size: usize,
}
impl Proposing {
    pub fn new(quorum_size: usize) -> Self {
        Self {
            quorum_size,
            broadcasted_accept: false,
            ..Default::default()
        }
    }
    /// Returns true if we have proposed a value
    pub fn has_begun(&self) -> bool {
        self.value.is_some()
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
            Some(Message::Accept(Proposal {
                num: self.num,
                value: self.value?,
            }))
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct Accepting {
    min_proposal: ProposalNum,
    accepted_prop: Option<Proposal>,
}
impl Accepting {
    pub fn prepare(&mut self, prop: &Proposal) -> Message {
        // if n > minProposal then minProposal = n
        self.min_proposal = self.min_proposal.max(prop.num);
        Message::PrepareAck(self.accepted_prop.clone())
    }
}

pub struct Learning;

pub enum PaxosRole {
    Prop(Proposing),
    Acc(Accepting),
    Learn(Learning),
}
