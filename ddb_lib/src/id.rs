use std::{fmt::Display, num::ParseIntError, str::FromStr};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Id {
    // some sort of id/asymmetric key
    tmp: u16,
}

impl Id {
    pub fn generate() -> Self {
        Self {
            tmp: rand::random(),
        }
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tmp)
    }
}

impl FromStr for Id {
	type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
		let inner = s.parse::<u16>()?;
		Ok(Id { tmp: inner })
	}
}

impl Default for Id {
	fn default() -> Self {
		Self { tmp: 12345 }
	}
}

impl From<u16> for Id {
    fn from(value: u16) -> Self {
        Self { tmp: value }
    }
}