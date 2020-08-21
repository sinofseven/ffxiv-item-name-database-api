use lambda_http::{handler, lambda, Context, IntoResponse, Request, Response};
use std::collections::HashMap;

use ffxiv_item_name_database_api::model::{
    convert_dynamodb_item_to_item, get_table_name, parse_query, HttpErrorType, Item,
    ItemSearchCategory, Language,
};
use maplit::hashmap;
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, ScanInput};
use serde::Serialize;
use std::str::FromStr;

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
    lambda::run(handler(lambda_handler)).await?;
    Ok(())
}

async fn lambda_handler(event: Request, _: Context) -> Result<impl IntoResponse, Error> {
    let query = parse_query(&event);
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
                println!("scan error{:?}", e);
                return Err(HttpErrorType::InternalServerError);
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
                Err(_) => return Err(HttpErrorType::InternalServerError),
                Ok(item) => item,
            });
        }

        last_evaluated_key.is_some()
    } {}

    result.sort_by(|a, b| {
        let id_a = a.get_item_search_category_id();
        let id_b = b.get_item_search_category_id();
        id_a.cmp(&id_b)
    });

    Ok(result)
}
