use std::{env::args, net::SocketAddr};

use ddb_lib::Id;

mod config;
mod node;
use node::Node;

use crate::config::Config;
mod data;
mod identification;

fn main() {
    // let config = if let Some(path) = args().nth(1) {
    //     let buf = PathBuf::from(path);
    //     config::Config::load(&buf)
    // } else {
    //     config::Config::default()
    // };
    let config = Config::default();
    let mut listen_addr = config.bind_addr();


    let val = args().nth(1).map(|cmd_addr|{cmd_addr.parse::<SocketAddr>().expect("invalid argument format")});
    if val.is_some() {
        listen_addr = val.as_ref().unwrap();
    }

    let id = Id::generate();
    println!("Running node with id={}", id);
    let node = Node::new(id, listen_addr).expect("node should be able to start");

    node.run();
}
