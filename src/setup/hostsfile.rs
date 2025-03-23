use indexmap::IndexMap;
use nom::{
    bytes::complete::{tag, take_until},
    character::{complete::alpha1, complete::digit1},
    combinator::map_res,
    multi::separated_list1,
    sequence::separated_pair,
    IResult, Parser,
};
use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    hash::Hash,
    io::Read,
    path::PathBuf,
};

use crate::{
    messaging::dist_types::PeerId,
    state::paxos::{Accepting, Learning, PaxosRole, PaxosStage, Proposing},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Proposer(u32),
    Acceptor(u32),
    Learner(u32),
}
fn parse_roleid(input: &str) -> IResult<&str, Role> {
    let (input, role_type) = alpha1(input)?;
    let (input, id) = map_res(digit1, |s: &str| s.parse::<u32>()).parse(input)?;

    let role = match role_type {
        "proposer" => Role::Proposer,
        "acceptor" => Role::Acceptor,
        "learner" => Role::Learner,
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
    }(id);

    Ok((input, role))
}

fn parse_roles(input: &str) -> IResult<&str, VecDeque<Role>> {
    separated_list1(tag(","), parse_roleid)
        .parse(input)
        .map(|(input, vec)| (input, vec.into()))
}
fn make_roles<'a>(input: &'a str) -> IResult<&'a str, IndexMap<String, VecDeque<Role>>> {
    let mut key_values = separated_pair(take_until(":"), tag(":"), parse_roles);
    let mut out = IndexMap::new();
    for line in input.lines() {
        let (_input, (peer_name, roles)) = key_values.parse(line)?;
        out.insert(peer_name.into(), roles);
    }
    Ok((input, out))
}

/// Helper to keep track of whos who
#[derive(Debug)]
pub struct PeerList {
    peer_names: IndexMap<String, VecDeque<Role>>,
    hostname: String,
}
impl PeerList {
    pub fn load(path: PathBuf) -> std::io::Result<Self> {
        let hostname = hostname::get()?.into_string().expect("Converted");
        let peer_names = File::open(path).map(|mut f| {
            let mut names = String::new();
            let _bytes = f.read_to_string(&mut names).expect("Can read hostsfile");
            let (_, pn) = make_roles(&names).expect("Valid lists");
            pn
        })?;

        Ok(PeerList {
            peer_names,
            hostname,
        })
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn id(&self) -> PeerId {
        self.peer_names
            .keys()
            .position(|v| *v == self.hostname)
            .expect("Host should be in hostsfile")
            + 1
    }

    /// Returns an iterator going to all of this process' acceptors
    pub fn acceptors(&self, num: PaxosStage) -> Vec<PeerId> {
        // welcome to ITER CHAIN HELL! 🔥💀🔥
        self.peer_names
            .iter()
            .enumerate()
            .filter_map(move |(index, (_, roles))| {
                if roles.contains(&Role::Acceptor(num)) {
                    Some(index + 1)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns iterator of all peer Ids and their names
    pub fn ids_and_names(&self) -> impl Iterator<Item = (PeerId, &String)> {
        self.peer_names
            .keys()
            .enumerate()
            .map(|(index, name)| (index + 1, name))
            .filter(|(_, name)| **name != self.hostname)
    }

    /// Returns the number of peers this one is connected to
    pub fn peers_count(&self) -> usize {
        self.peer_names.len() - 1
    }

    pub fn establish_stages(&self) -> HashMap<PaxosStage, PaxosRole> {
        self.peer_names
            .get(&self.hostname)
            .expect("Host in hostsfile")
            .iter()
            .map(|r| match r {
                Role::Proposer(stage_num) => (
                    *stage_num,
                    PaxosRole::Prop(Proposing::new(self.acceptors(*stage_num).len())),
                ),
                Role::Acceptor(stage_num) => (*stage_num, PaxosRole::Acc(Accepting::default())),
                Role::Learner(stage_num) => (*stage_num, PaxosRole::Learn(Learning)),
            })
            .collect()
    }

    pub fn proposer(&self, num: PaxosStage) -> PeerId {
        self.ids_and_names()
            .find_map(|(id, name)| {
                self.peer_names
                    .get(name)
                    .and_then(|roles| roles.contains(&Role::Proposer(num)).then_some(id))
            })
            .expect("Should have a proposer")
    }

    pub fn acceptors_and_learners(&self, num: PaxosStage) -> Vec<PeerId> {
        self.peer_names
            .iter()
            .enumerate()
            .filter_map(move |(index, (_, roles))| {
                if roles.contains(&Role::Acceptor(num)) || roles.contains(&Role::Learner(num)) {
                    Some(index + 1)
                } else {
                    None
                }
            })
            .collect()
    }
}
