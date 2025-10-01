use std::{
    collections::{HashMap, hash_map::Entry},
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::{Duration, Instant},
};

use rand::{
    distr::{Alphabetic, SampleString},
    rng,
    seq::{IndexedRandom, SliceRandom},
};

use crate::{Id, message::Message};

const VERIFICATION_TIMEOUT: Duration = Duration::from_secs(10 * 60); // 10 mins
const CHALLENGE_TIMEOUT: Duration = Duration::from_secs(16);
const PENDING_TIMEOUT: Duration = CHALLENGE_TIMEOUT;
const REBROADCAST_TIMEOUT: Duration = Duration::from_secs(60);

/// Number of connections to try to have
///
/// More connections will be made to meet this target
const TARGET_CONNECTIONS: usize = 10;

pub struct Network {
    id: Id,
    sock: UdpSocket,
    // keep list of verified addrs (verified addrs have replied with their key to prevent reflection attacks)
    // bool is if this addr is considered a neighbor
    verified_addrs: HashMap<SocketAddr, (Instant, bool)>,
    challenges: HashMap<String, SocketAddr>,
    pending: HashMap<SocketAddr, Vec<(Message, Instant)>>,
    recent_broadcasts: HashMap<Message, Instant>,
}

impl Network {
    pub fn new<A: ToSocketAddrs>(addrs: A, id: Id) -> Option<Self> {
        Some(Self {
            id,
            sock: UdpSocket::bind(addrs).ok()?,
            verified_addrs: HashMap::new(),
            challenges: HashMap::new(),
            pending: HashMap::new(),
            recent_broadcasts: HashMap::new(),
        })
    }

    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        let _ = self.sock.set_read_timeout(timeout);
    }

    pub fn listen(&self) -> Option<(SocketAddr, Message)> {
        let mut buf = [0u8; 2000];
        let (byte_count, from_addr) = self.sock.recv_from(&mut buf).ok()?;
        Message::deserialize(&buf[..byte_count]).map(|msg| (from_addr, msg))
    }

    pub fn send<A: ToSocketAddrs>(&mut self, addrs: A, msg: Message) -> bool {
        if let Ok(addrs) = addrs.to_socket_addrs() {
            let mut success = false;
            for addr in addrs {
                if self.send_addr(Into::<SocketAddr>::into(addr), msg.clone()) {
                    success = true;
                    break;
                }
            }
            success
        } else {
            false
        }
    }
    pub fn send_addr<A: Into<SocketAddr>>(&mut self, addr: A, msg: Message) -> bool {
        let addr = addr.into();
        let sent = send_addr(&self.sock, &mut self.verified_addrs, addr, &msg);
        if !sent {
            self.request_verification(self.id, addr);
            self.add_pending(addr.into(), msg);
        }
        sent
    }

    pub fn send_several(&mut self, msg: Message) {
        // if same message has been sent recently, do not repeat
        if let Some(timeout) = self.recent_broadcasts.get(&msg) {
            if *timeout + REBROADCAST_TIMEOUT < Instant::now() {
                return;
            }
        }

        let neighbors: Vec<_> = self
            .verified_addrs
            .iter()
            .filter_map(
                |(sockaddr, (_verification_time, is_neighbor))| {
                    if *is_neighbor { Some(sockaddr) } else { None }
                },
            )
            .collect();
        println!("sending several to {:?}", neighbors);
        let mut rng = rng();
        let recipients: Vec<_> = neighbors
            .choose_multiple(&mut rng, 10)
            .map(|addr| (*addr).clone())
            .collect();
        for recipient in recipients {
            send_addr(&self.sock, &mut self.verified_addrs, recipient, &msg);
        }
        self.recent_broadcasts.insert(msg, Instant::now());
    }

    /// This node would like to send a message to another node, but first it must verify that node as part of the network.
    pub fn request_verification(&mut self, from: Id, addr: SocketAddr) {
        let mut rng = rand::rng();

        let challenge = Alphabetic.sample_string(&mut rng, 10);
        self.challenges.insert(challenge.clone(), addr.clone());
        let data = Message::verify(from, challenge);
        let _ = self.sock.send_to(&data.serialize(), addr);
    }

    pub fn challenge_exists(&self, challenge: &String) -> bool {
        self.challenges.contains_key(challenge)
    }

    /// This node has received a verify challenge and must return it.
    pub fn verify(&self, addr: &SocketAddr, challenge: String) {
        let _ = self.sock.send_to(
            &Message::verified(self.id, challenge, true).serialize(),
            addr,
        );
    }

    /// Another node as returned our challenge and we can now send the messages to them
    pub fn verified(&mut self, challenge: &String, is_neighbor: bool) {
        if let Some(addr) = self.challenges.remove(challenge) {
            self.verified_addrs
                .insert(addr.into(), (Instant::now(), is_neighbor));

            // send pending
            if let Some(pending) = self.pending.remove(&addr) {
                for (msg, _sent_time) in pending {
                    send_addr(&self.sock, &mut self.verified_addrs, addr, &msg);
                }
            }
        }
    }

    fn add_pending(&mut self, addr: SocketAddr, msg: Message) {
        let entry = self.pending.entry(addr).or_default();
        entry.push((msg, Instant::now()));
    }

    pub fn clean(&mut self) {
        // clean addrs
        self.verified_addrs
            .retain(|_, (verification_time, _is_neighbor)| {
                (*verification_time + VERIFICATION_TIMEOUT) >= Instant::now()
            });

        // Clean Challenges (Need to store time challenge was issued)
        // self.challenges.retain(|x, y|);

        // Clean recent broadcasts
        self.recent_broadcasts
            .retain(|_, sent_time| *sent_time + REBROADCAST_TIMEOUT > Instant::now());

        // clean pending
        self.pending.retain(|addr, messages| {
            if self.verified_addrs.contains_key(addr) {
                // send messages, remove
                for (msg, _timeout) in messages {
                    send_addr(&self.sock, &mut self.verified_addrs, *addr, msg);
                }
                false
            } else {
                // filter timed out messages only
                messages.retain(|(_msg, timeout)| *timeout + PENDING_TIMEOUT > Instant::now());
                true
            }
        });
    }

    /// Sends a list of neighbors to some of its neighbors
    pub fn swap_neighbors(&mut self) {
        let neighbors: Vec<_> = self
            .verified_addrs
            .iter()
            .filter_map(
                |(sockaddr, (_verification_time, is_neighbor))| {
                    if *is_neighbor { Some(sockaddr) } else { None }
                },
            )
            .collect();

        let mut rng = rng();
        let selected: Vec<_> = neighbors
            .choose_multiple(&mut rng, 10)
            .map(|addr| addr.to_string())
            .collect();

        println!("selected neighbors {:?}", selected);
        self.send_several(Message::neighbors(self.id, selected.clone()));
    }

    /// Initializes connections to other nodes if needed
    pub fn swapped_neighbors(&mut self, mut neighbors: Vec<String>) {
        neighbors.shuffle(&mut rng());
        let connection_deficit = TARGET_CONNECTIONS - self.verified_addrs.len();
        // filter list to parsable, yet unconnected addrs
        let addrs = neighbors
            .iter()
            .filter_map(|new_neighbor| {
                let mut addrs = new_neighbor.to_socket_addrs().ok()?;
                addrs
                    .next()
                    .filter(|addr| !self.verified_addrs.contains_key(&addr))
            })
            .take(connection_deficit)
            .collect::<Vec<_>>();

        for addr in addrs {
            self.request_verification(self.id, addr);
        }
    }
}

fn send_addr<A: Into<SocketAddr>>(
    sock: &UdpSocket,
    verified_addrs: &mut HashMap<SocketAddr, (Instant, bool)>,
    addr: A,
    msg: &Message,
) -> bool {
    let addr = addr.into();
    // check addr against verified addrs

    let entry = verified_addrs.entry(addr);
    let verified = match &entry {
        Entry::Occupied(occupied_entry) => {
            let (verification_time, _is_neighbor) = occupied_entry.get();
            if (*verification_time + VERIFICATION_TIMEOUT) < Instant::now() {
                false
            } else {
                true
            }
        }
        _ => false,
    };

    // send the message if verified
    if verified {
        // send msg
        let data = msg.serialize();
        let _ = sock
            .send_to(&data, entry.key())
            .expect("Send msg should succeed");
    }
    if let Entry::Occupied(occupied_entry) = entry {
        if !verified {
            occupied_entry.remove();
        }
    }

    verified
}
