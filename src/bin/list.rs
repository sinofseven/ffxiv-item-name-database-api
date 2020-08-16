use lambda_http::{handler, lambda, Context, IntoResponse, Request, RequestExt, Response};
use serde::Serialize;
use std::collections::HashMap;

type Error = Box<dyn std::error::Error + Sync + Send + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda::run(handler(lambda_handler)).await?;
    Ok(())
}

fn parse_query(event: &Request) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    let query = event.query_string_parameters();
    for (k, v) in query.iter() {
        println!("k={}, v={}", k, v);
        map.insert(String::from(k), String::from(v));
    }

    map
}

async fn lambda_handler(event: Request, _: Context) -> Result<impl IntoResponse, Error> {
    // `serde_json::Values` impl `IntoResponse` by default
    // creating an application/json response
    let query = parse_query(&event);
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&query).unwrap())
        .expect("failed"))
}