use lambda_http::{handler, lambda, Context, IntoResponse, Request, RequestExt, Response};
use std::collections::HashMap;

use ffxiv_item_name_database_api::model::{create_error_response, HttpErrorType, Language};
use maplit::hashmap;
use std::str::FromStr;
use serde::Serialize;

type Error = Box<dyn std::error::Error + Sync + Send + 'static>;

#[derive(Debug)]
struct Condition {
    string: String,
    language: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda::run(handler(lambda_handler)).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct TmpData {
    query: HashMap<String, String>,
    lang: String,
    string: String,
}

async fn lambda_handler(event: Request, _: Context) -> Result<impl IntoResponse, Error> {
    // `serde_json::Values` impl `IntoResponse` by default
    // creating an application/json response
    let query = parse_query(&event);
    let (lang, string) = match parse_condition(&query) {
        Err(e) => return Ok(create_error_response(e)),
        Ok(result) => result
    };
    let body = TmpData {
        query: query,
        lang: match lang {
            Language::Japanese => "japanese".to_string(),
            Language::English => "english".to_string(),
            Language::French => "french".to_string(),
            Language::Deutsch => "deutsch".to_string(),
        },
        string: string.clone()
    };
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).unwrap())
        .expect("failed"))
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
                    "language \"{}\" is invalid.",
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
