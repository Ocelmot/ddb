use crossbeam::{
    channel::{self, Receiver, Sender, never},
    select,
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ddb_lib::{Id, Message};
use std::{
    io::{self, Read},
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    thread::{self},
};

mod ui;
use ui::UiMessage;

use crate::ui::ui_thread;

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;

    let (ui_in_tx, ui_in_rx) = channel::bounded(10);
    let (ui_out_tx, ui_out_rx) = channel::bounded(10);
    let (in_tx, in_rx) = channel::bounded(10);

    thread::spawn(move || ui_thread(ui_in_rx, ui_out_tx));
    thread::spawn(|| input_thread(in_tx));

    let mut sock = Option::<UdpSocket>::None;
    let mut net_rx = None;
    let mut port = 1500u16;
    let mut id = Id::default();

    loop {
        select! {
            recv(in_rx) -> char => {
                let Ok(char) = char else{break;};
                let res = ui_in_tx.send(UiMessage::Char(char));
                if res.is_err() {
                    break;
                }
            },
            recv(ui_out_rx) -> cmd => {
                let Ok(cmd) = cmd else{eprintln!("ui thread closed");break;};
                let mut parts = cmd.split(' ').filter(|part|{!part.is_empty()});
                let Some(command) = parts.next() else {continue};

                match command {
                    "q" | "quit" => {break;}
                    "id" => {
                        let Some(new_id) = parts.next().map(|str| str.parse() ) else {let _ = ui_in_tx.send(UiMessage::Message(format!("Current id is {id}"))); continue;};
                        if let Ok(new_id) = new_id {
                            id = new_id;
                        }else{
                            id = Id::default();
                        }
                        let _ = ui_in_tx.send(UiMessage::Message(format!("Id set to {:?}", id)));
                    }
                    "port" => {
                        let Some(new_port) = parts.next().map(|str| str.parse() ) else {let _ = ui_in_tx.send(UiMessage::Message(format!("Current port is {port}"))); continue;};
                        if let Ok(new_port) = new_port {
                            port = new_port;
                        }else{
                            let _ = ui_in_tx.send(UiMessage::Message(format!("port not changed")));
                        }
                        let _ = ui_in_tx.send(UiMessage::Message(format!("Port set to {:?}", port)));
                    }
                    "connect" => {
                        match parts.next() {
                            Some(addr) => {
                                sock = None;
                                net_rx = None;
                                if let Some((new_sock, new_rx)) = create_network_thread((IpAddr::V4(Ipv4Addr::UNSPECIFIED), port), addr) {
                                    let _ = ui_in_tx.send(UiMessage::Message(format!("Connected to: {:?}", new_sock.peer_addr())));
                                    sock = Some(new_sock);
                                    net_rx = Some(new_rx);
                                } else {
                                    let _ = ui_in_tx.send(UiMessage::Message("failed to connect".into()));
                                }
                            },
                            None => {let _ = ui_in_tx.send(UiMessage::Message("connect requires an address".into()));},
                        }

                    }
                    "disconnect" => {
                        sock = None;
                        net_rx = None;
                        let _ = ui_in_tx.send(UiMessage::Message("Disconnected".into()));
                    },
                    "get" => {
                        if let Some(sock) = sock.as_ref() {
                            let Some(key) = parts.next() else {let _ = ui_in_tx.send(UiMessage::Message("Key required".into())); continue;};
                            let count = parts.next().map_or(1, |part|{let x = part.parse::<usize>().unwrap_or(1); x});
                            sock.send(&Message::get(id, key.to_string(), count).serialize());
                        }else{
                            let _ = ui_in_tx.send(UiMessage::Message("Not Connected".into()));
                        }
                    }
                    "set" => {
                        // make and send the message for the node to set the data
                        if let Some(sock) = sock.as_ref() {
                            let Some(key) = parts.next() else {let _ = ui_in_tx.send(UiMessage::Message("Key required".into())); continue;};
                            let value = parts.collect::<Vec<_>>().join(" ");
                            sock.send(&Message::set(id, key.to_string(), value).serialize());
                        }else{
                            let _ = ui_in_tx.send(UiMessage::Message("Not Connected".into()));
                        }
                    }
                    "link" => {
                        // instruct node to connect to new other node

                        if let Some(sock) = sock.as_ref() {
                            if let Some(addr) = parts.next() {
                                let _ = ui_in_tx.send(UiMessage::Message(format!("Linking to {}", addr)));
                                sock.send(&Message::link(id, addr.into()).serialize());
                            }else{
                                let _ = ui_in_tx.send(UiMessage::Message(format!("Requires address to which to link")));
                            }
                        }else{
                            let _ = ui_in_tx.send(UiMessage::Message("Not Connected".into()));
                        }
                    }
                    "trust" => {
                        if let Some(sock) = sock.as_ref() {
                            let Some(Ok(target_id)) = parts.next().map(|str| str.parse::<u16>() ) else {let _ = ui_in_tx.send(UiMessage::Message(format!("Missing required id"))); continue;};
                            let Some(Ok(trust_delta)) = parts.next().map(|str| str.parse::<i16>() ) else {let _ = ui_in_tx.send(UiMessage::Message(format!("Missing required change in trust"))); continue;};

                            sock.send(&Message::trust(id, target_id.into(), trust_delta).serialize());
                        }else{
                            let _ = ui_in_tx.send(UiMessage::Message("Not Connected".into()));
                        }
                    }   
                    _ => {}
                }
            }
            recv(net_rx.as_ref().unwrap_or(&never())) -> res => {
                let Ok((_addr, msg)) = res else {continue;};
                match msg.take_msg_type() {
                    ddb_lib::MessageType::Verify(challenge, _pad) => {sock.as_ref().unwrap().send(Message::verified(Id::generate(), challenge, false).serialize().as_slice());},
                    ddb_lib::MessageType::Verified(_challenge, _is_neighbor) => {},// explorer never requests verification
                    ddb_lib::MessageType::Get { key: _, count: _ } => {}, // Explorer should not be asked this
                    ddb_lib::MessageType::Values(items) => {
                        for entry in items {
                            let _ = ui_in_tx.send(UiMessage::Message(format!("Got data: {}={}", entry.key, entry.val)));
                        }
                    },
                    ddb_lib::MessageType::Set(_entry) => {}, // Explorer does not store items
                    ddb_lib::MessageType::Link(_addr) => {}, // Explorer does not link anywhere else
                    ddb_lib::MessageType::Neighbors(_neighbors) => {}, // Explorer has no neighbors except the node it connects to.
                    ddb_lib::MessageType::GetTrust => {}, // Explorer only trusts the one it is connected to
                    ddb_lib::MessageType::Trust{of, delta } => {}, // Explorer does not hold any trust tables
                }
                // new message from the network
                // should process it
            }
        }
    }

    eprintln!("main loop exited");

    disable_raw_mode()?;
    Ok(())
}

