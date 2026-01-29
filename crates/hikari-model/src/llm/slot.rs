use serde_yml::Value;

#[derive(Debug, Clone)]
pub struct Slot {
    pub name: String,
    pub value: Value,
}

impl Slot {
    #[must_use]
    pub fn new(name: String, value: Value) -> Self {
        Self { name, value }
    }
}
