use lambda_http::{handler, lambda, Context, IntoResponse, Request, Response};
use std::collections::HashMap;

use ffxiv_item_name_database_api::model::{load_database, parse_query, HttpErrorType, Item, Language, ItemSearchCategory};
use serde::Serialize;
use std::str::FromStr;
use rusoto_dynamodb::{AttributeValue, DynamoDbClient, ScanInput, DynamoDb};
use rusoto_core::Region;
use maplit::hashmap;
use serde_json::ser::CharEscape::CarriageReturn;

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
    let items = match scan_and_sort(&lang, &string).await {
        Err(e) => return Ok(e.create_response()),
        Ok(items) => items
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

fn filter_and_sort(list: &Vec<Item>, lang: &Language, string: &String) -> Vec<Item> {
    let mut filtered: Vec<Item> = list
        .iter()
        .filter(|&item| item.get_name(lang).contains(string))
        .cloned()
        .collect();
    filtered.sort_by(|a, b| {
        let a_id = a.get_item_name_category_id();
        let b_id = b.get_item_name_category_id();
        a_id.cmp(&b_id)
    });

    filtered
}

fn convert_item(item: &HashMap<String, AttributeValue>) -> Result<Item, HttpErrorType> {
    let item_search_category = item.get("ItemSearchCategory");
    Ok(Item {
        id: match item.get("ID") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(id) => match &id.n {
                None => return Err(HttpErrorType::InternalServerError),
                Some(id) => match id.parse::<u32>() {
                    Err(_) => return Err(HttpErrorType::InternalServerError),
                    Ok(id) => id
                }
            }
        },
        icon: match item.get("Icon") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(icon) => match &icon.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(icon) => icon.clone()
            }
        },
        item_search_category: ItemSearchCategory {
            id: match item_search_category {
                None => None,
                Some(id) => match &id.n {
                    None => None,
                    Some(id) => match id.parse::<u32>() {
                        Err(_) => return Err(HttpErrorType::InternalServerError),
                        Ok(id) => Some(id)
                    }
                }
            },
            name: match item_search_category {
                None => None,
                Some(name) => match &name.s {
                    None => None,
                    Some(name) => Some(name.clone())
                }
            }
        },
        name_de: match item.get("Name_de") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone()
            }
        },
        name_en: match item.get("Name_en") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone()
            }
        },
        name_fr: match item.get("Name_fr") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone()
            }
        },
        name_ja: match item.get("Name_ja") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone()
            }
        },
        eorzea_database_id: match item.get("EorzeaDatabaseId") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone()
            }
        }
    })
}

async fn scan_and_sort(lang: &Language, string: &String) -> Result<Vec<Item>, HttpErrorType> {
    let mut result: Vec<Item> = Vec::new();

    let client = DynamoDbClient::new(Region::default());
    let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

    while {
        let input = ScanInput {
            table_name: "ffxiv-item-name-database-resources-ItemDataTable-11T4AVHK7AKRA".to_string(),
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
                return Err(HttpErrorType::InternalServerError)
            },
            Ok(resp) => resp
        };

        last_evaluated_key = resp.last_evaluated_key;

        let items: Vec<HashMap<String, AttributeValue>> = match resp.items {
            None => Vec::new(),
            Some(items) => items
        };

        for item in items {
            result.push(match convert_item(&item) {
                Err(_) => return Err(HttpErrorType::InternalServerError),
                Ok(item) => item
            });
        }

        last_evaluated_key.is_some()
    } {}

    result.sort_by(|a, b| {
        let id_a = a.get_item_name_category_id();
        let id_b = b.get_item_name_category_id();
        id_a.cmp(&id_b)
    });

    Ok(result)
}