fn create_network_thread<A1: ToSocketAddrs, A2: ToSocketAddrs>(
    listen: A1,
    addr: A2,
) -> Option<(UdpSocket, Receiver<(SocketAddr, Message)>)> {
    let sock = UdpSocket::bind(listen).ok()?;
    sock.connect(addr).expect("socket should connect");
    let inner_sock = sock.try_clone().ok()?;
    let (tx, rx) = channel::bounded(10);
    thread::spawn(move || network_thread(inner_sock, tx));
    Some((sock, rx))
}

fn network_thread(sock: UdpSocket, tx: Sender<(SocketAddr, Message)>) -> Option<()> {
    loop {
        let mut buf = [0u8; 2000];
        let (byte_count, from_addr) = sock.recv_from(&mut buf).ok()?;
        let msg = Message::deserialize(&buf[..byte_count]).map(|msg| (from_addr, msg));
        let Some(msg) = msg else { continue };

        tx.send(msg);
    }
}

fn input_thread(tx: Sender<char>) {
    while let Some(key) = std::io::stdin().bytes().next() {
        if let Ok(key) = key {
            if let Some(char) = char::from_u32(key as u32) {
                tx.send(char).expect("io should be open");
            } else {
                eprintln!("could not convert byte to char");
            }
        } else {
            eprintln!("Encountered io read error");
        }
    }
}
