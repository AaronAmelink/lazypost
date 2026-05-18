use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestType {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl RequestType {
    pub fn as_str(&self) -> &str {
        match self {
            RequestType::Get => "GET",
            RequestType::Post => "POST",
            RequestType::Put => "PUT",
            RequestType::Delete => "DELETE",
            RequestType::Patch => "PATCH",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            RequestType::Get => Color::Blue,
            RequestType::Post => Color::Green,
            RequestType::Put => Color::Yellow,
            RequestType::Delete => Color::Red,
            RequestType::Patch => Color::Cyan,
        }
    }
}

/// One entry in the workspace tree.
///
/// `Request` is meaningfully larger than `Folder` (HashMaps + Strings + JSON
/// values), so this enum is variant-sized. At workspace scale (hundreds of
/// items max) the wasted bytes are negligible and boxing would just add an
/// allocation per item — accept the size diff.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
#[allow(clippy::large_enum_variant)]
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
    /// JSON template with `%var_name%` placeholders. After a successful response,
    /// the template is walked in parallel with the actual response body; any
    /// placeholders capture the matching JSON value into the active environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<String>,
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
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        location: ApiKeyLocation,
    },
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
    #[serde(default)]
    pub grant: OAuth2Grant,
    pub client_id: String,
    pub client_secret: String,
    pub token_url: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Most recently fetched access token. Never persisted — runtime cache only,
    /// because workspace.json is committed to git and tokens are secrets.
    #[serde(skip)]
    pub access_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OAuth2Grant {
    #[default]
    ClientCredentials,
    RefreshToken,
}
