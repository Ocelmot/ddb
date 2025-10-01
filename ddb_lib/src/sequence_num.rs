use std::cmp::Ordering;




#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SequenceNumber{
	pub num: u64
}

impl SequenceNumber {
	pub const ZERO: Self = Self{num: 0};

	pub fn order(&self, other: &Self) -> Ordering {
		Ordering::Equal
	}
}