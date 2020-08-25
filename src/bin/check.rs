use lambda_http::{lambda::{Context, handler_fn, run}};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type Error = Box<dyn std::error::Error + Sync + Send + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler_fn(lambda_handler)).await?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Query {
    language: Option<String>,
    string: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RequestDataTry {
    #[serde(rename = "queryStringParameters")]
    query_string_parameters: Option<Query>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseDataTry {
    #[serde(rename = "statusCode")]
    status_code: u32,
    headers: HashMap<String, String>,
    body: String,
}

async fn lambda_handler(
    event: RequestDataTry,
    _context: Context,
) -> Result<ResponseDataTry, Error> {
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("Content-Type".to_string(), "application/json".to_string());
    let body = serde_json::to_string(&event).unwrap();
    Ok(ResponseDataTry {
        status_code: 200,
        headers: map,
        body: body,
    })
}
