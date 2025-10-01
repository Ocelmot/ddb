use crate::{id::Id, sequence_num::SequenceNumber};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub id: Id,
    pub seq: SequenceNumber,
    pub key: String,
    pub val: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Message {
    from: Id,
    msg_type: MessageType,
}

impl Message {
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        serde_json::de::from_slice(&data).ok()
    }

    pub fn from(&self) -> &Id {
        &self.from
    }

    pub fn msg_type(&self) -> &MessageType {
        &self.msg_type
    }

    pub fn take_msg_type(self) -> MessageType {
        self.msg_type
    }

    pub fn get(from: Id, key: String, count: usize) -> Self {
        Self {
            from,
            msg_type: MessageType::Get { key, count },
        }
    }

    pub fn values(from: Id, entries: Vec<Entry>) -> Self {
        Self {
            from,
            msg_type: MessageType::Values(entries),
        }
    }

    pub fn set(from: Id, key: String, val: String) -> Self {
        // TODO: sequence number is not really needed here, the node will determine the next one
        let entry = Entry {
            id: from.clone(),
            key,
            val,
            seq: SequenceNumber::ZERO,
        };
        Self {
            from,
            msg_type: MessageType::Set(entry),
        }
    }

    pub fn verify(from: Id, challenge: String) -> Message {
        Message {
            from,
            msg_type: MessageType::Verify(challenge, [0; 16]),
        }
    }

    pub fn verified(from: Id, challenge: String, can_be_neighbor: bool) -> Message {
        Message {
            from,
            msg_type: MessageType::Verified(challenge, can_be_neighbor),
        }
    }

    pub fn link(from: Id, addr: String) -> Message {
        Message {
            from,
            msg_type: MessageType::Link(addr),
        }
    }

    pub fn neighbors(from: Id, addrs: Vec<String>) -> Message {
        Message {
            from,
            msg_type: MessageType::Neighbors(addrs),
        }
    }

    pub fn trust(from: Id, target_id: Id, delta: i16) -> Message {
        Message {
            from,
            msg_type: MessageType::Trust(target_id, delta),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        serde_json::ser::to_vec(self).expect("should be serializable")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MessageType {
    /// Before a message can be sent, a node should respond to a Verify challenge.
    /// This is to ensure that messages are sent to valid, active nodes.
    /// 
    /// First String is the challenge
    /// Second array of u8s is padding
    Verify(String, [u8; 16]),

    /// Response to a Verify challenge
    /// 
    /// String is the returned challenge.
    /// bool is if this node should be considered a neighbor of the other node
    Verified(String, bool),

    /// Request the values for some key
    Get {
        key: String,
        count: usize,
    },

    /// The returned entries for a Get request
    Values(Vec<Entry>),

    /// Set the entry in the data
    Set(Entry),

    /// Attempt to connect to the following address
    Link(String),

    /// Neighbors represents a partial list of neighbors known to the sending node.
    /// Neighbors uses a push strategy to propagate node's addrs through out the network.
    Neighbors(Vec<String>),

    /// Change the node's trust in an Id by the given number of ten thousandths
    Trust(Id, i16),
}
