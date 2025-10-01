use std::{
    net::{SocketAddr, ToSocketAddrs},
    time::{Duration, Instant},
};

use ddb_lib::{Id, Message, Network};

use crate::{data::Data, identification::Identification};

static UPKEEP_INTERVAL: Duration = Duration::from_secs(15);

pub struct Node {
    id: Id,
    network: Network,
    data: Data,
    identification: Identification,
}

impl Node {
    pub fn new<A: ToSocketAddrs>(id: Id, addrs: A) -> Option<Self> {
        Some(Self {
            id: id.clone(),
            network: Network::new(addrs, id.clone())?,
            data: Data::new(),
            identification: Identification::new(id),
        })
    }

    pub fn run(mut self) {
        self.network.set_read_timeout(Some(Duration::from_secs(1)));
        let mut last_upkeep = Instant::now();
        loop {
            let msg = self.network.listen();
            if let Some((from, msg)) = msg {
                self.process_msg(from, msg)
            }

            if last_upkeep + UPKEEP_INTERVAL < Instant::now() {
                last_upkeep = Instant::now();
                self.upkeep();
            }
        }
    }

    fn process_msg(&mut self, from: SocketAddr, msg: Message) {
        println!("Got message {:?}", msg);
        // do some processing
        let msg_id = msg.from().clone();
        match msg.take_msg_type() {
            ddb_lib::MessageType::Verify(challenge) => {
                // another node wants to contact us, reply with challenge
                self.network.verify(&from, challenge);
            }
            ddb_lib::MessageType::Verified(challenge) => {
                // our challenge has succeeded
                self.network.verified(&challenge);
            }
            ddb_lib::MessageType::Get { key, count } => {
                let entries = self.data.get(&key, count);
                self.network
                    .send(from, Message::values(self.id.clone(), entries));
            }
            ddb_lib::MessageType::Values(mut entries) => {
                // discard duplicates and distrusted messages
                entries.retain(|entry| {
                    !self.identification.is_distrusted(&entry.id) && !self.data.contains(entry)
                });

                // if all the messages are filtered out, no need to continue
                if entries.is_empty() {
                    return;
                }

                // forward these messages across the network
                self.network
                    .send_several(Message::values(self.id, entries.clone()));

                // retain only trusted messages
                entries.retain(|entry| self.identification.is_trusted(&entry.id));
                // store trusted messages
                self.data.ingest(entries);
            }
            ddb_lib::MessageType::Set(mut entry) => {
                if self.identification.is_us(&msg_id) {
                    // calculate proper next id
                    let next_seq = self.data.get_next_id(&entry.key);
                    entry.seq = next_seq;
                    self.data.insert(entry.clone());

                    // rebroadcast
                    self.network
                        .send_several(Message::values(self.id, vec![entry]));
                }
            }
            ddb_lib::MessageType::Link(addr) => {
                if self.identification.is_us(&msg_id) {
                    if let Ok(addr) = addr.parse() {
                        self.network.request_verification(self.id, addr);
                    }
                }
            }
            ddb_lib::MessageType::Neighbors(neighbors) => {
                // TODO: try to connect to some of the neighbors
            }
            ddb_lib::MessageType::Trust(id, delta) => {
                if self.identification.is_us(&msg_id) {
                    self.identification.change_trust(id, delta as f32 / 10000.0);
                }
            }
        };
    }

    /// Periodic functions to maintain the health of the network
    fn upkeep(&mut self) {
        println!("upkeep!");

        // let network clean up its old items
        self.network.clean();

        // prepare a list of neighbors to send
        self.network.swap_neighbors();
    }
}
