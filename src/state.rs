use std::sync::{Arc, Mutex};

use crate::setup::{hostsfile::PeerList, socketry::Nexus};

pub struct Data {
    peer_list: PeerList,
    nexus: Arc<Mutex<Nexus>>,
}

impl Data {
    pub fn new(peer_list: PeerList, nexus: Nexus) -> Self {
        Self {
            peer_list,
            nexus: Arc::new(Mutex::new(nexus)),
        }
    }
}
