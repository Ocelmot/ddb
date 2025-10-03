use std::collections::HashMap;

use ddb_lib::Id;

static DEFAULT_TRUST: f32 = 0.5;
static TRUSTED_LEVEL: f32 = 0.75;
static DISTRUSTED_LEVEL: f32 = 0.25;

pub struct Identification {
    us: Id,
    base_trust: HashMap<Id, f32>,
    trust_offset: HashMap<Id, HashMap<Id, f32>>,
}

impl Identification {
    pub fn new(us: Id) -> Self {
        Self {
            us,
            base_trust: HashMap::new(),
            trust_offset: HashMap::new(),
        }
    }

    pub fn is_us(&self, id: &Id) -> bool {
        self.us == *id
    }

    pub fn get_trust(&self, of: &Id) -> f32 {
        let base_trust = *self.base_trust.get(of).unwrap_or(&DEFAULT_TRUST);
        let trust_offset = self.get_offset(of);
        base_trust + trust_offset
    }

    pub fn change_trust(&mut self, id: Id, delta: f32) {
        if self.is_us(&id) {
            return;
        }
        let trust = self.base_trust.entry(id).or_insert(DEFAULT_TRUST);
        *trust = (*trust + delta).clamp(0.0, 1.0);
    }

    pub fn is_trusted(&self, id: &Id) -> bool {
        self.get_trust(id) >= TRUSTED_LEVEL
    }

    pub fn is_neutral(&self, id: &Id) -> bool {
        let trust = self.get_trust(id);
        trust < TRUSTED_LEVEL && trust > DISTRUSTED_LEVEL
    }

    pub fn is_distrusted(&self, id: &Id) -> bool {
        self.get_trust(id) <= DISTRUSTED_LEVEL
    }

    pub fn get_offset(&self, of: &Id) -> f32 {
        self.trust_offset.get(of).map_or(0.0, |trustors| {
            trustors
                .iter()
                .map(|(trustor, trust)| {
                    let trustor_trust = self.base_trust.get(trustor).unwrap_or(&0.5);
                    trustor_trust * (trust - 0.5)
                })
                .fold(0.0, |a, b| a + b)
        })
    }

    pub fn adjust_offset(&mut self, from: Id, of: Id, level: f32) {
        let trustors = self.trust_offset.entry(of).or_default();
        trustors.insert(from, level);
    }

    pub fn base_trust(&self) -> impl Iterator<Item = (&Id, &f32)> {
        self.base_trust.iter()
    }
}
