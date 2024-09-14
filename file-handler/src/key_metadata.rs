use std::hash::{Hash, Hasher};

use crate::{convertion, methods, Result};

#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub value: String,
    pub index: u32,
    pub data_type: String,
    pub command: String,
    pub parameters: Vec<String>,
}

impl KeyMetadata {
    pub fn parse_output(&self, value: &str) -> Result<String> {
        let ret_value = match convertion.get(&self.data_type) {
            Some(f) => f(value),
            None => {
                println!("No such type");
                Box::new(value.to_string())
            }
        }
        .to_string();
        if self.value != *"ID" {
            if self.command != "None" && !self.command.is_empty() {
                match methods.get(self.command.as_str()) {
                    Some(f) => return Ok(f(value, &self)),
                    None => {
                        println!("No such command");
                    }
                }
            }
        }

        Ok(ret_value)
    }
}

impl PartialEq for KeyMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for KeyMetadata {}

impl Hash for KeyMetadata {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
