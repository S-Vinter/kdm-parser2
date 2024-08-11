use anyhow::{Context, Result};
use build_html::{Container, ContainerType, Html, HtmlContainer};
use serde::{Deserialize, Serialize};
use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
use spin_sdk::http_component;
use spin_sdk::sqlite::{Connection, Value};

#[http_component]
fn handle_api(req: Request) -> Result<impl IntoResponse> {
    // lets use the Router to handle requests based on method and path
    let mut r = Router::default();
    r.post("/api/items", add_new);
    r.get("/api/items", get_all);
    r.delete("/api/items/:value", delete_one);
    Ok(r.handle(req))
}

#[derive(Debug, Deserialize, Serialize)]
struct Item {
    // #[serde(skip_deserializing)]
    // id: i64,
    value: String,
}

impl Html for Item {
    fn to_html_string(&self) -> String {
        Container::new(ContainerType::Div)
            .with_attributes(vec![
                ("class", "item"),
                ("value", format!("item-{}", &self.value).as_str()),
            ])
            .with_container(
                Container::new(ContainerType::Div)
                    .with_attributes(vec![("class", "value")])
                    .with_raw(&self.value),
            )
            .with_container(
                Container::new(ContainerType::Div)
                    .with_attributes(vec![
                        ("class", "delete-item"),
                        ("hx-delete", format!("/api/items/{}", &self.value).as_str()),
                    ])
                    .with_raw("❌"),
            )
            .to_html_string()
    }
}

fn get_all(_r: Request, _p: Params) -> Result<impl IntoResponse> {
    let connection = Connection::open_default()?;

    let row_set = connection.execute("SELECT name FROM PRAGMA_TABLE_INFO('ITEMS')", &[])?;
    let row_set = row_set.rows.iter().filter(|row| {
        row.get::<&str>(0).unwrap() != String::from("VALUE")
    });

    let items = row_set
        .map(|row| {
            Item {
            value: row.get::<&str>(0).unwrap().to_owned(),
        }})
        .map(|item| item.to_html_string())
        .reduce(|acc, e| format!("{} {}", acc, e))
        .unwrap_or(String::from(""));

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(items)
        .build())
}

fn add_new(req: Request, _params: Params) -> Result<impl IntoResponse> {
    let Ok(item): Result<Item> =
        serde_json::from_reader(req.body()).with_context(|| "Error while deserializing payload")
    else {
        return Ok(Response::new(400, "Invalid payload received"));
    };
    let connection = Connection::open_default()?;

    connection.execute("BEGIN TRANSACTION;", &[])?;
    let command = format!("ALTER TABLE ITEMS ADD {:?} VARCHAR(20);", item.value);
    if let core::result::Result::Err(err) = connection.execute(&command, &[]) {
        println!("error: {}", err);
    }

    connection.execute("COMMIT", &[]);
    Ok(Response::builder()
        .status(200)
        .header("HX-Trigger", "newItem")
        .body(())
        .build())
}

fn delete_one(_req: Request, params: Params) -> Result<impl IntoResponse> {
    let Some(value) = params.get("value") else {
        return Ok(Response::new(404, "Missing identifier"));
    };

    let connection = Connection::open_default()?;

    let value = value.replace("%20", " ");
    let command = format!("ALTER TABLE ITEMS DROP COLUMN {:?}", value);
    Ok(
        match connection.execute(&command, &[]) {
            // HTMX requires status 200 instead of 204
            Ok(_) => Response::new(200, ()),
            Err(e) => {
                println!("Error while deleting item: {}", e);
                Response::builder()
                    .status(500)
                    .body("Error while deleting item")
                    .build()
            }
        },
    )
}
