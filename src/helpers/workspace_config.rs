use std::{collections::HashMap, path::{Path, PathBuf}};
use crate::helpers::sidebar::{RequestType, SidebarItem, SidebarItemType};
use serde_json::Value;
use serde::{Serialize, Deserialize};
use std::fs::{self, File};
use std::io::BufReader;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum Item {
    Folder(ConfigFolder),
    Request(Request),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigFolder {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    pub name: String,
    pub request_type: RequestType,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<RequestBody>,
    pub auth: Option<Auth>,
    pub params: Option<Vec<QueryParam>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "content")]
pub enum RequestBody {
    Json(Value),
    Form(HashMap<String, String>),
    Multipart(Vec<FormField>),
    Raw(String),
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormField {
    pub key: String,
    pub value: FormValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum FormValue {
    Text(String),
    File(FileRef),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileRef {
    pub path: String,
    pub mime_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryParam {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Auth {
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { key: String, value: String, location: ApiKeyLocation },
    OAuth2(OAuth2Config),
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub token_url: String,
    pub scopes: Vec<String>,
    pub access_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Environment {
    pub name: String,
    pub variables: HashMap<String, EnvVar>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvVar {
    pub value: String,
    pub secret: bool,
    pub enabled: bool,
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

pub fn sidebar_item_to_config(si: &SidebarItem) -> Item {
    match &si.item_type {
        SidebarItemType::HTTP(http) => Item::Request(Request {
            name: si.name.clone(),
            request_type: http.label.clone(),
            url: String::new(),
            headers: None,
            body: None,
            auth: None,
            params: None,
        }),
        SidebarItemType::Folder(folder) => Item::Folder(ConfigFolder {
            name: si.name.clone(),
            items: folder.items.iter().map(sidebar_item_to_config).collect(),
        }),
    }
}

pub fn config_item_to_sidebar(item: &Item) -> SidebarItem {
    match item {
        Item::Request(req) => SidebarItem::new_http(req.name.clone(), req.request_type.clone()),
        Item::Folder(folder) => SidebarItem::new_folder(
            folder.name.clone(),
            folder.items.iter().map(config_item_to_sidebar).collect(),
        ),
    }
}


#[derive(Debug)]
pub struct WorkspaceConfig {
    pub data: WorkspaceFile,
    pub path: PathBuf,
}

impl WorkspaceConfig {
    pub fn load_or_create(path: &Path) -> std::io::Result<Self> {
        let data = if path.exists() {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            serde_json::from_reader(reader)
                .unwrap_or_else(|_| WorkspaceFile::empty())
        } else {
            WorkspaceFile::empty()
        };

        let cfg = Self {
            data,
            path: path.to_path_buf(),
        };

        if !path.exists() {
            cfg.save()?;
        }

        Ok(cfg)
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&self.path, json)
    }

    pub fn sync_from_sidebar(&mut self, items: &[SidebarItem]) -> std::io::Result<()> {
        self.data.items = items.iter().map(sidebar_item_to_config).collect();
        self.save()
    }

    pub fn to_sidebar_items(&self) -> Vec<SidebarItem> {
        self.data.items.iter().map(config_item_to_sidebar).collect()
    }
}
