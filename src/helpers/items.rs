use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use ratatui::style::Color;
use serde_json::Value;


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestType {
    Get,
    Post,
    Put,
    Delete,
}

impl RequestType {
    pub fn as_str(&self) -> &str {
        match self {
            RequestType::Get => "GET",
            RequestType::Post => "POST",
            RequestType::Put => "PUT",
            RequestType::Delete => "DELETE",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            RequestType::Get => Color::Blue,
            RequestType::Post => Color::Green,
            RequestType::Put => Color::Yellow,
            RequestType::Delete => Color::Red,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
pub enum Item {
    Folder(ConfigFolder),
    Request(Request),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ConfigFolder {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Request {
    pub name: String,
    pub request_type: RequestType,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<RequestBody>,
    pub auth: Option<Auth>,
    pub params: Option<Vec<QueryParam>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum RequestBody {
    Json(Value),
    Form(HashMap<String, String>),
    Multipart(Vec<FormField>),
    Raw(String),
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FormField {
    pub key: String,
    pub value: FormValue,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum FormValue {
    Text(String),
    File(FileRef),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FileRef {
    pub path: String,
    pub mime_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct QueryParam {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Auth {
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { key: String, value: String, location: ApiKeyLocation },
    OAuth2(OAuth2Config),
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub token_url: String,
    pub scopes: Vec<String>,
    pub access_token: Option<String>,
}
