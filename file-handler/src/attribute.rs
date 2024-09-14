use std::cmp::Ordering;

use rusqlite::Connection;

use crate::error::Result;

pub fn load_db() -> Result<Vec<Attribute>> {
    let connection = Connection::open("../.spin/sqlite_db.db")?;

    let mut stmt = connection.prepare("SELECT * FROM key_value")?;

    let mut rows = stmt.query([])?;

    let mut names: Vec<Attribute> = Vec::new();
    while let Some(row) = rows.next()? {
        let index: u32 = row.get(1)?;
        let name: String = row.get(0)?;
        let data_type: String = row.get(3)?;
        let command: String = row.get(4)?;
        let parameters_string: String = row.get(5)?;
        let parameters: Vec<String> = parameters_string
            .split(", ")
            .map(|param| param.to_string())
            .collect();
        names.push(Attribute::new(
            index, &name, &data_type, &command, parameters,
        ));
    }

    names.sort();

    Ok(names)
}

#[derive(Debug)]
pub struct Attribute {
    pub index: u32,
    pub name: String,
    pub data_type: String,
    pub command: String,
    pub parameters: Vec<String>,
}

impl Attribute {
    pub fn new(
        index: u32,
        name: &str,
        data_type: &str,
        command: &str,
        parameter: Vec<String>,
    ) -> Self {
        Attribute {
            index,
            name: name.to_string(),
            data_type: data_type.to_string(),
            command: command.to_string(),
            parameters: parameter,
        }
    }
}

impl Ord for Attribute {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl PartialOrd for Attribute {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Attribute {
    fn eq(&self, other: &Self) -> bool {
        (self.index, &self.name) == (other.index, &other.name)
    }
}

impl Eq for Attribute {}
