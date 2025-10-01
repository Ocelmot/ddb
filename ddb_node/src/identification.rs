use std::collections::HashMap;

use ddb_lib::Id;

static DEFAULT_TRUST: f32 = 0.5;
static TRUSTED_LEVEL: f32 = 0.75;
static DISTRUSTED_LEVEL: f32 = 0.25;

pub struct Identification {
	us: Id,
    identities: HashMap<Id, f32>,
}

impl Identification {
    pub fn new(us: Id) -> Self {
        Self {
			us,
            identities: HashMap::new(),
        }
    }

	pub fn is_us(&self, id: &Id) -> bool {
		self.us == *id
	}

	pub fn change_trust(&mut self, id: Id, delta: f32) {
		let trust = self.identities.entry(id).or_insert(DEFAULT_TRUST);
		*trust = (*trust + delta).clamp(0.0, 1.0 );
	}

    pub fn is_trusted(&self, id: &Id) -> bool {
        self.identities
            .get(id)
            .map(|level| level >= &TRUSTED_LEVEL)
            .unwrap_or(false)
    }

	pub fn is_neutral(&self, id: &Id) -> bool {
        self.identities
            .get(id)
            .map(|level| level < &TRUSTED_LEVEL && level > &DISTRUSTED_LEVEL)
            .unwrap_or(false)
    }

	pub fn is_distrusted(&self, id: &Id) -> bool {
        self.identities
            .get(id)
            .map(|level| level <= &DISTRUSTED_LEVEL)
            .unwrap_or(false)
    }
}
