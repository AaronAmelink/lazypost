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

fn merge_sidebar_with_config(sidebar_items: &[SidebarItem], config_items: &[Item]) -> Vec<Item> {
    sidebar_items.iter().map(|si| {
        match &si.item_type {
            SidebarItemType::HTTP(http) => {
                // Try to find matching request in config by name
                if let Some(existing) = config_items.iter().find(|ci| {
                    if let Item::Request(req) = ci {
                        req.name == si.name
                    } else {
                        false
                    }
                }) {
                    // Keep the existing request with all its data
                    existing.clone()
                } else {
                    // Create new request from sidebar item
                    Item::Request(Request {
                        name: si.name.clone(),
                        request_type: http.label.clone(),
                        url: String::new(),
                        headers: None,
                        body: None,
                        auth: None,
                        params: None,
                    })
                }
            }
            SidebarItemType::Folder(folder) => {
                // Try to find matching folder in config by name
                if let Some(Item::Folder(existing_folder)) = config_items.iter().find(|ci| {
                    if let Item::Folder(f) = ci {
                        f.name == si.name
                    } else {
                        false
                    }
                }) {
                    // Recursively merge folder contents, preserving config data
                    Item::Folder(ConfigFolder {
                        name: si.name.clone(),
                        items: merge_sidebar_with_config(&folder.items, &existing_folder.items),
                    })
                } else {
                    // Create new folder from sidebar item
                    Item::Folder(ConfigFolder {
                        name: si.name.clone(),
                        items: merge_sidebar_with_config(&folder.items, &[]),
                    })
                }
            }
        }
    }).collect()
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

    pub fn remove_at_path(&mut self, path: Vec<usize>) -> Result<(), &'static str> {
        Self::remove_from_items(&mut self.data.items, &path)
    }

    fn remove_from_items(items: &mut Vec<Item>, path: &[usize]) -> Result<(), &'static str> {
        if path.is_empty() {
            return Err("Path is empty");
        }

        if path.len() == 1 {
            if path[0] >= items.len() {
                return Err("Path out of bounds");
            }
            items.remove(path[0]);
            Ok(())
        } else {
            let first = path[0];
            if first >= items.len() {
                return Err("Path out of bounds");
            }

            match &mut items[first] {
                Item::Folder(folder) => {
                    Self::remove_from_items(&mut folder.items, &path[1..])
                }
                Item::Request(_) => {
                    Err("Path out of bounds")
                }
            }
        }
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
        self.data.items = merge_sidebar_with_config(items, &self.data.items);
        self.save()
    }

    pub fn to_sidebar_items(&self) -> Vec<SidebarItem> {
        self.data.items.iter().map(config_item_to_sidebar).collect()
    }
}
