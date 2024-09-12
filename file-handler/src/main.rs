use std::{collections::HashMap, fs::File, io::BufReader};

use fallible_streaming_iterator::FallibleStreamingIterator;
use glob::glob;
use quick_xml::{events::Event, Reader};
use rusqlite::Connection;
use rust_xlsxwriter::Workbook;

use file_handler::{KeyMetadata, KeysToFind};

use file_handler::Result;

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

fn write_to_excel() -> Result<()> {
    // Create a new workbook
    let mut workbook = Workbook::new();

    // Add a worksheet
    let worksheet = workbook.add_worksheet();

    let connection = Connection::open("../.spin/sqlite_db.db")?;

    let mut column_stmt = connection.prepare("SELECT * FROM key_value")?;
    let mut row_stmt = connection.prepare("SELECT * FROM ITEMS")?;

    let columns_names = column_stmt.query([])?;
    let column_number = columns_names.count()?;
    let mut max_length = vec![0; column_number];
    max_length.resize(column_number, 0);

    let mut columns_vec = Vec::with_capacity(column_number);
    columns_vec.resize(column_number, String::new());
    let mut columns_names = column_stmt.query([])?;
    while let Some(row) = columns_names.next()? {
        let index: usize = row.get(1)?;
        let value: String = row.get(2)?;
        max_length[index - 1] = value.len();
        columns_vec.insert(index - 1, value.clone());
        worksheet.write(0, (index - 1).try_into()?, value)?;
    }

    let mut rows = row_stmt.query([])?;
    while let Some(row) = rows.next()? {
        let row_number: u32 = row.get(0)?;
        for (index, _column) in columns_vec.iter().enumerate() {
            if index == column_number {
                break;
            }
            let value: String = row.get(index + 1)?;
            if value.len() > max_length[index] {
                max_length[index] = value.len();
            }
            worksheet.write(row_number, index.try_into()?, value)?;
        }
    }
    for (index, column_len) in max_length.into_iter().enumerate() {
        worksheet.set_column_width(index.try_into()?, column_len as u32)?;
    }

    // Save the workbook
    workbook.save("kdm-info.xlsx")?;
    Ok(())
}

fn main() -> Result<()> {
    let mut methods: HashMap<&str, fn(&str, &str) -> String> = HashMap::new();
    methods.insert("convert_from", convert_from);

    let connection = Connection::open("../.spin/sqlite_db.db")?;

    let mut files = Vec::new();
    // Open the XML file
    for file in glob("/home/shiri/Downloads/*.xml").expect("Failed to read glob pattern") {
        files.push(file?);
    }

    for (counter, file) in files.iter().enumerate() {
        let file = BufReader::new(File::open(file)?);

        // Create a new XML reader
        let mut reader = Reader::from_reader(file);

        // Buffer to hold XML data
        let mut buf = Vec::new();

        let mut keys_to_find = KeysToFind::new()?;
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
                        let text_content = e.unescape()?;
                        keys_to_find.update(&current_key, &text_content)?;
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

        let mut command = format!("INSERT OR REPLACE INTO ITEMS VALUES ({}", counter + 1);
        let mut keys: Vec<(KeyMetadata, String)> = keys_to_find.keys();
        keys.sort_by(|a, b| a.0.index.cmp(&b.0.index));

        for key in keys.iter_mut() {
            if key.0.value != *"ID" {
                let value = if key.0.command != "None" && !key.0.command.is_empty() {
                    match methods.get(key.0.command.as_str()) {
                        Some(f) => f(&key.1, &key.0.parameter),
                        None => {
                            println!("No such command");
                            String::from("Unable to perform operation")
                        }
                    }
                } else {
                    key.1.to_string()
                };
                command.push_str(&format!(", {:?}", value));
            }
        }
        command.push(')');

        let mut stmt = connection.prepare(&command)?;
        stmt.execute([])?;
    }

    write_to_excel()?;
    Ok(())
}
