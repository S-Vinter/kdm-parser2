use std::{any::Any, cmp::Ordering, collections::HashMap, fmt::Debug, ops::Add};

use chrono::{DateTime, FixedOffset, Local, TimeDelta};
use dateparser::DateTimeUtc;

pub trait InternalGeneric: ToString + Debug {
    // fn clone(&self) -> Self where Self: Sized;

    fn try_cmp(&self) -> Ordering {
        return Ordering::Equal;
    }

    fn check_limit(&self, _delta: Box<dyn InternalGeneric>) -> bool {
        return true;
    }

    fn as_any(&self) -> &dyn InternalGeneric;
}

impl InternalGeneric for chrono::DateTime<chrono::FixedOffset> {
    fn try_cmp(&self) -> Ordering {
        let current = Box::new(Local::now().fixed_offset());
        return self.cmp(&current);
    }

    fn check_limit(&self, delta: Box<dyn InternalGeneric>) -> bool {
        println!("delta: {:?}", delta);
        let current = Local::now().fixed_offset();
        return (self as &(dyn Any + '_))
            .downcast_ref::<DateTime<FixedOffset>>()
            .unwrap()
            .timestamp()
            > current.timestamp()
                + (&delta as &(dyn Any + '_))
                    .downcast_ref::<DateTime<FixedOffset>>()
                    .unwrap()
                    .timestamp();
    }

    fn as_any(&self) -> &dyn InternalGeneric {
        self
    }
}
impl InternalGeneric for String {
    fn as_any(&self) -> &dyn InternalGeneric {
        self
    }
}

lazy_static! {
    static ref function_names: &'static [&'static str] = &["String", "Date"];
    static ref functions: &'static [fn(&str) -> Box<dyn InternalGeneric>] =
        &[no_string_convertion, convert_to_date];
    pub static ref convertion: HashMap<String, fn(&str) -> Box<dyn InternalGeneric>> = {
        let mut hashmap = HashMap::new();
        for (name, function) in function_names.iter().zip(functions.iter()) {
            hashmap.insert(name.to_string(), *function);
        }
        hashmap
    };
}

fn no_string_convertion(string: &str) -> Box<dyn InternalGeneric> {
    Box::new(string.to_string()) as Box<dyn InternalGeneric>
}

fn convert_to_date(date: &str) -> Box<dyn InternalGeneric> {
    let tz = FixedOffset::east_opt(3 * 3600).unwrap();
    Box::new(
        date.parse::<DateTimeUtc>()
            .unwrap()
            .0
            .naive_local()
            .and_local_timezone(tz)
            .unwrap()
            .checked_add_signed(TimeDelta::hours(3))
            .unwrap(),
    ) as Box<dyn InternalGeneric>
}
