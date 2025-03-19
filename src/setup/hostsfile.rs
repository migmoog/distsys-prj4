use indexmap::IndexMap;
use nom::{
    bytes::complete::{tag, take_until},
    character::{complete::alpha1, complete::digit1},
    combinator::map_res,
    multi::separated_list1,
    sequence::separated_pair,
    IResult, Parser,
};
use std::{collections::HashMap, fs::File, io::Read, ops::Index, path::PathBuf};

use crate::messaging::dist_types::PeerId;

#[derive(Debug)]
pub enum Role {
    Proposer(u32),
    Acceptor(u32),
    Learner(u32),
}
impl Role {
    pub fn parse(input: &str) -> IResult<&str, Self> {
        let (input, role_type) = alpha1(input)?;
        let (input, id) = map_res(digit1, |s: &str| s.parse::<u32>()).parse(input)?;

        let role = match role_type {
            "proposer" => Self::Proposer,
            "acceptor" => Self::Acceptor,
            "learner" => Self::Learner,
            _ => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
        }(id);

        Ok((input, role))
    }
}
fn parse_roles(input: &str) -> IResult<&str, Vec<Role>> {
    separated_list1(tag(","), Role::parse).parse(input)
}
fn make_roles<'a>(input: &'a str) -> IResult<&'a str, IndexMap<String, Vec<Role>>> {
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
    peer_names: IndexMap<String, Vec<Role>>,
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
    pub fn ids_and_names(&self) -> impl Iterator<Item = (PeerId, &String)> {
        self.peer_names
            .keys()
            .filter(|name| **name != self.hostname)
            .enumerate()
            .map(|(index, name)| (index + 1, name))
    }

    pub fn peers_count(&self) -> usize {
        self.peer_names.len() - 1
    }
}
