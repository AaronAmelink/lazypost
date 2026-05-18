use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Tabs as TabsWidget};
use ratatui::{Frame, symbols};

use crate::helpers::body_editor::{BodyEditor, EditorMode};
use crate::helpers::items::{
    ApiKeyLocation, Auth, FileRef, FormField, FormValue, OAuth2Config, OAuth2Grant, QueryParam,
    Request, RequestBody, RequestType,
};
use crate::helpers::text_input::TextInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Info,
    Auth,
    Body,
    Params,
    Headers,
    Capture,
}

impl Tab {
    const ALL: &'static [Tab] = &[
        Tab::Info,
        Tab::Auth,
        Tab::Body,
        Tab::Params,
        Tab::Headers,
        Tab::Capture,
    ];

    fn titles() -> Vec<Line<'static>> {
        Self::ALL.iter().map(|t| Line::from(t.label())).collect()
    }

    fn label(self) -> &'static str {
        match self {
            Tab::Info => " Info ",
            Tab::Auth => " Auth ",
            Tab::Body => " Body ",
            Tab::Params => " Params ",
            Tab::Headers => " Headers ",
            Tab::Capture => " Capture ",
        }
    }
}

const METHODS: &[RequestType] = &[
    RequestType::Get,
    RequestType::Post,
    RequestType::Put,
    RequestType::Patch,
    RequestType::Delete,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    None,
    Bearer,
    Basic,
    ApiKey,
    OAuth2,
}

const AUTH_KINDS: &[AuthKind] = &[
    AuthKind::None,
    AuthKind::Bearer,
    AuthKind::Basic,
    AuthKind::ApiKey,
    AuthKind::OAuth2,
];

impl AuthKind {
    fn label(self) -> &'static str {
        match self {
            AuthKind::None => "None",
            AuthKind::Bearer => "Bearer",
            AuthKind::Basic => "Basic",
            AuthKind::ApiKey => "API Key",
            AuthKind::OAuth2 => "OAuth2",
        }
    }
}

const OAUTH2_GRANTS: &[OAuth2Grant] = &[OAuth2Grant::ClientCredentials, OAuth2Grant::RefreshToken];

fn oauth2_grant_label(g: &OAuth2Grant) -> &'static str {
    match g {
        OAuth2Grant::ClientCredentials => "client_credentials",
        OAuth2Grant::RefreshToken => "refresh_token",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    None,
    Raw,
    Json,
    Form,
    Multipart,
}

const BODY_KINDS: &[BodyKind] = &[
    BodyKind::None,
    BodyKind::Raw,
    BodyKind::Json,
    BodyKind::Form,
    BodyKind::Multipart,
];

impl BodyKind {
    fn label(self) -> &'static str {
        match self {
            BodyKind::None => "None",
            BodyKind::Raw => "Raw",
            BodyKind::Json => "JSON",
            BodyKind::Form => "Form",
            BodyKind::Multipart => "Multipart",
        }
    }
}

const API_KEY_LOCATIONS: &[ApiKeyLocation] = &[ApiKeyLocation::Header, ApiKeyLocation::Query];

