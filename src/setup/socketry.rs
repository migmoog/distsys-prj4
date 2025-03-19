use std::{
    collections::HashMap, future::Future, net::ToSocketAddrs, thread::sleep, time::Duration,
};

use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

use crate::messaging::{dist_types::PeerId, Letter, Message};

use super::hostsfile::PeerList;
const TCP_PORT: &str = "6969";

// Responsible for managing all receiving and sending of messages
pub struct Nexus {
    // Streams created by a TcpListener
    incoming: HashMap<PeerId, TcpStream>,
    // Streams made by TcpStream::connect
    outgoing: HashMap<PeerId, TcpStream>,
}

async fn attempt_op<F, Fut, Socket>(op: F, host: &str) -> Socket
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = std::io::Result<Socket>>,
{
    let addr = format!("{}:{}", host, TCP_PORT);
    loop {
        match op(addr.clone()).await {
            Ok(s) => break s,
            Err(_) => sleep(Duration::from_secs(2)),
        }
    }
}
impl Nexus {
    pub async fn new(list: &PeerList) -> Self {
        let mut outgoing = HashMap::new();

        // set up incoming channels
        let listener = attempt_op(TcpListener::bind, list.hostname()).await;
        // connect to other peers
        for (id, peer_name) in list.ids_and_names() {
            // just block until we connect
            let mut sock = attempt_op(TcpStream::connect, peer_name).await;

            // send an "I am alive message"
            let i_am_alive: Letter = (list.id(), Message::Alive, id).into();
            i_am_alive.send(&mut sock).await.expect("Successful send");

            outgoing.insert(id, sock);
        }

        // accept connections
        let mut anon_socks = Vec::new();
        while anon_socks.len() < list.peers_count() {
            match listener.accept().await {
                Ok((stream, _)) => {
                    anon_socks.push(stream);
                }
                Err(_) => continue,
            }
        }

        // organize these anonymous socks based on their alive messages
        let mut incoming = HashMap::new();
        for mut sock in anon_socks {
            let mut buf = [0; 1024];
            if let Ok(bytes_read) = sock.read(&mut buf).await {
                let letter: Letter =
                    bincode::deserialize(&buf[..bytes_read]).expect("Deserializeable");
                if matches!(letter.message(), Message::Alive) {
                    incoming.insert(letter.from(), sock);
                }
            }
        }

        // waiting for "I am alive messages"
        Self { incoming, outgoing }
    }
}
