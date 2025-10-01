#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use ddb_lib::{Id, Message, MessageType, Network};

    #[test]
    fn localhost_send_recv() {
        // setup listen/send
        let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1930);
		let listen_id = Id::generate();
        let listener = Network::new(listen_addr, listen_id).unwrap();

        let send_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1931);
		let send_id = Id::generate();
        let mut sender = Network::new(send_addr, send_id).unwrap();

        // verify listener to sender
        sender.request_verification(send_id.clone(), listen_addr);
        let (_v_addr, v_msg) = listener.listen().expect("verification should be received");

        let MessageType::Verify(challenge, _padding) = v_msg.msg_type() else {
            panic!("Incorrect message type received")
        };

        sender.verified(challenge, true);


		// send test message
        let msg = Message::get(send_id, "test".into(), 4);
        sender.send(listen_addr, msg.clone());

        let (_addr, return_msg) = listener.listen().unwrap();

        assert_eq!(return_msg, msg);
    }
}
