use env_logger;
use ffxiv_item_name_database_api::model::{
    convert_dynamodb_item_to_item, get_table_name, parse_query, sort_func, HttpErrorType, Item,
};
use lambda_http::{handler, lambda, Context, IntoResponse, Request, Response};
use log::{info, warn};
use rusoto_core::Region;
use rusoto_dynamodb::{
    AttributeValue, BatchGetItemInput, DynamoDb, DynamoDbClient, KeysAndAttributes,
};
use serde::Serialize;
use std::collections::HashMap;

type Error = Box<dyn std::error::Error + Sync + Send + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda::run(handler(lambda_handler)).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct Condition {
    ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct ResponseData {
    condition: Condition,
    results: Vec<Item>,
}

async fn lambda_handler(event: Request, _: Context) -> Result<impl IntoResponse, Error> {
    match env_logger::try_init() {
        Err(e) => warn!("error occurred in env_logger::try_init(): {}", e),
        Ok(_) => (),
    };
    info!("event: {:?}", event);
    let query = parse_query(&event);
    let ids = match parse_ids(&query) {
        Err(e) => return Ok(e.create_response()),
        Ok(ids) => ids,
    };

    let table_name = match get_table_name() {
        Err(e) => return Ok(e.create_response()),
        Ok(name) => name,
    };
    info!("table name: {}", table_name);
    let filtered = match get_data(&ids, &table_name).await {
        Err(e) => return Ok(e.create_response()),
        Ok(data) => data,
    };

    let body = ResponseData {
        condition: Condition { ids: ids },
        results: filtered,
    };

    Ok(Response::builder()
        .status(200)
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).unwrap())
        .expect("failed"))
}

fn parse_ids(query: &HashMap<String, String>) -> Result<Vec<u32>, HttpErrorType> {
    let mut result: Vec<u32> = Vec::new();

    let text: &String = match query.get("ids") {
        None => return Err(HttpErrorType::BadRequest("ids is required.".to_string())),
        Some(text) => text,
    };

    for raw in text.split(",") {
        let num = match raw.parse::<u32>() {
            Err(_) => {
                return Err(HttpErrorType::BadRequest(
                    "ids must be comma separated numbers".to_string(),
                ))
            }
            Ok(num) => num,
        };
        result.push(num);
    }

    Ok(result)
}

async fn get_data(ids: &Vec<u32>, table_name: &String) -> Result<Vec<Item>, HttpErrorType> {
    let mut result: Vec<Item> = Vec::new();
    let client = DynamoDbClient::new(Region::default());

    for chunk in ids.chunks(100) {
        let mut keys_and_attributes: Option<KeysAndAttributes> = Some(KeysAndAttributes {
            keys: chunk
                .iter()
                .map(|id| {
                    let mut map: HashMap<String, AttributeValue> = HashMap::new();
                    let attr = AttributeValue {
                        n: Some(id.to_string()),
                        ..Default::default()
                    };

                    map.insert("ID".to_string(), attr);

                    map
                })
                .collect(),
            ..Default::default()
        });
        while keys_and_attributes.is_some() {
            let current: KeysAndAttributes = keys_and_attributes.unwrap();

            let mut request_items: HashMap<String, KeysAndAttributes> = HashMap::new();
            request_items.insert(table_name.clone(), current);
            let input = BatchGetItemInput {
                request_items: request_items,
                ..Default::default()
            };

            let resp = match client.batch_get_item(input).await {
                Err(e) => {
                    return Err(HttpErrorType::InternalServerError(format!(
                        "failed fetch data: {}",
                        e
                    )))
                }
                Ok(resp) => resp,
            };
            match resp.responses {
                None => (),
                Some(table_response) => match table_response.get(table_name) {
                    None => (),
                    Some(items) => {
                        for item in items {
                            match convert_dynamodb_item_to_item(item) {
                                Err(e) => return Err(e),
                                Ok(item) => result.push(item),
                            }
                        }
                    }
                },
            };

            keys_and_attributes = match resp.unprocessed_keys {
                None => None,
                Some(unprocessed) => match unprocessed.get(table_name) {
                    None => None,
                    Some(keys) => Some(keys.clone()),
                },
            };
        }
    }

    result.sort_by(sort_func);

    Ok(result)
}
