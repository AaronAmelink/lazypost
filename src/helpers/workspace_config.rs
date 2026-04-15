use std::{collections::HashMap, path::{self, Path, PathBuf}};
use serde_json::Value;
use serde::{Serialize, Deserialize};
use std::fs::{self, File};
use std::io::BufReader;
use ratatui::style::Color;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkspaceFile {
    pub items: Vec<Item>,
}

impl WorkspaceFile {
    fn empty() -> Self {
        Self { items: vec![] }
    }
}

#[derive(Debug)]
pub struct WorkspaceConfig {
    pub data: WorkspaceFile,
    pub path: PathBuf,
}

impl WorkspaceConfig {
    pub fn create_from_file(path: &Path) -> std::io::Result<Self> {
        let data = if path.exists() {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap_or_else(|_| WorkspaceFile::empty())
        } else {
            WorkspaceFile::empty()
        };

        let cfg = Self { data, path: path.to_path_buf() };

        if !path.exists() {
            cfg.save()?;
        }

        Ok(cfg)
    }

    pub fn create_from_items(items: Vec<Item>, path: &Path) -> Self {
            Self {
                data: WorkspaceFile { items },
                path: path.to_path_buf(),
            }
        }


    pub fn save_items_to_file(items: Vec<Item>, path: &Path) -> std::io::Result<()> {
        let workspace_items = Self::create_from_items(items, path);
        workspace_items.save()
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&self.path, json)
    }

    pub(crate) fn remove_from_items(items: &mut Vec<Item>, path: &[usize]) -> Result<(), &'static str> {
        if path.is_empty() {
            return Err("Path is empty");
        }
        if path.len() == 1 {
            if path[0] >= items.len() {
                return Err("Path out of bounds");
            }
            items.remove(path[0]);
            return Ok(());
        }
        let first = path[0];
        if first >= items.len() {
            return Err("Path out of bounds");
        }
        match &mut items[first] {
            Item::Folder(folder) => Self::remove_from_items(&mut folder.items, &path[1..]),
            Item::Request(_) => Err("Path out of bounds"),
        }
    }
}
