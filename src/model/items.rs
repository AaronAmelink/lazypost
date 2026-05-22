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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_vars: Option<Vec<QueryParam>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use serde_json::json;

    // --- RequestType accessors ---

    #[test]
    fn request_type_as_str() {
        assert_eq!(RequestType::Get.as_str(), "GET");
        assert_eq!(RequestType::Post.as_str(), "POST");
        assert_eq!(RequestType::Put.as_str(), "PUT");
        assert_eq!(RequestType::Delete.as_str(), "DELETE");
        assert_eq!(RequestType::Patch.as_str(), "PATCH");
    }

    #[test]
    fn request_type_color() {
        assert_eq!(RequestType::Get.color(), Color::Blue);
        assert_eq!(RequestType::Post.color(), Color::Green);
        assert_eq!(RequestType::Put.color(), Color::Yellow);
        assert_eq!(RequestType::Delete.color(), Color::Red);
        assert_eq!(RequestType::Patch.color(), Color::Cyan);
    }

    // --- serde round-trips ---

    fn round_trip<T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug>(v: &T) {
        let json = serde_json::to_string(v).unwrap();
        let back: T = serde_json::from_str(&json).unwrap();
        assert_eq!(*v, back);
    }

    #[test]
    fn request_type_round_trips() {
        for rt in [
            RequestType::Get,
            RequestType::Post,
            RequestType::Put,
            RequestType::Delete,
            RequestType::Patch,
        ] {
            round_trip(&rt);
        }
    }

    #[test]
    fn auth_bearer_round_trip() {
        round_trip(&Auth::Bearer {
            token: "tok".into(),
        });
    }

    #[test]
    fn auth_basic_round_trip() {
        round_trip(&Auth::Basic {
            username: "user".into(),
            password: "pass".into(),
        });
    }

    #[test]
    fn auth_api_key_header_round_trip() {
        round_trip(&Auth::ApiKey {
            key: "X-Api-Key".into(),
            value: "secret".into(),
            location: ApiKeyLocation::Header,
        });
    }

    #[test]
    fn auth_api_key_query_round_trip() {
        round_trip(&Auth::ApiKey {
            key: "api_key".into(),
            value: "secret".into(),
            location: ApiKeyLocation::Query,
        });
    }

    #[test]
    fn request_body_json_round_trip() {
        round_trip(&RequestBody::Json(json!({"a": 1, "b": [1, 2]})));
    }

    #[test]
    fn request_body_raw_round_trip() {
        round_trip(&RequestBody::Raw("raw text".into()));
    }

    #[test]
    fn request_body_none_round_trip() {
        round_trip(&RequestBody::None);
    }

    #[test]
    fn item_folder_round_trip() {
        let item = Item::Folder(ConfigFolder {
            name: "my folder".into(),
            items: vec![],
        });
        round_trip(&item);
    }

    #[test]
    fn item_request_round_trip() {
        let item = Item::Request(Request {
            name: "get users".into(),
            request_type: RequestType::Get,
            url: "https://api.example.com/users".into(),
            headers: None,
            body: None,
            auth: None,
            params: None,
            url_vars: None,
            capture: None,
        });
        round_trip(&item);
    }

    #[test]
    fn oauth2_grant_defaults_to_client_credentials() {
        assert_eq!(OAuth2Grant::default(), OAuth2Grant::ClientCredentials);
    }

    #[test]
    fn query_param_enabled_flag_round_trips() {
        round_trip(&QueryParam {
            key: "k".into(),
            value: "v".into(),
            enabled: false,
        });
    }
}
