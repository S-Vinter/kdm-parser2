use std::{collections::HashMap, fs::File, io::BufReader};

use quick_xml::{events::Event, Reader};
use rust_xlsxwriter::Workbook;
use rusqlite::{Connection, Result};

fn load_db() {
    let connection = Connection::open("../.spin/sqlite_db.db").unwrap();

    let mut stmt = connection.prepare(
        "SELECT * FROM PRAGMA_TABLE_INFO('ITEMS')",
    ).unwrap();

    println!("{:?}", stmt);
    println!("{:?}", stmt.column_names());

    stmt.query_map(["a"], |row| {
        println!("check");
        println!("{:?}", row);
        Ok(())
    });
}

#[derive(Debug)]
pub struct KeysToFind {
    keys_values: HashMap<String, String>,
}

impl KeysToFind {
    pub fn new() -> Self {
        let mut keys_values = HashMap::new();
        keys_values.insert(String::from("ContentTitleText"), String::new());
        keys_values.insert(String::from("ContentKeysNotValidBefore"), String::new());
        keys_values.insert(String::from("ContentKeysNotValidAfter"), String::new());
        keys_values.insert(String::from("X509SubjectName"), String::new());
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
}

fn write_to_excel(keys_to_find: KeysToFind) {
    // Create a new workbook
    let mut workbook = Workbook::new();

    // Add a worksheet
    let worksheet = workbook.add_worksheet();

    // Write some data
    for (counter, key_value) in keys_to_find.get().iter().enumerate() {
        worksheet
            .write_string(0, counter.try_into().unwrap(), key_value.0)
            .unwrap();
        worksheet
            .write_string(1, counter.try_into().unwrap(), key_value.1)
            .unwrap();
    }

    // Save the workbook
    workbook.save("kdm_chart.xlsx").unwrap();
    // Ok(())
}

// #[async_std::main]
fn main() {
    load_db();

    // Open the XML file
    let file = File::open("/home/shiri/Downloads/k_KDM_Shambhala_BO-EN__cert_Dolby-IMS3000-372441-SMPTE_20240719_20240722_TIT_OV.xml").expect("Failed to open file");
    let file = BufReader::new(file);

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

    // write_to_excel(keys_to_find);
}
