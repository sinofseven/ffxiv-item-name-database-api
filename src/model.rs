use lambda_http::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;

type Error = Box<dyn std::error::Error + Sync + Send + 'static>;

#[derive(Debug)]
pub enum Language {
    Deutsch,
    French,
    English,
    Japanese,
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = match s {
            "de" => Language::Deutsch,
            "fr" => Language::French,
            "en" => Language::English,
            "ja" => Language::Japanese,
            _ => return Err("invalid lang code".to_string()),
        };
        Ok(name)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ItemSearchCategory {
    #[serde(rename = "ID")]
    id: Option<u32>,
    #[serde(rename = "Name")]
    name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Item {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "Icon")]
    icon: String,
    #[serde(rename = "ItemSearchCategory")]
    item_search_category: ItemSearchCategory,
    #[serde(rename = "Name_de")]
    name_de: String,
    #[serde(rename = "Name_en")]
    name_en: String,
    #[serde(rename = "Name_fr")]
    name_fr: String,
    #[serde(rename = "Name_ja")]
    name_ja: String,
    #[serde(rename = "EorzeaDatabaseId")]
    eorzea_database_id: String,
}

impl Item {
    fn get_name(&self, language: Language) -> String {
        let name = match language {
            Language::Japanese => &self.name_ja,
            Language::English => &self.name_en,
            Language::French => &self.name_fr,
            Language::Deutsch => &self.name_de,
        };
        name.clone()
    }
    fn get_item_name_category_id(&self) -> u32 {
        match self.item_search_category.id {
            Some(num) => num,
            None => 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorBody {
    #[serde(rename = "type")]
    error_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn create_response(status: u16, error: &str, message: Option<String>) -> Response<String> {
    let data = ErrorBody {
        error_type: error.to_string(),
        message: message,
    };
    let body = serde_json::to_string(&data).unwrap();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(body)
        .unwrap()
}

pub enum HttpErrorType {
    BadRequest(String),
    InternalServerError
}

pub fn create_error_response(http_error_type: HttpErrorType) -> Response<String> {
    match http_error_type {
        HttpErrorType::BadRequest(message) => {
            create_response(400, "BadRequest", Option::from(message.clone()))
        },
        HttpErrorType::InternalServerError => {
            create_response(500, "InternalServerError", Option::None)
        }
    }
}


pub fn load_database() -> Result<Vec<Item>, HttpErrorType> {
    let file = match File::open("/opt/database.json") {
        Err(_) => return Err(HttpErrorType::InternalServerError),
        Ok(file) => file
    };
    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
        Err(_) => return Err(HttpErrorType::InternalServerError),
        Ok(data) => Ok(data)
    }
}