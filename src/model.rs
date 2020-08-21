use lambda_http::{Request, RequestExt, Response};
use rusoto_dynamodb::AttributeValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

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

impl Language {
    pub fn to_string(&self) -> String {
        let lang = match self {
            Language::Deutsch => "de",
            Language::French => "fr",
            Language::English => "en",
            Language::Japanese => "ja",
        };
        lang.to_string()
    }

    pub fn get_key(&self) -> String {
        let key = match self {
            Language::Deutsch => "Name_de",
            Language::French => "Name_fr",
            Language::English => "Name_en",
            Language::Japanese => "Name_ja",
        };
        key.to_string()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ItemSearchCategory {
    #[serde(rename = "ID")]
    pub id: Option<u32>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Item {
    #[serde(rename = "ID")]
    pub id: u32,
    #[serde(rename = "Icon")]
    pub icon: String,
    #[serde(rename = "ItemSearchCategory")]
    pub item_search_category: ItemSearchCategory,
    #[serde(rename = "Name_de")]
    pub name_de: String,
    #[serde(rename = "Name_en")]
    pub name_en: String,
    #[serde(rename = "Name_fr")]
    pub name_fr: String,
    #[serde(rename = "Name_ja")]
    pub name_ja: String,
    #[serde(rename = "EorzeaDatabaseId")]
    pub eorzea_database_id: String,
}

impl Item {
    pub fn get_name(&self, language: &Language) -> String {
        let name = match language {
            Language::Japanese => &self.name_ja,
            Language::English => &self.name_en,
            Language::French => &self.name_fr,
            Language::Deutsch => &self.name_de,
        };
        name.clone()
    }
    pub fn get_item_search_category_id(&self) -> u32 {
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
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .unwrap()
}

pub enum HttpErrorType {
    BadRequest(String),
    InternalServerError,
}

impl HttpErrorType {
    pub fn create_response(&self) -> Response<String> {
        match self {
            HttpErrorType::InternalServerError => {
                create_response(500, "InternalServerError", Option::None)
            }
            HttpErrorType::BadRequest(message) => {
                create_response(400, "BadRequest", Some(message.clone()))
            }
        }
    }
}

pub fn parse_query(event: &Request) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    let query = event.query_string_parameters();
    for (k, v) in query.iter() {
        map.insert(k.to_string(), v.to_string());
    }

    map
}

pub fn get_table_name() -> Result<String, HttpErrorType> {
    match env::var("TABLE_NAME") {
        Err(e) => {
            println!("failed get environment {:?}", e);
            Err(HttpErrorType::InternalServerError)
        }
        Ok(name) => Ok(name),
    }
}

pub fn convert_dynamodb_item_to_item(
    item: &HashMap<String, AttributeValue>,
) -> Result<Item, HttpErrorType> {
    println!("Item Data {:?}", item);
    let item_search_category = item.get("ItemSearchCategory");
    Ok(Item {
        id: match item.get("ID") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(id) => match &id.n {
                None => return Err(HttpErrorType::InternalServerError),
                Some(id) => match id.parse::<u32>() {
                    Err(_) => return Err(HttpErrorType::InternalServerError),
                    Ok(id) => id,
                },
            },
        },
        icon: match item.get("Icon") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(icon) => match &icon.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(icon) => icon.clone(),
            },
        },
        item_search_category: ItemSearchCategory {
            id: match item_search_category {
                None => None,
                Some(id) => match &id.n {
                    None => None,
                    Some(id) => match id.parse::<u32>() {
                        Err(_) => return Err(HttpErrorType::InternalServerError),
                        Ok(id) => Some(id),
                    },
                },
            },
            name: match item_search_category {
                None => None,
                Some(name) => match &name.s {
                    None => None,
                    Some(name) => Some(name.clone()),
                },
            },
        },
        name_de: match item.get("Name_de") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone(),
            },
        },
        name_en: match item.get("Name_en") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone(),
            },
        },
        name_fr: match item.get("Name_fr") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone(),
            },
        },
        name_ja: match item.get("Name_ja") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone(),
            },
        },
        eorzea_database_id: match item.get("EorzeaDatabaseId") {
            None => return Err(HttpErrorType::InternalServerError),
            Some(name) => match &name.s {
                None => return Err(HttpErrorType::InternalServerError),
                Some(name) => name.clone(),
            },
        },
    })
}
