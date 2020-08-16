use ffxiv_item_name_database_api::model::{load_database, parse_query, HttpErrorType, Item};
use lambda_http::{handler, lambda, Context, IntoResponse, Request, Response};
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
    let query = parse_query(&event);
    let ids = match parse_ids(&query) {
        Err(e) => return Ok(e.create_response()),
        Ok(ids) => ids,
    };
    let all_items = match load_database() {
        Err(e) => return Ok(e.create_response()),
        Ok(data) => data,
    };
    let filtered = filter_and_sort(&all_items, &ids);

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

fn filter_and_sort(list: &Vec<Item>, ids: &Vec<u32>) -> Vec<Item> {
    let mut filtered: Vec<Item> = list
        .iter()
        .filter(|&item| ids.contains(&item.id))
        .cloned()
        .collect();

    filtered.sort_by(|a, b| {
        let a_id = a.get_item_name_category_id();
        let b_id = b.get_item_name_category_id();
        a_id.cmp(&b_id)
    });

    filtered
}