fn api_key_location_label(loc: &ApiKeyLocation) -> &'static str {
    match loc {
        ApiKeyLocation::Header => "Header",
        ApiKeyLocation::Query => "Query",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusId {
    // Info tab
    MethodSelector,
    NameInput,
    UrlInput,
    // Auth tab
    AuthKindSelector,
    AuthBearerToken,
    AuthBasicUser,
    AuthBasicPass,
    AuthApiKey,
    AuthApiValue,
    AuthApiKeyLocation,
    AuthOAuth2Grant,
    AuthOAuth2ClientId,
    AuthOAuth2ClientSecret,
    AuthOAuth2TokenUrl,
    AuthOAuth2Scopes,
    AuthOAuth2RefreshToken,
    // Body tab
    BodyKindSelector,
    BodyText,                 // shared by Raw + Json
    BodyFormRow(usize, bool), // (index, is_value)
    BodyMultipartKind(usize), // text/file toggle
    BodyMultipartKey(usize),
    BodyMultipartValue(usize),
    // Capture tab
    CaptureText,
    // Params tab
    ParamEnabled(usize),
    ParamKey(usize),
    ParamValue(usize),
    // Headers tab
    HeaderKey(usize),
    HeaderValue(usize),
}

#[derive(Debug, Clone)]
pub struct ParamRow {
    pub key: TextInput,
    pub value: TextInput,
    pub enabled: bool,
}

impl ParamRow {
    fn new(key_title: &str, val_title: &str) -> Self {
        Self {
            key: TextInput::new(key_title.to_owned()),
            value: TextInput::new(val_title.to_owned()),
            enabled: true,
        }
    }

    fn from(k: &str, v: &str, enabled: bool) -> Self {
        let mut row = Self::new("Key", "Value");
        row.key.set_value(k);
        row.value.set_value(v);
        row.enabled = enabled;
        row
    }
}

#[derive(Debug, Clone)]
pub struct MultipartRow {
    pub key: TextInput,
    pub value: TextInput,
    pub is_file: bool,
}

impl MultipartRow {
    fn new(key_title: &str, val_title: &str) -> Self {
        Self {
            key: TextInput::new(key_title.to_owned()),
            value: TextInput::new(val_title.to_owned()),
            is_file: false,
        }
    }

    fn from_field(field: &FormField) -> Self {
        let mut row = Self::new("Key", "Value");
        row.key.set_value(field.key.clone());
        match &field.value {
            FormValue::Text(t) => {
                row.value.set_value(t.clone());
                row.is_file = false;
            }
            FormValue::File(f) => {
                row.value.set_value(f.path.clone());
                row.is_file = true;
            }
        }
        row
    }
}

pub struct RequestEditor {
    current_tab: usize,
    focused: Option<FocusId>,
    editing: bool,

    method_index: usize,

    name_input: TextInput,
    url_input: TextInput,

    auth_kind_index: usize,
    auth_bearer_token: TextInput,
    auth_basic_user: TextInput,
    auth_basic_pass: TextInput,
    auth_api_key: TextInput,
    auth_api_value: TextInput,
    api_key_location_index: usize,
    oauth2_grant_index: usize,
    oauth2_client_id: TextInput,
    oauth2_client_secret: TextInput,
    oauth2_token_url: TextInput,
    oauth2_scopes: TextInput,
    oauth2_refresh_token: TextInput,

    body_kind_index: usize,
    body_editor: BodyEditor,
    body_form_rows: Vec<ParamRow>,
    body_multipart_rows: Vec<MultipartRow>,

    capture_editor: BodyEditor,

    param_rows: Vec<ParamRow>,
    header_rows: Vec<ParamRow>,

    pub validation_error: Option<String>,
}

impl Default for RequestEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestEditor {
    pub fn new() -> Self {
        let mut capture_editor = BodyEditor::new();
        // Capture templates are always JSON.
        capture_editor.set_mode(EditorMode::Json);
        Self {
            current_tab: 0,
            focused: None,
            editing: false,
            method_index: 0,
            name_input: TextInput::new("Name"),
            url_input: TextInput::new("URL"),
            auth_kind_index: 0,
            auth_bearer_token: TextInput::new("Bearer Token"),
            auth_basic_user: TextInput::new("Username"),
            auth_basic_pass: TextInput::new("Password"),
            auth_api_key: TextInput::new("API Key"),
            auth_api_value: TextInput::new("API Value"),
            api_key_location_index: 0,
            oauth2_grant_index: 0,
            oauth2_client_id: TextInput::new("Client ID"),
            oauth2_client_secret: TextInput::new("Client Secret"),
            oauth2_token_url: TextInput::new("Token URL"),
            oauth2_scopes: TextInput::new("Scopes (space-separated)"),
            oauth2_refresh_token: TextInput::new("Refresh Token"),
            body_kind_index: 0,
            body_editor: BodyEditor::new(),
            body_form_rows: vec![ParamRow::new("Key 1", "Value 1")],
            body_multipart_rows: vec![MultipartRow::new("Key 1", "Value 1")],
            capture_editor,
            param_rows: vec![ParamRow::new("Key 1", "Value 1")],
            header_rows: vec![ParamRow::new("Header 1", "Value 1")],
            validation_error: None,
        }
    }

    /// JSON body kind enables bracket matching / auto-indent in `body_editor`;
    /// every other kind goes back to plain mode. Call this after every change
    /// to `body_kind_index`.
    fn apply_body_editor_mode(&mut self) {
        let mode = if BODY_KINDS[self.body_kind_index] == BodyKind::Json {
            EditorMode::Json
        } else {
            EditorMode::Plain
        };
        self.body_editor.set_mode(mode);
    }

    pub fn from_request(req: &Request) -> Self {
        let mut e = Self::new();
        e.name_input.set_value(req.name.clone());
        e.url_input.set_value(req.url.clone());
        e.method_index = METHODS
            .iter()
            .position(|m| m == &req.request_type)
            .unwrap_or(0);

        // Auth
        match req.auth.as_ref() {
            None | Some(Auth::None) => e.auth_kind_index = 0,
            Some(Auth::Bearer { token }) => {
                e.auth_kind_index = 1;
                e.auth_bearer_token.set_value(token.clone());
            }
            Some(Auth::Basic { username, password }) => {
                e.auth_kind_index = 2;
                e.auth_basic_user.set_value(username.clone());
                e.auth_basic_pass.set_value(password.clone());
            }
            Some(Auth::ApiKey {
                key,
                value,
                location,
            }) => {
                e.auth_kind_index = 3;
                e.auth_api_key.set_value(key.clone());
                e.auth_api_value.set_value(value.clone());
                e.api_key_location_index = API_KEY_LOCATIONS
                    .iter()
                    .position(|l| l == location)
                    .unwrap_or(0);
            }
            Some(Auth::OAuth2(cfg)) => {
                e.auth_kind_index = 4;
                e.oauth2_grant_index = OAUTH2_GRANTS
                    .iter()
                    .position(|g| g == &cfg.grant)
                    .unwrap_or(0);
                e.oauth2_client_id.set_value(cfg.client_id.clone());
                e.oauth2_client_secret.set_value(cfg.client_secret.clone());
                e.oauth2_token_url.set_value(cfg.token_url.clone());
                e.oauth2_scopes.set_value(cfg.scopes.join(" "));
                if let Some(rt) = &cfg.refresh_token {
                    e.oauth2_refresh_token.set_value(rt.clone());
                }
            }
        }

        // Body
        match req.body.as_ref() {
            None | Some(RequestBody::None) => e.body_kind_index = 0,
            Some(RequestBody::Raw(s)) => {
                e.body_kind_index = 1;
                e.body_editor.set_text(s);
            }
            Some(RequestBody::Json(v)) => {
                e.body_kind_index = 2;
                let pretty = serde_json::to_string_pretty(v).unwrap_or_default();
                e.body_editor.set_text(&pretty);
            }
            Some(RequestBody::Form(map)) => {
                e.body_kind_index = 3;
                e.body_form_rows = map
                    .iter()
                    .map(|(k, v)| ParamRow::from(k, v, true))
                    .collect();
                if e.body_form_rows.is_empty() {
                    e.body_form_rows.push(ParamRow::new("Key 1", "Value 1"));
                }
            }
            Some(RequestBody::Multipart(fields)) => {
                e.body_kind_index = 4;
                e.body_multipart_rows = fields.iter().map(MultipartRow::from_field).collect();
                if e.body_multipart_rows.is_empty() {
                    e.body_multipart_rows
                        .push(MultipartRow::new("Key 1", "Value 1"));
                }
            }
        }

        if let Some(params) = req.params.as_ref() {
            e.param_rows = params
                .iter()
                .map(|p| ParamRow::from(&p.key, &p.value, p.enabled))
                .collect();
            if e.param_rows.is_empty() {
                e.param_rows.push(ParamRow::new("Key 1", "Value 1"));
            }
        }

        if let Some(headers) = req.headers.as_ref() {
            e.header_rows = headers
                .iter()
                .map(|(k, v)| ParamRow::from(k, v, true))
                .collect();
            if e.header_rows.is_empty() {
                e.header_rows.push(ParamRow::new("Header 1", "Value 1"));
            }
        }

        if let Some(capture) = req.capture.as_ref() {
            e.capture_editor.set_text(capture);
        }

        e.apply_body_editor_mode();
        e
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % Tab::ALL.len();
        self.reset_focus();
    }

    fn prev_tab(&mut self) {
        self.current_tab = (self.current_tab + Tab::ALL.len() - 1) % Tab::ALL.len();
        self.reset_focus();
    }

    fn reset_focus(&mut self) {
        self.focused = None;
        self.editing = false;
        self.disable_all_inputs();
    }

    fn disable_all_inputs(&mut self) {
        self.name_input.disable();
        self.url_input.disable();
        self.auth_bearer_token.disable();
        self.auth_basic_user.disable();
        self.auth_basic_pass.disable();
        self.auth_api_key.disable();
        self.auth_api_value.disable();
        self.oauth2_client_id.disable();
        self.oauth2_client_secret.disable();
        self.oauth2_token_url.disable();
        self.oauth2_scopes.disable();
        self.oauth2_refresh_token.disable();
        self.body_editor.disable();
        self.capture_editor.disable();
        for r in &mut self.body_form_rows {
            r.key.disable();
            r.value.disable();
        }
        for r in &mut self.body_multipart_rows {
            r.key.disable();
            r.value.disable();
        }
        for r in &mut self.param_rows {
            r.key.disable();
            r.value.disable();
        }
        for r in &mut self.header_rows {
            r.key.disable();
            r.value.disable();
        }
    }

    fn enable_focused_input(&mut self) {
        self.disable_all_inputs();
        match self.focused {
            Some(FocusId::NameInput) => self.name_input.enable(),
            Some(FocusId::UrlInput) => self.url_input.enable(),
            Some(FocusId::AuthBearerToken) => self.auth_bearer_token.enable(),
            Some(FocusId::AuthBasicUser) => self.auth_basic_user.enable(),
            Some(FocusId::AuthBasicPass) => self.auth_basic_pass.enable(),
            Some(FocusId::AuthApiKey) => self.auth_api_key.enable(),
            Some(FocusId::AuthApiValue) => self.auth_api_value.enable(),
            Some(FocusId::AuthOAuth2ClientId) => self.oauth2_client_id.enable(),
            Some(FocusId::AuthOAuth2ClientSecret) => self.oauth2_client_secret.enable(),
            Some(FocusId::AuthOAuth2TokenUrl) => self.oauth2_token_url.enable(),
            Some(FocusId::AuthOAuth2Scopes) => self.oauth2_scopes.enable(),
            Some(FocusId::AuthOAuth2RefreshToken) => self.oauth2_refresh_token.enable(),
            Some(FocusId::BodyText) => self.body_editor.enable(),
            Some(FocusId::CaptureText) => self.capture_editor.enable(),
            Some(FocusId::BodyFormRow(i, is_value)) => {
                if let Some(r) = self.body_form_rows.get_mut(i) {
                    if is_value {
                        r.value.enable();
                    } else {
                        r.key.enable();
                    }
                }
            }
            Some(FocusId::BodyMultipartKey(i)) => {
                if let Some(r) = self.body_multipart_rows.get_mut(i) {
                    r.key.enable();
                }
            }
            Some(FocusId::BodyMultipartValue(i)) => {
                if let Some(r) = self.body_multipart_rows.get_mut(i) {
                    r.value.enable();
                }
            }
            Some(FocusId::ParamKey(i)) => {
                if let Some(r) = self.param_rows.get_mut(i) {
                    r.key.enable();
                }
            }
            Some(FocusId::ParamValue(i)) => {
                if let Some(r) = self.param_rows.get_mut(i) {
                    r.value.enable();
                }
            }
            Some(FocusId::HeaderKey(i)) => {
                if let Some(r) = self.header_rows.get_mut(i) {
                    r.key.enable();
                }
            }
            Some(FocusId::HeaderValue(i)) => {
                if let Some(r) = self.header_rows.get_mut(i) {
                    r.value.enable();
                }
            }
            _ => {}
        }
    }

    fn forward_to_active_input(&mut self, event: &Event) {
        match self.focused {
            Some(FocusId::NameInput) => {
                self.name_input.handle_event(event);
            }
            Some(FocusId::UrlInput) => {
                self.url_input.handle_event(event);
            }
            Some(FocusId::AuthBearerToken) => {
                self.auth_bearer_token.handle_event(event);
            }
            Some(FocusId::AuthBasicUser) => {
                self.auth_basic_user.handle_event(event);
            }
            Some(FocusId::AuthBasicPass) => {
                self.auth_basic_pass.handle_event(event);
            }
            Some(FocusId::AuthApiKey) => {
                self.auth_api_key.handle_event(event);
            }
            Some(FocusId::AuthApiValue) => {
                self.auth_api_value.handle_event(event);
            }
            Some(FocusId::AuthOAuth2ClientId) => {
                self.oauth2_client_id.handle_event(event);
            }
            Some(FocusId::AuthOAuth2ClientSecret) => {
                self.oauth2_client_secret.handle_event(event);
            }
            Some(FocusId::AuthOAuth2TokenUrl) => {
                self.oauth2_token_url.handle_event(event);
            }
            Some(FocusId::AuthOAuth2Scopes) => {
                self.oauth2_scopes.handle_event(event);
            }
            Some(FocusId::AuthOAuth2RefreshToken) => {
                self.oauth2_refresh_token.handle_event(event);
            }
            Some(FocusId::BodyText) => {
                self.body_editor.handle_event(event);
            }
            Some(FocusId::CaptureText) => {
                self.capture_editor.handle_event(event);
            }
            Some(FocusId::BodyFormRow(i, is_value)) => {
                if let Some(r) = self.body_form_rows.get_mut(i) {
                    if is_value {
                        r.value.handle_event(event);
                    } else {
                        r.key.handle_event(event);
                    }
                }
            }
            Some(FocusId::BodyMultipartKey(i)) => {
                if let Some(r) = self.body_multipart_rows.get_mut(i) {
                    r.key.handle_event(event);
                }
            }
            Some(FocusId::BodyMultipartValue(i)) => {
                if let Some(r) = self.body_multipart_rows.get_mut(i) {
                    r.value.handle_event(event);
                }
            }
            Some(FocusId::ParamKey(i)) => {
                if let Some(r) = self.param_rows.get_mut(i) {
                    r.key.handle_event(event);
                }
            }
            Some(FocusId::ParamValue(i)) => {
                if let Some(r) = self.param_rows.get_mut(i) {
                    r.value.handle_event(event);
                }
            }
            Some(FocusId::HeaderKey(i)) => {
                if let Some(r) = self.header_rows.get_mut(i) {
                    r.key.handle_event(event);
                }
            }
            Some(FocusId::HeaderValue(i)) => {
                if let Some(r) = self.header_rows.get_mut(i) {
                    r.value.handle_event(event);
                }
            }
            _ => {}
        }
    }

    fn focus_list(&self) -> Vec<FocusId> {
        match Tab::ALL[self.current_tab] {
            Tab::Info => vec![
                FocusId::MethodSelector,
                FocusId::NameInput,
                FocusId::UrlInput,
            ],
            Tab::Auth => {
                let mut out = vec![FocusId::AuthKindSelector];
                match AUTH_KINDS[self.auth_kind_index] {
                    AuthKind::None => {}
                    AuthKind::Bearer => out.push(FocusId::AuthBearerToken),
                    AuthKind::Basic => {
                        out.push(FocusId::AuthBasicUser);
                        out.push(FocusId::AuthBasicPass);
                    }
                    AuthKind::ApiKey => {
                        out.push(FocusId::AuthApiKey);
                        out.push(FocusId::AuthApiValue);
                        out.push(FocusId::AuthApiKeyLocation);
                    }
                    AuthKind::OAuth2 => {
                        out.push(FocusId::AuthOAuth2Grant);
                        out.push(FocusId::AuthOAuth2ClientId);
                        out.push(FocusId::AuthOAuth2ClientSecret);
                        out.push(FocusId::AuthOAuth2TokenUrl);
                        out.push(FocusId::AuthOAuth2Scopes);
                        if OAUTH2_GRANTS[self.oauth2_grant_index] == OAuth2Grant::RefreshToken {
                            out.push(FocusId::AuthOAuth2RefreshToken);
                        }
                    }
                }
                out
            }
            Tab::Body => {
                let mut out = vec![FocusId::BodyKindSelector];
                match BODY_KINDS[self.body_kind_index] {
                    BodyKind::None => {}
                    BodyKind::Raw | BodyKind::Json => out.push(FocusId::BodyText),
                    BodyKind::Form => {
                        for i in 0..self.body_form_rows.len() {
                            out.push(FocusId::BodyFormRow(i, false));
                            out.push(FocusId::BodyFormRow(i, true));
                        }
                    }
                    BodyKind::Multipart => {
                        for i in 0..self.body_multipart_rows.len() {
                            out.push(FocusId::BodyMultipartKind(i));
                            out.push(FocusId::BodyMultipartKey(i));
                            out.push(FocusId::BodyMultipartValue(i));
                        }
                    }
                }
                out
            }
            Tab::Params => (0..self.param_rows.len())
                .flat_map(|i| {
                    [
                        FocusId::ParamEnabled(i),
                        FocusId::ParamKey(i),
                        FocusId::ParamValue(i),
                    ]
                })
                .collect(),
            Tab::Headers => (0..self.header_rows.len())
                .flat_map(|i| [FocusId::HeaderKey(i), FocusId::HeaderValue(i)])
                .collect(),
            Tab::Capture => vec![FocusId::CaptureText],
        }
    }

    fn move_focus_down(&mut self) {
        let list = self.focus_list();
        if list.is_empty() {
            return;
        }
        self.focused = Some(match self.focused {
            None => list[0],
            Some(cur) => {
                let pos = list.iter().position(|f| *f == cur).unwrap_or(0);
                list[(pos + 1) % list.len()]
            }
        });
    }

    fn move_focus_up(&mut self) {
        let list = self.focus_list();
        if list.is_empty() {
            return;
        }
        self.focused = Some(match self.focused {
            None => *list.last().unwrap(),
            Some(cur) => {
                let pos = list.iter().position(|f| *f == cur).unwrap_or(0);
                list[(pos + list.len() - 1) % list.len()]
            }
        });
    }

    // Takes `&mut self` because validation failure updates `validation_error` so
    // the next render can show the message. clippy::wrong_self_convention only
    // expects `to_*` methods on Copy types.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_request(&mut self) -> Result<Request, String> {
        self.validation_error = None;
        let method = METHODS[self.method_index].clone();
        let name = self.name_input.value().to_string();
        let url = self.url_input.value().to_string();

        let auth = match AUTH_KINDS[self.auth_kind_index] {
            AuthKind::None => None,
            AuthKind::Bearer => {
                let t = self.auth_bearer_token.value().to_string();
                if t.is_empty() {
                    None
                } else {
                    Some(Auth::Bearer { token: t })
                }
            }
            AuthKind::Basic => Some(Auth::Basic {
                username: self.auth_basic_user.value().to_string(),
                password: self.auth_basic_pass.value().to_string(),
            }),
            AuthKind::ApiKey => Some(Auth::ApiKey {
                key: self.auth_api_key.value().to_string(),
                value: self.auth_api_value.value().to_string(),
                location: API_KEY_LOCATIONS[self.api_key_location_index].clone(),
            }),
            AuthKind::OAuth2 => {
                let scopes: Vec<String> = self
                    .oauth2_scopes
                    .value()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                let grant = OAUTH2_GRANTS[self.oauth2_grant_index];
                let refresh_raw = self.oauth2_refresh_token.value();
                let refresh_token = if grant == OAuth2Grant::RefreshToken && !refresh_raw.is_empty()
                {
                    Some(refresh_raw.to_string())
                } else {
                    None
                };
                Some(Auth::OAuth2(OAuth2Config {
                    grant,
                    client_id: self.oauth2_client_id.value().to_string(),
                    client_secret: self.oauth2_client_secret.value().to_string(),
                    token_url: self.oauth2_token_url.value().to_string(),
                    scopes,
                    refresh_token,
                    access_token: None,
                }))
            }
        };

        let body = match BODY_KINDS[self.body_kind_index] {
            BodyKind::None => None,
            BodyKind::Raw => {
                let s = self.body_editor.value();
                if s.trim().is_empty() {
                    None
                } else {
                    Some(RequestBody::Raw(s))
                }
            }
            BodyKind::Json => {
                let s = self.body_editor.value();
                if s.trim().is_empty() {
                    None
                } else {
                    match serde_json::from_str::<serde_json::Value>(&s) {
                        Ok(v) => Some(RequestBody::Json(v)),
                        Err(e) => {
                            let msg = format!("JSON body invalid: {e}");
                            self.validation_error = Some(msg.clone());
                            return Err(msg);
                        }
                    }
                }
            }
            BodyKind::Form => {
                let map: HashMap<String, String> = self
                    .body_form_rows
                    .iter()
                    .filter_map(|r| {
                        let k = r.key.value().to_string();
                        if k.is_empty() {
                            None
                        } else {
                            Some((k, r.value.value().to_string()))
                        }
                    })
                    .collect();
                if map.is_empty() {
                    None
                } else {
                    Some(RequestBody::Form(map))
                }
            }
            BodyKind::Multipart => {
                let fields: Vec<FormField> = self
                    .body_multipart_rows
                    .iter()
                    .filter_map(|r| {
                        let k = r.key.value().to_string();
                        if k.is_empty() {
                            return None;
                        }
                        let v = if r.is_file {
                            FormValue::File(FileRef {
                                path: r.value.value().to_string(),
                                mime_type: None,
                            })
                        } else {
                            FormValue::Text(r.value.value().to_string())
                        };
                        Some(FormField { key: k, value: v })
                    })
                    .collect();
                if fields.is_empty() {
                    None
                } else {
                    Some(RequestBody::Multipart(fields))
                }
            }
        };

        let params: Vec<QueryParam> = self
            .param_rows
            .iter()
            .filter_map(|r| {
                let k = r.key.value().to_string();
                if k.is_empty() {
                    None
                } else {
                    Some(QueryParam {
                        key: k,
                        value: r.value.value().to_string(),
                        enabled: r.enabled,
                    })
                }
            })
            .collect();

        let headers: HashMap<String, String> = self
            .header_rows
            .iter()
            .filter_map(|r| {
                let k = r.key.value().to_string();
                if k.is_empty() {
                    None
                } else {
                    Some((k, r.value.value().to_string()))
                }
            })
            .collect();

        let capture = {
            let s = self.capture_editor.value();
            if s.trim().is_empty() { None } else { Some(s) }
        };

        Ok(Request {
            name: if name.is_empty() {
                "Unnamed".into()
            } else {
                name
            },
            request_type: method,
            url,
            headers: if headers.is_empty() {
                None
            } else {
                Some(headers)
            },
            body,
            auth,
            params: if params.is_empty() {
                None
            } else {
                Some(params)
            },
            capture,
        })
    }

    /// Returns true if the editor consumed the event.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        let Event::Key(key) = event else { return false };
        if key.kind != KeyEventKind::Press {
            return false;
        }

        if self.editing {
            if key.code == KeyCode::Esc {
                self.editing = false;
                self.disable_all_inputs();
            } else {
                self.forward_to_active_input(event);
            }
            return true;
        }

        match key.code {
            // Tab belongs to global pane cycling; sub-tabs use [ and ] or h/l.
            KeyCode::Char(']') => self.next_tab(),
            KeyCode::Char('[') => self.prev_tab(),
            KeyCode::Char('j') | KeyCode::Down => self.move_focus_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_focus_up(),
            KeyCode::Char('e') => match self.focused {
                Some(FocusId::MethodSelector) => {
                    self.method_index = (self.method_index + 1) % METHODS.len();
                }
                Some(FocusId::AuthKindSelector) => {
                    self.auth_kind_index = (self.auth_kind_index + 1) % AUTH_KINDS.len();
                }
                Some(FocusId::AuthApiKeyLocation) => {
                    self.api_key_location_index =
                        (self.api_key_location_index + 1) % API_KEY_LOCATIONS.len();
                }
                Some(FocusId::AuthOAuth2Grant) => {
                    self.oauth2_grant_index = (self.oauth2_grant_index + 1) % OAUTH2_GRANTS.len();
                }
                Some(FocusId::BodyKindSelector) => {
                    self.body_kind_index = (self.body_kind_index + 1) % BODY_KINDS.len();
                    self.apply_body_editor_mode();
                }
                Some(FocusId::BodyMultipartKind(i)) => {
                    if let Some(r) = self.body_multipart_rows.get_mut(i) {
                        r.is_file = !r.is_file;
                    }
                }
                Some(_) => {
                    self.editing = true;
                    self.enable_focused_input();
                }
                None => {}
            },
            KeyCode::Char('l') | KeyCode::Right => match self.focused {
                Some(FocusId::MethodSelector) => {
                    self.method_index = (self.method_index + 1) % METHODS.len();
                }
                Some(FocusId::AuthKindSelector) => {
                    self.auth_kind_index = (self.auth_kind_index + 1) % AUTH_KINDS.len();
                }
                Some(FocusId::AuthApiKeyLocation) => {
                    self.api_key_location_index =
                        (self.api_key_location_index + 1) % API_KEY_LOCATIONS.len();
                }
                Some(FocusId::AuthOAuth2Grant) => {
                    self.oauth2_grant_index = (self.oauth2_grant_index + 1) % OAUTH2_GRANTS.len();
                }
                Some(FocusId::BodyKindSelector) => {
                    self.body_kind_index = (self.body_kind_index + 1) % BODY_KINDS.len();
                    self.apply_body_editor_mode();
                }
                _ => self.next_tab(),
            },
            KeyCode::Char('h') | KeyCode::Left => match self.focused {
                Some(FocusId::MethodSelector) => {
                    self.method_index = (self.method_index + METHODS.len() - 1) % METHODS.len();
                }
                Some(FocusId::AuthKindSelector) => {
                    self.auth_kind_index =
                        (self.auth_kind_index + AUTH_KINDS.len() - 1) % AUTH_KINDS.len();
                }
                Some(FocusId::AuthApiKeyLocation) => {
                    self.api_key_location_index =
                        (self.api_key_location_index + API_KEY_LOCATIONS.len() - 1)
                            % API_KEY_LOCATIONS.len();
                }
                Some(FocusId::AuthOAuth2Grant) => {
                    self.oauth2_grant_index =
                        (self.oauth2_grant_index + OAUTH2_GRANTS.len() - 1) % OAUTH2_GRANTS.len();
                }
                Some(FocusId::BodyKindSelector) => {
                    self.body_kind_index =
                        (self.body_kind_index + BODY_KINDS.len() - 1) % BODY_KINDS.len();
                    self.apply_body_editor_mode();
                }
                _ => self.prev_tab(),
            },
            KeyCode::Char('t') => match self.focused {
                Some(FocusId::ParamEnabled(i))
                | Some(FocusId::ParamKey(i))
                | Some(FocusId::ParamValue(i)) => {
                    if let Some(r) = self.param_rows.get_mut(i) {
                        r.enabled = !r.enabled;
                    }
                }
                Some(FocusId::BodyMultipartKind(i)) => {
                    if let Some(r) = self.body_multipart_rows.get_mut(i) {
                        r.is_file = !r.is_file;
                    }
                }
                _ => {}
            },
            KeyCode::Char('d') => match self.focused {
                Some(FocusId::ParamEnabled(i) | FocusId::ParamKey(i) | FocusId::ParamValue(i))
                    if i < self.param_rows.len() && self.param_rows.len() > 1 =>
                {
                    self.param_rows.remove(i);
                    self.focused = Some(FocusId::ParamEnabled(i.min(self.param_rows.len() - 1)));
                }
                Some(FocusId::HeaderKey(i) | FocusId::HeaderValue(i))
                    if i < self.header_rows.len() && self.header_rows.len() > 1 =>
                {
                    self.header_rows.remove(i);
                    self.focused = Some(FocusId::HeaderKey(i.min(self.header_rows.len() - 1)));
                }
                Some(FocusId::BodyFormRow(i, _))
                    if i < self.body_form_rows.len() && self.body_form_rows.len() > 1 =>
                {
                    self.body_form_rows.remove(i);
                    self.focused = Some(FocusId::BodyFormRow(
                        i.min(self.body_form_rows.len() - 1),
                        false,
                    ));
                }
                Some(
                    FocusId::BodyMultipartKind(i)
                    | FocusId::BodyMultipartKey(i)
                    | FocusId::BodyMultipartValue(i),
                ) if i < self.body_multipart_rows.len() && self.body_multipart_rows.len() > 1 => {
                    self.body_multipart_rows.remove(i);
                    self.focused = Some(FocusId::BodyMultipartKind(
                        i.min(self.body_multipart_rows.len() - 1),
                    ));
                }
                _ => {}
            },
            KeyCode::Char('a') => match Tab::ALL[self.current_tab] {
                Tab::Params => {
                    let i = self.param_rows.len();
                    self.param_rows.push(ParamRow::new(
                        &format!("Key {}", i + 1),
                        &format!("Value {}", i + 1),
                    ));
                    self.focused = Some(FocusId::ParamKey(i));
                }
                Tab::Headers => {
                    let i = self.header_rows.len();
                    self.header_rows.push(ParamRow::new(
                        &format!("Header {}", i + 1),
                        &format!("Value {}", i + 1),
                    ));
                    self.focused = Some(FocusId::HeaderKey(i));
                }
                Tab::Body => match BODY_KINDS[self.body_kind_index] {
                    BodyKind::Form => {
                        let i = self.body_form_rows.len();
                        self.body_form_rows.push(ParamRow::new(
                            &format!("Key {}", i + 1),
                            &format!("Value {}", i + 1),
                        ));
                        self.focused = Some(FocusId::BodyFormRow(i, false));
                    }
                    BodyKind::Multipart => {
                        let i = self.body_multipart_rows.len();
                        self.body_multipart_rows.push(MultipartRow::new(
                            &format!("Key {}", i + 1),
                            &format!("Value {}", i + 1),
                        ));
                        self.focused = Some(FocusId::BodyMultipartKind(i));
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => return false,
        }
        true
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, has_focus: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // tabs
                Constraint::Length(1), // divider
                Constraint::Min(1),    // body
                Constraint::Length(1), // hint
            ])
            .split(area);

        let tabs_widget = TabsWidget::new(Tab::titles())
            .style(Style::default().fg(if has_focus {
                Color::White
            } else {
                Color::DarkGray
            }))
            .highlight_style(Style::default().yellow().on_black().bold())
            .select(self.current_tab)
            .divider(symbols::DOT)
            .padding("", "");
        frame.render_widget(tabs_widget, chunks[0]);

        frame.render_widget(
            Paragraph::new("─".repeat(chunks[1].width as usize))
                .style(Style::default().fg(Color::DarkGray)),
            chunks[1],
        );

        match Tab::ALL[self.current_tab] {
            Tab::Info => self.render_info_tab(frame, chunks[2]),
            Tab::Auth => self.render_auth_tab(frame, chunks[2]),
            Tab::Body => self.render_body_tab(frame, chunks[2]),
            Tab::Params => self.render_params_tab(frame, chunks[2]),
            Tab::Headers => self.render_headers_tab(frame, chunks[2]),
            Tab::Capture => self.render_capture_tab(frame, chunks[2]),
        }

        let hint = if let Some(err) = &self.validation_error {
            err.clone()
        } else if self.editing {
            "Esc: stop editing".to_string()
        } else if !has_focus {
            "press 2 to focus editor".to_string()
        } else {
            match Tab::ALL[self.current_tab] {
                Tab::Params => {
                    "j/k navigate  e edit  t toggle  a add  d delete  [/] tab  w save".to_string()
                }
                Tab::Headers | Tab::Body => {
                    "j/k navigate  e edit  a add  d delete  [/] tab  w save".to_string()
                }
                _ => "j/k navigate  e edit  [/] tab  w save".to_string(),
            }
        };
        let hint_style = if self.validation_error.is_some() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(hint)
                .style(hint_style)
                .alignment(Alignment::Center),
            chunks[3],
        );
    }

    fn render_info_tab(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);
        self.render_method_selector(frame, chunks[0]);
        Self::render_input(
            frame,
            chunks[1],
            &self.name_input,
            self.focused == Some(FocusId::NameInput),
            self.editing,
        );
        Self::render_input(
            frame,
            chunks[2],
            &self.url_input,
            self.focused == Some(FocusId::UrlInput),
            self.editing,
        );
    }

    fn render_auth_tab(&self, frame: &mut Frame, area: Rect) {
        let mut constraints = vec![Constraint::Length(3)]; // kind selector
        match AUTH_KINDS[self.auth_kind_index] {
            AuthKind::None => {}
            AuthKind::Bearer => constraints.push(Constraint::Length(3)),
            AuthKind::Basic => {
                constraints.push(Constraint::Length(3));
                constraints.push(Constraint::Length(3));
            }
            AuthKind::ApiKey => {
                constraints.push(Constraint::Length(3));
                constraints.push(Constraint::Length(3));
                constraints.push(Constraint::Length(3));
            }
            AuthKind::OAuth2 => {
                // grant + 4 always-present fields + optional refresh token row
                constraints.push(Constraint::Length(3)); // grant
                constraints.push(Constraint::Length(3)); // client_id
                constraints.push(Constraint::Length(3)); // client_secret
                constraints.push(Constraint::Length(3)); // token_url
                constraints.push(Constraint::Length(3)); // scopes
                if OAUTH2_GRANTS[self.oauth2_grant_index] == OAuth2Grant::RefreshToken {
                    constraints.push(Constraint::Length(3));
                }
            }
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        self.render_kind_selector(
            frame,
            chunks[0],
            " Auth Type (e/h/l) ",
            AUTH_KINDS.iter().map(|k| k.label()).collect(),
            self.auth_kind_index,
            self.focused == Some(FocusId::AuthKindSelector),
        );

        match AUTH_KINDS[self.auth_kind_index] {
            AuthKind::None => {}
            AuthKind::Bearer => {
                Self::render_input(
                    frame,
                    chunks[1],
                    &self.auth_bearer_token,
                    self.focused == Some(FocusId::AuthBearerToken),
                    self.editing,
                );
            }
            AuthKind::Basic => {
                Self::render_input(
                    frame,
                    chunks[1],
                    &self.auth_basic_user,
                    self.focused == Some(FocusId::AuthBasicUser),
                    self.editing,
                );
                Self::render_input(
                    frame,
                    chunks[2],
                    &self.auth_basic_pass,
                    self.focused == Some(FocusId::AuthBasicPass),
                    self.editing,
                );
            }
            AuthKind::ApiKey => {
                Self::render_input(
                    frame,
                    chunks[1],
                    &self.auth_api_key,
                    self.focused == Some(FocusId::AuthApiKey),
                    self.editing,
                );
                Self::render_input(
                    frame,
                    chunks[2],
                    &self.auth_api_value,
                    self.focused == Some(FocusId::AuthApiValue),
                    self.editing,
                );
                self.render_kind_selector(
                    frame,
                    chunks[3],
                    " Location (e/h/l) ",
                    API_KEY_LOCATIONS
                        .iter()
                        .map(api_key_location_label)
                        .collect(),
                    self.api_key_location_index,
                    self.focused == Some(FocusId::AuthApiKeyLocation),
                );
            }
            AuthKind::OAuth2 => {
                self.render_kind_selector(
                    frame,
                    chunks[1],
                    " Grant Type (e/h/l) ",
                    OAUTH2_GRANTS
                        .iter()
                        .map(oauth2_grant_label)
                        .collect(),
                    self.oauth2_grant_index,
                    self.focused == Some(FocusId::AuthOAuth2Grant),
                );
                Self::render_input(
                    frame,
                    chunks[2],
                    &self.oauth2_client_id,
                    self.focused == Some(FocusId::AuthOAuth2ClientId),
                    self.editing,
                );
                Self::render_input(
                    frame,
                    chunks[3],
                    &self.oauth2_client_secret,
                    self.focused == Some(FocusId::AuthOAuth2ClientSecret),
                    self.editing,
                );
                Self::render_input(
                    frame,
                    chunks[4],
                    &self.oauth2_token_url,
                    self.focused == Some(FocusId::AuthOAuth2TokenUrl),
                    self.editing,
                );
                Self::render_input(
                    frame,
                    chunks[5],
                    &self.oauth2_scopes,
                    self.focused == Some(FocusId::AuthOAuth2Scopes),
                    self.editing,
                );
                if OAUTH2_GRANTS[self.oauth2_grant_index] == OAuth2Grant::RefreshToken {
                    Self::render_input(
                        frame,
                        chunks[6],
                        &self.oauth2_refresh_token,
                        self.focused == Some(FocusId::AuthOAuth2RefreshToken),
                        self.editing,
                    );
                }
            }
        }
    }

    fn render_body_tab(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        self.render_kind_selector(
            frame,
            chunks[0],
            " Body Type (e/h/l) ",
            BODY_KINDS.iter().map(|k| k.label()).collect(),
            self.body_kind_index,
            self.focused == Some(FocusId::BodyKindSelector),
        );
        match BODY_KINDS[self.body_kind_index] {
            BodyKind::None => {
                frame.render_widget(
                    Paragraph::new("(no body)").style(Style::default().fg(Color::DarkGray)),
                    chunks[1],
                );
            }
            BodyKind::Raw => {
                let focused = self.focused == Some(FocusId::BodyText);
                self.body_editor
                    .render(frame, chunks[1], focused, " Body (raw) ");
            }
            BodyKind::Json => {
                let focused = self.focused == Some(FocusId::BodyText);
                self.body_editor
                    .render(frame, chunks[1], focused, " Body (JSON) ");
            }
            BodyKind::Form => Self::render_kv_rows(
                frame,
                chunks[1],
                &self.body_form_rows,
                |i| {
                    (
                        Some(FocusId::BodyFormRow(i, false)),
                        Some(FocusId::BodyFormRow(i, true)),
                    )
                },
                self.focused,
                self.editing,
                false,
            ),
            BodyKind::Multipart => self.render_multipart_rows(frame, chunks[1]),
        }
    }

    fn render_params_tab(&self, frame: &mut Frame, area: Rect) {
        let list_area = area;
        let mut y = 0u16;
        for (i, row) in self.param_rows.iter().enumerate() {
            if y + 3 > list_area.height {
                break;
            }
            let enabled_w = 7u16;
            let half = (list_area.width.saturating_sub(enabled_w)) / 2;
            let enabled_rect = Rect {
                x: list_area.x,
                y: list_area.y + y,
                width: enabled_w,
                height: 3,
            };
            let key_rect = Rect {
                x: list_area.x + enabled_w,
                y: list_area.y + y,
                width: half,
                height: 3,
            };
            let val_rect = Rect {
                x: list_area.x + enabled_w + half,
                y: list_area.y + y,
                width: list_area.width.saturating_sub(enabled_w + half),
                height: 3,
            };

            let mark = if row.enabled { "[x]" } else { "[ ]" };
            let mark_focused = self.focused == Some(FocusId::ParamEnabled(i));
            let mark_style = if mark_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(if mark_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                })
                .title(" en ");
            frame.render_widget(
                Paragraph::new(mark)
                    .style(mark_style)
                    .alignment(Alignment::Center)
                    .block(block),
                enabled_rect,
            );
            Self::render_input(
                frame,
                key_rect,
                &row.key,
                self.focused == Some(FocusId::ParamKey(i)),
                self.editing,
            );
            Self::render_input(
                frame,
                val_rect,
                &row.value,
                self.focused == Some(FocusId::ParamValue(i)),
                self.editing,
            );
            y += 3;
        }
    }

    fn render_capture_tab(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(area);
        frame.render_widget(
            Paragraph::new(
                "JSON template using %name% placeholders. After each response, matched \
                 values are captured into the active env. Example: {\"id\": \"%user_id%\"}",
            )
            .style(Style::default().fg(Color::DarkGray))
            .wrap(ratatui::widgets::Wrap { trim: false }),
            chunks[0],
        );
        let focused = self.focused == Some(FocusId::CaptureText);
        self.capture_editor
            .render(frame, chunks[1], focused, " Capture template ");
    }

    fn render_headers_tab(&self, frame: &mut Frame, area: Rect) {
        Self::render_kv_rows(
            frame,
            area,
            &self.header_rows,
            |i| (Some(FocusId::HeaderKey(i)), Some(FocusId::HeaderValue(i))),
            self.focused,
            self.editing,
            true,
        );
    }

    fn render_multipart_rows(&self, frame: &mut Frame, area: Rect) {
        let mut y = 0u16;
        for (i, row) in self.body_multipart_rows.iter().enumerate() {
            if y + 3 > area.height {
                break;
            }
            let kind_w = 9u16;
            let half = (area.width.saturating_sub(kind_w)) / 2;
            let kind_rect = Rect {
                x: area.x,
                y: area.y + y,
                width: kind_w,
                height: 3,
            };
            let key_rect = Rect {
                x: area.x + kind_w,
                y: area.y + y,
                width: half,
                height: 3,
            };
            let val_rect = Rect {
                x: area.x + kind_w + half,
                y: area.y + y,
                width: area.width.saturating_sub(kind_w + half),
                height: 3,
            };

            let kind_label = if row.is_file { "file" } else { "text" };
            let kind_focused = self.focused == Some(FocusId::BodyMultipartKind(i));
            let kind_style = if kind_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(if kind_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                })
                .title(" t ");
            frame.render_widget(
                Paragraph::new(kind_label)
                    .style(kind_style)
                    .alignment(Alignment::Center)
                    .block(block),
                kind_rect,
            );
            Self::render_input(
                frame,
                key_rect,
                &row.key,
                self.focused == Some(FocusId::BodyMultipartKey(i)),
                self.editing,
            );
            Self::render_input(
                frame,
                val_rect,
                &row.value,
                self.focused == Some(FocusId::BodyMultipartValue(i)),
                self.editing,
            );
            y += 3;
        }
    }

    fn render_kv_rows<F>(
        frame: &mut Frame,
        area: Rect,
        rows: &[ParamRow],
        focus_for: F,
        focused: Option<FocusId>,
        editing: bool,
        _is_header: bool,
    ) where
        F: Fn(usize) -> (Option<FocusId>, Option<FocusId>),
    {
        let mut y = 0u16;
        for (i, row) in rows.iter().enumerate() {
            if y + 3 > area.height {
                break;
            }
            let half = area.width / 2;
            let key_rect = Rect {
                x: area.x,
                y: area.y + y,
                width: half,
                height: 3,
            };
            let val_rect = Rect {
                x: area.x + half,
                y: area.y + y,
                width: area.width - half,
                height: 3,
            };
            let (k_focus, v_focus) = focus_for(i);
            Self::render_input(
                frame,
                key_rect,
                &row.key,
                k_focus == focused && focused.is_some(),
                editing,
            );
            Self::render_input(
                frame,
                val_rect,
                &row.value,
                v_focus == focused && focused.is_some(),
                editing,
            );
            y += 3;
        }
    }

    fn render_method_selector(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focused == Some(FocusId::MethodSelector);
        let selected = &METHODS[self.method_index];
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Method (e/h/l) ")
            .border_style(border_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let spans: Vec<Span> = METHODS
            .iter()
            .enumerate()
            .flat_map(|(i, m)| {
                let style = if m == selected {
                    Style::default().fg(m.color()).bold().underlined()
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let mut v: Vec<Span> = vec![Span::styled(m.as_str(), style)];
                if i + 1 < METHODS.len() {
                    v.push(Span::raw("  "));
                }
                v
            })
            .collect();
        frame.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    fn render_kind_selector(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        labels: Vec<&str>,
        selected_index: usize,
        is_focused: bool,
    ) {
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title.to_string())
            .border_style(border_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let spans: Vec<Span> = labels
            .iter()
            .enumerate()
            .flat_map(|(i, label)| {
                let style = if i == selected_index {
                    Style::default().fg(Color::White).bold().underlined()
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let mut v: Vec<Span> = vec![Span::styled(label.to_string(), style)];
                if i + 1 < labels.len() {
                    v.push(Span::raw("  "));
                }
                v
            })
            .collect();
        frame.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    fn render_input(
        frame: &mut Frame,
        area: Rect,
        input: &TextInput,
        is_focused: bool,
        editing: bool,
    ) {
        input.render(frame, area);
        let style = if is_focused && editing {
            Style::default().fg(Color::Yellow)
        } else if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            return;
        };
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(style)
                .title(input.title.clone()),
            area,
        );
    }
}
