use serde::{Serialize, Deserialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ItemSearchCategory {
    #[serde(rename = "ID")]
    id: Option<u32>,
    #[serde(rename = "Name")]
    name: Option<String>
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
    eorzea_database_id: String
}