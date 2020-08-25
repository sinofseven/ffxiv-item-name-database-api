use env_logger;
use lambda_http::{handler, lambda, Context, IntoResponse, Request, Response};
use log::{info, warn};
use std::collections::HashMap;
use lambda::{handler_fn, Context as LambdaContext};
use lambda as lambda_runtime;

use ffxiv_item_name_database_api::model::{
    convert_dynamodb_item_to_item, get_table_name, parse_query, sort_func, HttpErrorType, Item,
    Language,
};
use maplit::hashmap;
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, ScanInput};
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use serde_json::Value;

type Error = Box<dyn std::error::Error + Sync + Send + 'static>;

#[derive(Debug, Serialize)]
struct Condition {
    string: String,
    language: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct ResponseData {
    condition: Condition,
    results: Vec<Item>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(handler_fn(lambda_handler_v2)).await?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Query {
    language: Option<String>,
    string: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct RequestDataTry {
    #[serde(rename = "queryStringParameters")]
    query_string_parameters: Option<Query>
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseDataTry {
    #[serde(rename = "statusCode")]
    status_code: u32,
    headers: HashMap<String, String>,
    body: String
}

async fn lambda_handler_v2(event: RequestDataTry, _context: LambdaContext) -> Result<ResponseDataTry, Error> {
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    map.insert("Content-Type".to_string(), "application/json".to_string());
    let body = serde_json::to_string(&event).unwrap();
    Ok(ResponseDataTry {
        status_code: 200,
        headers: map,
        body: body
    })
}

async fn lambda_handler(event: Request, _: Context) -> Result<impl IntoResponse, Error> {
    match env_logger::try_init() {
        Err(e) => warn!("error occurred in env_logger::try_init(): {}", e),
        Ok(_) => (),
    };
    info!("event: {:?}", event);
    let query = parse_query(&event);
    info!("query: {:?}", query);
    let (lang, string) = match parse_condition(&query) {
        Err(e) => return Ok(e.create_response()),
        Ok(result) => result,
    };
    let table_name = match get_table_name() {
        Err(e) => return Ok(e.create_response()),
        Ok(name) => name,
    };
    let items = match scan_and_sort(&lang, &string, &table_name).await {
        Err(e) => return Ok(e.create_response()),
        Ok(items) => items,
    };
    let body = ResponseData {
        condition: Condition {
            language: lang.to_string(),
            string: string.clone(),
        },
        results: items,
    };
    Ok(Response::builder()
        .status(200)
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).unwrap())
        .expect("failed"))
}

fn parse_condition(query: &HashMap<String, String>) -> Result<(Language, String), HttpErrorType> {
    let lang: Language = match query.get("language") {
        None => {
            return Err(HttpErrorType::BadRequest(
                "language is required.".to_string(),
            ))
        }
        Some(lang) => match Language::from_str(lang) {
            Err(_) => {
                return Err(HttpErrorType::BadRequest(format!(
                    "language '{}' is invalid.",
                    lang
                )))
            }
            Ok(lang) => lang,
        },
    };
    let string: String = match query.get("string") {
        None => return Err(HttpErrorType::BadRequest("string is required.".to_string())),
        Some(string) => string.clone(),
    };
    Ok((lang, string))
}

async fn scan_and_sort(
    lang: &Language,
    string: &String,
    table_name: &String,
) -> Result<Vec<Item>, HttpErrorType> {
    let mut result: Vec<Item> = Vec::new();

    let client = DynamoDbClient::new(Region::default());
    let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

    while {
        let input = ScanInput {
            table_name: table_name.clone(),
            filter_expression: Some("contains(#path, :value)".to_string()),
            expression_attribute_names: Some(hashmap! {
                "#path".to_string() => lang.get_key()
            }),
            expression_attribute_values: Some(hashmap! {
                ":value".to_string() => AttributeValue {
                    s: Some(string.clone()),
                    ..Default::default()
                }
            }),
            exclusive_start_key: last_evaluated_key.clone(),
            ..Default::default()
        };

        let resp = match client.scan(input).await {
            Err(e) => {
                return Err(HttpErrorType::InternalServerError(format!(
                    "error occurred in scan: {}",
                    e
                )))
            }
            Ok(resp) => resp,
        };

        last_evaluated_key = resp.last_evaluated_key;

        let items: Vec<HashMap<String, AttributeValue>> = match resp.items {
            None => Vec::new(),
            Some(items) => items,
        };

        for item in items {
            result.push(match convert_dynamodb_item_to_item(&item) {
                Err(e) => return Err(e),
                Ok(item) => item,
            });
        }

        last_evaluated_key.is_some()
    } {}

    result.sort_by(sort_func);

    Ok(result)
}
