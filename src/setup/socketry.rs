use core::panic;
use std::{collections::HashMap, future::Future, thread::sleep, time::Duration};

use tokio::io::{self, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::messaging::{dist_types::PeerId, Letter, Message};

use super::hostsfile::PeerList;
const TCP_PORT: &str = "6969";

// Responsible for managing all receiving and sending of messages
pub struct Nexus {
    rec_incoming: UnboundedReceiver<Letter>,
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

        // Check for signs of life off our connections and give their own threads for polling
        let (send, rec_incoming) = unbounded_channel();
        for mut sock in anon_socks {
            let mut buf = [0; 1024];
            if let Ok(bytes_read) = sock.read(&mut buf).await {
                let letter: Letter =
                    bincode::deserialize(&buf[..bytes_read]).expect("Deserializeable");
                // shouldnt get any other message
                assert!(matches!(letter.message(), Message::Alive));

                // once we know our peer is alive, move the stream into its own async
                // thread where you can poll it for all eternity
                let send = send.clone();
                tokio::spawn(async move {
                    let mut buffer = [0; 1024];

                    loop {
                        if let Ok(bytes_read) = sock.read(&mut buffer).await {
                            assert!(bytes_read > 0);
                            let l: Letter =
                                bincode::deserialize(&buffer[..bytes_read]).expect("Full Message");
                            send.send(l).expect("Successful send");
                        }
                    }
                });
            }
        }

        Self {
            outgoing,
            rec_incoming,
        }
    }

    /// Polls for letters
    pub fn check_mailbox(&mut self) -> Option<Letter> {
        self.rec_incoming.try_recv().ok()
    }

    pub async fn send_letter(&mut self, letter: Letter) -> io::Result<()> {
        let to = letter.to();
        if let Some(sock) = self.outgoing.get_mut(&to) {
            letter.send(sock).await?;
        } else {
            panic!("DOESNT EXIST");
        }
        Ok(())
    }
}
