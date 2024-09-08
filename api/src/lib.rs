use anyhow::{Context, Result};
use build_html::{Container, ContainerType, Html, HtmlContainer};
use serde::{Deserialize, Serialize};
use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
use spin_sdk::http_component;
use spin_sdk::sqlite::{Connection, RowResult, Value};

#[http_component]
fn handle_api(req: Request) -> Result<impl IntoResponse> {
    // lets use the Router to handle requests based on method and path
    let mut r = Router::default();
    r.post("/api/items", add_new);
    r.get("/api/items", get_all);
    r.delete("/api/items/:value", delete_one);
    r.post("/api/servers", add_server);
    r.get("/api/servers", get_server);
    r.delete("/api/servers/:name", delete_from_server);
    Ok(r.handle(req))
}

#[derive(Debug, Deserialize, Serialize)]
struct Item {
    index: String,
    value: String,
    value2: String,
    data_type: String,
    action: String,
    additional: String,
}

impl Html for Item {
    fn to_html_string(&self) -> String {
        Container::new(ContainerType::Div)
            .with_attributes(vec![
                ("class", "item"),
                ("value", format!("item-{}", &self.value).as_str()),
                ("value2", format!("item-{}", &self.value2).as_str()),
            ])
            .with_container(
                Container::new(ContainerType::Div)
                    .with_attributes(vec![("class", "value")])
                    .with_raw(format!("{}: {}", &self.value, &self.value2)),
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
    let row_set = row_set
        .rows
        .iter()
        .filter(|row| row.get::<&str>(0).unwrap() != String::from("VALUE"));

    let items = row_set
        .map(|row| {
            let value = row.get::<&str>(0).unwrap().to_owned();
            let value2_command = 
                connection
                    .execute(
                        &format!(
                            "SELECT \"key\", value FROM key_value WHERE \"key\" LIKE {:?}",
                            value
                        ),
                        &[],
                    ).unwrap();
            let value2 = match value2_command.rows.get(0) {
                Some(row_result) => {
                    row_result.get::<&str>(1).unwrap().to_owned()
                }
                None => {
                    String::new()
                }
            };
            Item {
                value,
                value2,
                index: String::new(),
                data_type: String::new(),
                action: String::new(),
                additional: String::new(),
            }
        })
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
        println!("Invalid payload");
        return Ok(Response::new(400, "Invalid payload received"));
    };

    let connection = Connection::open_default()?;

    println!("action: {:?}", item.action);
    println!("additional: {:?}", item.additional);

    connection.execute("BEGIN TRANSACTION;", &[])?;
    let command = format!("ALTER TABLE ITEMS ADD {:?} VARCHAR(20);", item.value);
    if let core::result::Result::Err(err) = connection.execute(&command, &[]) {
        println!("error: {}", err);
    }
    connection.execute("COMMIT", &[]);

    let command = format!(
        "INSERT INTO key_value (key, id, value, type, command, parameter) VALUES({:?}, {:?}, {:?}, {:?}, {:?}, {:?}) RETURNING *;",
        item.value, item.index.parse::<u32>().unwrap(), item.value2, item.data_type, item.action, item.additional
    );
    if let core::result::Result::Err(err) = connection.execute(&command, &[]) {
        println!("error: {}", err);
    }

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

    let command = format!("DELETE FROM key_value WHERE \"key\"={:?}", value);
    connection.execute(&command, &[]).unwrap();

    let command = format!("ALTER TABLE ITEMS DROP COLUMN {:?}", value);
    Ok(match connection.execute(&command, &[]) {
        // HTMX requires status 200 instead of 204
        Ok(_) => Response::new(200, ()),
        Err(e) => {
            println!("Error while deleting item: {}", e);
            Response::builder()
                .status(500)
                .body("Error while deleting item")
                .build()
        }
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct Server {
    pub name: String,
    pub serial_number: String,
}

impl Html for Server {
    fn to_html_string(&self) -> String {
        Container::new(ContainerType::Div)
            .with_attributes(vec![
                ("class", "server"),
                ("name", format!("server-{}", &self.name).as_str()),
                ("serial_number", format!("server-{}", &self.serial_number).as_str()),
            ])
            .with_container(
                Container::new(ContainerType::Div)
                    .with_attributes(vec![("class", "name")])
                    .with_raw(format!("{}: {}", &self.name, &self.serial_number)),
            )
            .with_container(
                Container::new(ContainerType::Div)
                    .with_attributes(vec![
                        ("class", "delete-server"),
                        ("hx-delete", format!("/api/servers/{}", &self.name).as_str()),
                    ])
                    .with_raw("❌"),
            )
            .to_html_string()
    }
}

fn get_server(_r: Request, _p: Params) -> Result<impl IntoResponse> {
    let connection = Connection::open_default()?;
    let row_set = connection.execute("SELECT * FROM server_id", &[])?;

    let servers = row_set.rows.into_iter().map(|row| {
        let name = row.get::<&str>(0).unwrap();
        let serial_number = row.get::<u32>(1).unwrap();
        Server {
            name: name.to_string(),
            serial_number: serial_number.to_string(),
        }
    }).map(|server| server.to_html_string())
    .reduce(|acc, e| format!("{} {}", acc, e))
    .unwrap_or(String::from(""));
    
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(servers)
        .build())
}

fn add_server(req: Request, _params: Params) -> Result<impl IntoResponse> {
    let Ok(server): Result<Server> =
        serde_json::from_reader(req.body()).with_context(|| "Error while deserializing payload")
    else {
        println!("Invalid payload");
        return Ok(Response::new(400, "Invalid payload received"));
    };

    let connection = Connection::open_default()?;
    let command = format!(
        "INSERT INTO server_id (server, id) VALUES({:?}, {:?}) RETURNING *;",
        server.name, server.serial_number.parse::<u32>().unwrap()
    );
    if let core::result::Result::Err(err) = connection.execute(&command, &[]) {
        println!("error: {}", err);
    }

    Ok(Response::builder()
        .status(200)
        .header("HX-Trigger", "newServer")
        .body(())
        .build())
}

fn delete_from_server(_req: Request, params: Params) -> Result<impl IntoResponse> {
    let Some(name) = params.get("name") else {
        return Ok(Response::new(404, "Missing identifier"));
    };

    let connection = Connection::open_default()?;

    let name = name.replace("%20", " ");

    let command = format!("DELETE FROM server_id WHERE \"server\"={:?}", name);
    Ok(match connection.execute(&command, &[]) {
        // HTMX requires status 200 instead of 204
        Ok(_) => Response::new(200, ()),
        Err(e) => {
            println!("Error while deleting item: {}", e);
            Response::builder()
                .status(500)
                .body("Error while deleting item")
                .build()
        }
    })
}
