use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::File,
    hash::{Hash, Hasher},
    io::BufReader,
};

use glob::glob;
use quick_xml::{events::Event, Reader};
use rusqlite::Connection;
use rust_xlsxwriter::Workbook;

fn convert_from(value_from_xml: &str, chart: &str) -> String {
    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();
    let command = &format!("SELECT * FROM {}", chart);
    let mut row_stmt = connection.prepare(command).unwrap();

    let mut rows = row_stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let value: u32 = row.get(1).unwrap();
        if value_from_xml.contains(&value.to_string()) {
            let name: String = row.get(0).unwrap();
            return name;
        }
    }

    String::from("Not found")
}

#[derive(Debug)]
pub struct Attribute {
    pub index: u32,
    pub name: String,
    pub command: String,
    pub parameter: String,
}

impl Attribute {
    pub fn new(index: u32, name: &str, command: &str, parameter: &str) -> Self {
        Attribute {
            index,
            name: name.to_string(),
            command: command.to_string(),
            parameter: parameter.to_string(),
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

fn load_db() -> Vec<Attribute> {
    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();

    let mut stmt = connection.prepare("SELECT * FROM key_value").unwrap();

    let mut rows = stmt.query([]).unwrap();

    let mut names: Vec<Attribute> = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        let index: u32 = row.get(1).unwrap();
        let name: String = row.get(0).unwrap();
        let command: String = row.get(3).unwrap();
        let parameter: String = row.get(4).unwrap();
        names.push(Attribute::new(index, &name, &command, &parameter));
    }

    names.sort();

    names
}

#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub value: String,
    pub index: u32,
    pub command: String,
    pub parameter: String,
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

impl KeyMetadata {
    pub fn comparison_metadata(value: &str) -> Self {
        Self {
            value: value.to_string(),
            index: 0,
            command: String::new(),
            parameter: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct KeysToFind {
    keys_values: HashMap<KeyMetadata, String>,
}

impl KeysToFind {
    pub fn new() -> Self {
        let parameters = load_db();
        let mut keys_values = HashMap::new();

        for (counter, parameter) in parameters.iter().enumerate() {
            if parameter.name != *"VALUE" {
                keys_values.insert(
                    KeyMetadata {
                        value: parameter.name.to_string(),
                        index: counter.try_into().unwrap(),
                        command: parameter.command.to_string(),
                        parameter: parameter.parameter.to_string(),
                    },
                    String::new(),
                );
            }
        }
        Self { keys_values }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        for key_iter in self.keys_values.iter() {
            if key_iter.0.value == *key {
                return true;
            }
        }
        false
    }

    pub fn build_key_from_name(&self, key: &str) -> Option<KeyMetadata> {
        let mut key_iter: Vec<KeyMetadata> = self.keys_values.clone().into_keys().collect();
        key_iter.sort_by(|a, b| a.index.cmp(&b.index));
        for (counter, key_iter) in key_iter.iter().enumerate() {
            if key_iter.value == *key {
                return Some(KeyMetadata {
                    value: key.to_string(),
                    index: counter.try_into().unwrap(),
                    command: key_iter.command.to_string(),
                    parameter: key_iter.parameter.to_string(),
                });
            }
        }
        None
    }

    pub fn update(&mut self, key: &str, value: &str) {
        self.keys_values
            .insert(self.build_key_from_name(key).unwrap(), value.to_string());
    }

    pub fn get(&self) -> &HashMap<KeyMetadata, String> {
        &self.keys_values
    }

    pub fn get_value(&self, key: &str) -> String {
        let full_key = self.build_key_from_name(key).unwrap();
        self.keys_values.get(&full_key).unwrap().to_string()
    }

    pub fn keys(&self) -> Vec<&KeyMetadata> {
        let mut keys: Vec<&KeyMetadata> = self.keys_values.keys().collect();
        keys.sort_by(|a, b| a.index.cmp(&b.index));
        keys
    }
}

fn write_to_excel() {
    // Create a new workbook
    let mut workbook = Workbook::new();

    // Add a worksheet
    let worksheet = workbook.add_worksheet();

    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();

    let mut column_stmt = connection.prepare("SELECT * FROM key_value").unwrap();
    let mut row_stmt = connection.prepare("SELECT * FROM ITEMS").unwrap();

    let mut columns_names = column_stmt.query([]).unwrap();
    let mut columns_vec = vec![];
    let mut column_number = 0;
    while let Some(row) = columns_names.next().unwrap() {
        let value: String = row.get(2).unwrap();
        columns_vec.push(value.clone());
        worksheet.write(0, column_number, value).unwrap();
        column_number += 1;
    }

    let mut rows = row_stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let row_number: u32 = row.get(0).unwrap();
        for (index, _column) in columns_vec.iter().enumerate() {
            if index == columns_vec.len() {
                break;
            }
            let value: String = row.get(index + 1).unwrap();
            worksheet
                .write(row_number, index.try_into().unwrap(), value)
                .unwrap();
        }
    }

    // Save the workbook
    workbook.save("kdm-info.xlsx").unwrap();
}

fn main() {
    let mut methods: HashMap<&str, fn(&str, &str) -> String> = HashMap::new();
    methods.insert("convert_from", convert_from);

    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();

    let mut files = Vec::new();
    // Open the XML file
    for file in glob("/home/shiri/Downloads/*.xml").expect("Failed to read glob pattern") {
        files.push(file.unwrap());
    }

    for (counter, file) in files.iter().enumerate() {
        let file = BufReader::new(File::open(file).unwrap());

        // Create a new XML reader
        let mut reader = Reader::from_reader(file);

        // Buffer to hold XML data
        let mut buf = Vec::new();

        let mut keys_to_find = KeysToFind::new();
        let mut current_key = String::new();

        // Read XML events from the reader
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    // Process start tags (e.g., <tag>)
                    let start_tag = String::from_utf8_lossy(e.name().local_name().into_inner());
                    if keys_to_find.contains_key(&start_tag) {
                        current_key = start_tag.to_string();
                    }
                }
                Ok(Event::Text(e)) => {
                    // Process text content
                    if !current_key.is_empty() {
                        let text_content = e.unescape().unwrap();
                        keys_to_find.update(&current_key, &text_content);
                        current_key = String::new();
                    }
                }
                Ok(Event::Eof) => break, // Exit the loop when reaching end of file
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                _ => (), // Ignore other events
            }

            // Reset the buffer to reuse it for the next event
            buf.clear();
        }

        let mut command = format!("INSERT OR IGNORE INTO ITEMS VALUES ({}", counter + 1);
        let mut keys = keys_to_find.keys();
        keys.sort_by(|a, b| a.index.cmp(&b.index));

        for key in keys.iter_mut() {
            if key.value != *"ID" {
                let value = if key.command != "None" && !key.command.is_empty() {
                    match methods.get(key.command.as_str()) {
                        Some(f) => f(keys_to_find.get().get(key).unwrap(), &key.parameter),
                        None => {
                            println!("No such command");
                            String::from("Unable to perform operation")
                        }
                    }
                } else {
                    keys_to_find.get_value(&key.value)
                };
                command.push_str(&format!(", {:?}", value));
            }
        }
        command.push(')');

        let mut stmt = connection.prepare(&command).unwrap();
        stmt.execute([]).unwrap();
    }

    write_to_excel();
}
