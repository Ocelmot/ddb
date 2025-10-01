use std::{
    collections::{BTreeMap, HashMap},
    iter::repeat,
    time::Instant,
};

use ddb_lib::{Entry, Id, SequenceNumber};

pub struct Data {
    /// Data from us, and trusted peers
    /// Map from keys to sequences of values
    incorporated_data: HashMap<String, BTreeMap<u64, BTreeMap<Id, String>>>,
}

impl Data {
    pub fn new() -> Self {
        Self {
            incorporated_data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, entry: Entry) {
        // entries are sorted by key, then sequence number, then by id (should be id's trust, then id)
        let key_value = self.incorporated_data.entry(entry.key).or_default();
        let seq_value = key_value.entry(entry.seq.num).or_default();
        seq_value.insert(entry.id, entry.val);
    }

    pub fn ingest(&mut self, entries: Vec<Entry>) {
        for entry in entries {
            self.insert(entry);
        }
    }

    /// Does this node already have a copy of this Entry
    ///
    /// Finds an entry where the key matches, then where the seq_num matches, then where the id matches
    pub fn contains(&self, entry: &Entry) -> bool {
        let Some(sequence_tree) = self.incorporated_data.get(&entry.key) else {
            return false;
        };

        let Some(sequence_entries) = sequence_tree.get(&entry.seq.num) else {
            return false;
        };

        sequence_entries.contains_key(&entry.id)
    }

    pub fn get(&self, key: &String, count: usize) -> Vec<Entry> {
        self.incorporated_data
            .get(key)
            .iter()
            .flat_map(|key_value| key_value.iter().rev())
            .flat_map(|seq_value| seq_value.1.iter().zip(repeat(seq_value.0)))
            .take(count)
            .map(|((id, val), seq)| Entry {
                id: id.clone(),
                seq: SequenceNumber { num: *seq },
                key: key.to_string(),
                val: val.clone(),
            })
            .collect()
    }

    pub fn get_next_id(&self, key: &String) -> SequenceNumber {
        self.incorporated_data
            .get(key)
            .iter()
            .flat_map(|key_value| key_value.iter().last())
            .map(|(seq_num, _)| SequenceNumber { num: *seq_num + 1 })
            .next()
            .unwrap_or(SequenceNumber::ZERO)
    }
}
