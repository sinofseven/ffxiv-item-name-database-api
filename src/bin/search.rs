use lambda_http::{handler, lambda, Context, IntoResponse, Request, Response};
use std::collections::HashMap;

use ffxiv_item_name_database_api::model::{
    load_database, parse_query, HttpErrorType, Item, Language,
};
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
    let all_items = match load_database() {
        Err(e) => return Ok(e.create_response()),
        Ok(data) => data,
    };
    let filtered = filter_and_sort(&all_items, &lang, &string);
    let body = ResponseData {
        condition: Condition {
            language: lang.to_string(),
            string: string.clone(),
        },
        results: filtered,
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
