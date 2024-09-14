use std::collections::HashMap;

use crate::{attribute::load_db, key_metadata::KeyMetadata, Error, Result};

#[derive(Debug)]
pub struct KeysToFind {
    keys_values: HashMap<KeyMetadata, String>,
}

impl KeysToFind {
    pub fn new() -> Result<Self> {
        let parameters = load_db()?;
        let mut keys_values = HashMap::new();

        for (counter, parameter) in parameters.iter().enumerate() {
            if parameter.name != *"VALUE" {
                keys_values.insert(
                    KeyMetadata {
                        value: parameter.name.to_string(),
                        index: counter.try_into()?,
                        data_type: parameter.data_type.to_string(),
                        command: parameter.command.to_string(),
                        parameters: parameter.parameters.clone(),
                    },
                    String::new(),
                );
            }
        }
        Ok(Self { keys_values })
    }

    pub fn contains_key(&self, key: &str) -> bool {
        for key_iter in self.keys_values.iter() {
            if key_iter.0.value == *key {
                return true;
            }
        }
        false
    }

    pub fn get_metadata_by_value(&self, key: &str) -> Result<KeyMetadata> {
        let mut key_iter: Vec<KeyMetadata> = self.keys_values.clone().into_keys().collect();
        key_iter.sort_by(|a, b| a.index.cmp(&b.index));
        for (counter, key_iter) in key_iter.iter().enumerate() {
            if key_iter.value == *key {
                return Ok(KeyMetadata {
                    value: key.to_string(),
                    index: counter.try_into()?,
                    data_type: key_iter.data_type.to_string(),
                    command: key_iter.command.to_string(),
                    parameters: key_iter.parameters.clone(),
                });
            }
        }
        Err(Error::NoSuchKey)
    }

    pub fn update(&mut self, key: &str, value: &str) -> Result<()> {
        if self.contains_key(key) {
            self.keys_values
                .insert(self.get_metadata_by_value(key)?, value.to_string());
        }
        Ok(())
    }

    pub fn get(&self) -> &HashMap<KeyMetadata, String> {
        &self.keys_values
    }

    pub fn get_value(&self, key: &str) -> Result<String> {
        let full_key = self.get_metadata_by_value(key)?;
        Ok(self
            .keys_values
            .get(&full_key)
            .ok_or(Error::NoSuchKey)?
            .to_string())
    }

    pub fn keys(mut self) -> Vec<(KeyMetadata, String)> {
        self.keys_values.drain().collect()
    }
}
