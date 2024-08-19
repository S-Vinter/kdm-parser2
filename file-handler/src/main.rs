use std::{collections::HashMap, fs::File, io::BufReader};

use glob::glob;
use quick_xml::{events::Event, Reader};
use rusqlite::{Connection, Result};
use rust_xlsxwriter::Workbook;

fn load_db() -> Vec<String> {
    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();

    let mut stmt = connection
        .prepare("SELECT * FROM PRAGMA_TABLE_INFO('ITEMS')")
        .unwrap();

    let mut rows = stmt.query([]).unwrap();

    let mut names: Vec<String> = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        names.push(row.get(1).unwrap());
    }

    names
}

#[derive(Debug)]
pub struct KeysToFind {
    keys_values: HashMap<String, String>,
}

impl KeysToFind {
    pub fn new() -> Self {
        let parameters = load_db();
        let mut keys_values = HashMap::new();

        for parameter in parameters {
            if parameter != String::from("VALUE") {
                keys_values.insert(parameter, String::new());
            }
        }
        // keys_values.insert(String::from("ContentTitleText"), String::new());
        // keys_values.insert(String::from("ContentKeysNotValidBefore"), String::new());
        // keys_values.insert(String::from("ContentKeysNotValidAfter"), String::new());
        // keys_values.insert(String::from("X509SubjectName"), String::new());
        Self { keys_values }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.keys_values.contains_key(key)
    }

    pub fn update(&mut self, key: &str, value: &str) -> Option<String> {
        self.keys_values.insert(key.to_string(), value.to_string())
    }

    pub fn get(&self) -> &HashMap<String, String> {
        &self.keys_values
    }

    pub fn get_value(&self, key: &str) -> String {
        self.keys_values.get(key).unwrap().to_string()
    }
}

fn write_to_excel() {
    // Create a new workbook
    let mut workbook = Workbook::new();

    // Add a worksheet
    let worksheet = workbook.add_worksheet();

    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();

    let mut stmt = connection
        .prepare("SELECT * FROM ITEMS")
        .unwrap();

    let columns_names = stmt.column_names();
    
    for (counter, name) in columns_names.iter().enumerate() {
        if name.to_owned() != "ID" {
            let column_number = u16::try_from(counter).unwrap() - 1;
            worksheet.write(0, column_number, name.to_string()).unwrap();
        }
    }
    
    let mut rows = stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        println!("row: {:?}", row);
        let row_number: u32 = row.get(0).unwrap();
        let name: String = row.get(1).unwrap();
        let start_validity: String = row.get(2).unwrap();
        let end_validity: String = row.get(3).unwrap();
        let server_name: String = row.get(4).unwrap();
        worksheet.write(row_number, 0, name).unwrap();
        worksheet.write(row_number, 1, start_validity).unwrap();
        worksheet.write(row_number, 2, end_validity).unwrap();
        worksheet.write(row_number, 3, server_name).unwrap();
    }

    // // Write some data
    // for (counter, key_value) in keys_to_find.get().iter().enumerate() {
    //     worksheet
    //         .write_string(0, counter.try_into().unwrap(), key_value.0)
    //         .unwrap();
    //     worksheet
    //         .write_string(1, counter.try_into().unwrap(), key_value.1)
    //         .unwrap();
    // }

    // Save the workbook
    workbook.save("kdm-info.xlsx").unwrap();
}

fn main() {
    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();
    load_db();

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
                        keys_to_find.update(&current_key, &text_content).unwrap();
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

        let mut stmt = connection
            .prepare(&format!(
                "INSERT INTO ITEMS VALUES ({}, {:?}, {:?}, {:?}, {:?});",
                counter+1,
                keys_to_find.get_value("ContentTitleText"),
                keys_to_find.get_value("ContentKeysNotValidBefore"),
                keys_to_find.get_value("ContentKeysNotValidAfter"),
                keys_to_find.get_value("X509SubjectName")
            ))
            .unwrap();
        stmt.execute([]);
    }

    write_to_excel();
}
