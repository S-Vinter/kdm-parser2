use std::{collections::HashMap, fs::File, io::BufReader};

use glob::glob;
use quick_xml::{events::Event, Reader};
use rusqlite::{Connection, Result};
use rust_xlsxwriter::Workbook;
use table::Key;

fn load_db() -> Vec<String> {
    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();

    let mut stmt = connection
        .prepare("SELECT * FROM key_value")
        .unwrap();

    let mut rows = stmt.query([]).unwrap();

    let mut names: Vec<(u32, String)> = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        let index: u32 = row.get(1).unwrap();
        names.push((index, row.get(0).unwrap()));
    }

    names.sort_by(|a, b| a.0.cmp(&b.0));
    println!("{:?}", names);

    let mut names_sorted = vec![];
    for name in names {
        names_sorted.push(name.1);
    }

    names_sorted
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct KeyMetadata {
    pub value: String,
    pub index: u32,
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
            if parameter.to_string() != String::from("VALUE") {
                keys_values.insert(
                    KeyMetadata {
                        value: parameter.to_string(),
                        index: counter.try_into().unwrap(),
                    },
                    String::new(),
                );
            }
        }
        Self { keys_values }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        for key_iter in self.keys_values.iter() {
            if key_iter.0.value == key.to_string() {
                return true;
            }
        }
        return false;
    }

    pub fn build_key_from_name(&self, key: &str) -> Option<KeyMetadata> {
        let mut key_iter: Vec<KeyMetadata> = self.keys_values.clone().into_keys().collect();
        key_iter.sort_by(|a, b| a.index.cmp(&b.index));

        for (counter, key_iter) in key_iter.iter().enumerate() {
            if key_iter.value == key.to_string() {
                return Some(KeyMetadata {
                    value: key.to_string(),
                    index: counter.try_into().unwrap(),
                });
            }
        }
        return None;
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
        return keys;
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
        column_number+=1;
    }

    let mut rows = row_stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let row_number: u32 = row.get(0).unwrap();
        for (index, _column) in columns_vec.iter().enumerate() {
            if index == columns_vec.len() {
                break;
            }
            let value: String = row.get(index+1).unwrap();
            worksheet.write(row_number, index.try_into().unwrap(), value).unwrap();
        }
    }

    // Save the workbook
    workbook.save("kdm-info.xlsx").unwrap();
}

fn main() {
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
        println!("keys_to_find: {:?}", keys_to_find);
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
                        println!("{}: {}", current_key, text_content);
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

        let mut command = format!("INSERT INTO ITEMS VALUES ({}", counter + 1);
        let mut keys = keys_to_find.keys();
        keys.sort_by(|a, b| a.index.cmp(&b.index));

        for key in keys.iter() {
            if key.value != String::from("ID") {
                command.push_str(&format!(", {:?}", keys_to_find.get_value(&key.value)));
            }
        }
        println!("command: {:?}", command);
        command.push(')');

        let mut stmt = connection
            .prepare(&command)
            .unwrap();
        stmt.execute([]);
    }

    write_to_excel();
}
