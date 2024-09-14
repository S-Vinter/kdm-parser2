use std::{cmp::Ordering, collections::HashMap};

use chrono::TimeDelta;

use crate::{convertion, data_types::InternalGeneric, KeyMetadata};

lazy_static! {
    static ref function_names: &'static [&'static str] = &["convert_from", "range_with"];
    static ref functions: &'static [fn(&str, &KeyMetadata) -> String] = &[convert_from, range_with];
    pub static ref methods: HashMap<String, Box::<fn(&str, &KeyMetadata) -> String>> = {
        let mut hashmap = HashMap::new();
        for (name, function) in function_names.iter().zip(functions.iter()) {
            hashmap.insert(
                name.to_string(),
                Box::<fn(&str, &KeyMetadata) -> String>::new(*function),
            );
        }
        hashmap
    };
}

fn convert_from(value_from_xml: &str, metadata: &KeyMetadata) -> String {
    let connection = rusqlite::Connection::open("../.spin/sqlite_db.db").unwrap();
    let command = &format!("SELECT * FROM {}", metadata.parameters[0]);
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

lazy_static! {
    static ref colors_array: &'static [&'static str] =
        &["green", "green", "blue", "red", "red", "green"];
    static ref pairs: &'static [&'static (Ordering, &'static str)] = &[
        &(Ordering::Equal, "ContentKeysNotValidBefore"),
        &(Ordering::Greater, "ContentKeysNotValidBefore"),
        &(Ordering::Less, "ContentKeysNotValidBefore"),
        &(Ordering::Equal, "ContentKeysNotValidAfter"),
        &(Ordering::Greater, "ContentKeysNotValidAfter"),
        &(Ordering::Less, "ContentKeysNotValidAfter"),
    ];
    pub static ref colors: HashMap<(Ordering, String), String> = {
        let mut hashmap = HashMap::new();
        for (pair, color) in pairs.iter().zip(colors_array.iter()) {
            hashmap.insert((pair.0, pair.1.to_string()), color.to_string());
        }
        hashmap
    };
}

fn range_with(value_from_xml: &str, metadata: &KeyMetadata) -> String {
    let value = match convertion.get(&metadata.data_type) {
        Some(f) => f(value_from_xml),
        None => {
            println!("No such type");
            Box::new(metadata.value.to_string())
        }
    };

    let color = colors.get(&(value.try_cmp(), metadata.value.as_str().to_string())).unwrap();

    if metadata.value == String::from("ContentKeysNotValidAfter") {
        let time_delta = chrono::DateTime::UNIX_EPOCH
            .checked_add_signed(TimeDelta::hours(48))
            .unwrap()
            .fixed_offset();
        if !value.check_limit(Box::new(time_delta)) {
            let color = "orange";
        }
    }

    println!(
        "check: {:?}", color
    );

    value_from_xml.to_string()
}
