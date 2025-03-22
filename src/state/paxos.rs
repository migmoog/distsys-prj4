use serde::{Deserialize, Serialize};

use crate::messaging::Message;

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
}
impl Proposing {
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

    /// Returns true if we have proposed a value
    pub fn has_begun(&self) -> bool {
        self.value.is_some()
    }
}

#[derive(Default)]
pub struct Accepting {
    min_proposal: Option<ProposalNum>,
    accepted_proposal: Option<(ProposalNum, Value)>,
}

pub struct Learning;

pub enum PaxosRole {
    Prop(Proposing),
    Acc(Accepting),
    Learn(Learning),
}
