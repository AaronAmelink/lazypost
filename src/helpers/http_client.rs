use std::collections::HashMap;
use std::time::Instant;

use once_cell::sync::Lazy;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use regex::Regex;
use reqwest::header::{
    AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue,
};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};

use crate::helpers::env::substitute_required;
use crate::helpers::items::{
    ApiKeyLocation, Auth, FormValue, QueryParam, Request, RequestBody, RequestType,
};
use crate::helpers::oauth;

/// Cookie store is enabled, so Set-Cookie from a response is replayed on the
/// next request to the same domain. The store is in-memory and lives for the
/// process lifetime — "session-only".
static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent("lazypost/0.1")
        .cookie_store(true)
        .build()
        .expect("failed to build reqwest client")
});

static URL_VAR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<\s*([A-Za-z0-9_.\-]+)\s*>").expect("url var regex")
});

fn is_valid_url_var_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn substitute_url_vars(url: &str, vars: &HashMap<String, String>) -> Result<String, String> {
    use std::collections::BTreeSet;

    let mut missing: BTreeSet<String> = BTreeSet::new();
    let mut empty: BTreeSet<String> = BTreeSet::new();
    let mut out = String::with_capacity(url.len());
    let mut last = 0usize;

    for caps in URL_VAR_RE.captures_iter(url) {
        let m = caps.get(0).unwrap();
        out.push_str(&url[last..m.start()]);
        let name = caps.get(1).unwrap().as_str();
        match vars.get(name) {
            Some(v) if !v.is_empty() => out.push_str(v),
            Some(_) => {
                empty.insert(name.to_string());
            }
            None => {
                missing.insert(name.to_string());
            }
        }
        last = m.end();
    }
    out.push_str(&url[last..]);

    if missing.is_empty() && empty.is_empty() {
        Ok(out)
    } else {
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!(
                "missing url vars: {}",
                missing.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        if !empty.is_empty() {
            parts.push(format!(
                "empty url vars: {}",
                empty.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        Err(parts.join("; "))
    }
}

fn build_url_var_map(
    rows: &Option<Vec<QueryParam>>,
    vmap: &HashMap<String, String>,
) -> Result<HashMap<String, String>, HttpError> {
    let mut out: HashMap<String, String> = HashMap::new();
    let Some(rows) = rows.as_ref() else {
        return Ok(out);
    };

    for row in rows {
        if !row.enabled {
            continue;
        }
        let raw_key = substitute_required(&row.key, vmap)
            .map_err(|e| HttpError::Build(format!("url var key: {e}")))?;
        let key = raw_key.trim().to_string();
        if key.is_empty() {
            return Err(HttpError::Build("url var key is empty".into()));
        }
        if !is_valid_url_var_name(&key) {
            return Err(HttpError::Build(format!(
                "url var key '{key}' is invalid (allowed: A-Z a-z 0-9 _ . -)"
            )));
        }
        let raw_value = substitute_required(&row.value, vmap)
            .map_err(|e| HttpError::Build(format!("url var {key}: {e}")))?;
        if raw_value.is_empty() {
            return Err(HttpError::Build(format!("url var '{key}' is empty")));
        }
        let encoded = utf8_percent_encode(&raw_value, NON_ALPHANUMERIC).to_string();
        out.insert(key, encoded);
    }

    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpError {
    Build(String),
    Send(String),
    Body(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Build(s) => write!(f, "build error: {s}"),
            HttpError::Send(s) => write!(f, "send error: {s}"),
            HttpError::Body(s) => write!(f, "body error: {s}"),
        }
    }
}

/// reqwest's Display for its error is often a terse "builder error" / "request
/// error" with no detail — walk the source chain to surface the real cause.
fn stringify_error(err: &(dyn std::error::Error + 'static)) -> String {
    let mut parts: Vec<String> = vec![err.to_string()];
    let mut source = err.source();
    while let Some(cause) = source {
        let s = cause.to_string();
        if !parts.last().map(|p| p == &s).unwrap_or(false) {
            parts.push(s);
        }
        source = cause.source();
    }
    parts.join(": ")
}

fn method_of(rt: &RequestType) -> Method {
    match rt {
        RequestType::Get => Method::GET,
        RequestType::Post => Method::POST,
        RequestType::Put => Method::PUT,
        RequestType::Patch => Method::PATCH,
        RequestType::Delete => Method::DELETE,
    }
}

pub async fn execute(
    req: Request,
    vars: HashMap<String, String>,
) -> Result<ExecutedResponse, HttpError> {
    let vmap = &vars;
    let url = substitute_required(&req.url, vmap)
        .map_err(|e| HttpError::Build(format!("url: {e}")))?;
    let url_vars = build_url_var_map(&req.url_vars, vmap)?;
    let url = substitute_url_vars(&url, &url_vars)
        .map_err(|e| HttpError::Build(format!("url vars: {e}")))?;
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(HttpError::Build("URL is empty".into()));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(HttpError::Build(format!(
            "URL must start with http:// or https:// (got '{trimmed}')"
        )));
    }

    let request_type = req.request_type.clone();
    let mut builder = CLIENT.request(method_of(&request_type), trimmed);

    // Headers
    let mut header_map = HeaderMap::new();
    if let Some(headers) = &req.headers {
        for (k, v) in headers {
            let raw_k = k;
            let k = substitute_required(raw_k, vmap)
                .map_err(|e| HttpError::Build(format!("header name '{raw_k}': {e}")))?;
            let v = substitute_required(v, vmap)
                .map_err(|e| HttpError::Build(format!("header value for '{k}': {e}")))?;
            let name = HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| HttpError::Build(format!("bad header name {k}: {e}")))?;
            let value = HeaderValue::from_str(&v)
                .map_err(|e| HttpError::Build(format!("bad header value: {e}")))?;
            header_map.insert(name, value);
        }
    }

    let has_content_length = header_map.contains_key(CONTENT_LENGTH);

    // Query params (skip disabled)
    let mut query: Vec<(String, String)> = Vec::new();
    if let Some(params) = &req.params {
        for p in params {
            if !p.enabled {
                continue;
            }
            let raw = p.key.as_str();
            let k = substitute_required(&p.key, vmap)
                .map_err(|e| HttpError::Build(format!("query param key '{raw}': {e}")))?;
            let v = substitute_required(&p.value, vmap)
                .map_err(|e| HttpError::Build(format!("query param '{k}': {e}")))?;
            query.push((k, v));
        }
    }

    // Auth
    match &req.auth {
        None | Some(Auth::None) => {}
        Some(Auth::Bearer { token }) => {
            let token = substitute_required(token, vmap)
                .map_err(|e| HttpError::Build(format!("bearer token: {e}")))?;
            let val = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| HttpError::Build(format!("bad bearer token: {e}")))?;
            header_map.insert(AUTHORIZATION, val);
        }
        Some(Auth::Basic { username, password }) => {
            let user = substitute_required(username, vmap)
                .map_err(|e| HttpError::Build(format!("basic auth username: {e}")))?;
            let pass = substitute_required(password, vmap)
                .map_err(|e| HttpError::Build(format!("basic auth password: {e}")))?;
            builder = builder.basic_auth(user, Some(pass));
        }
        Some(Auth::ApiKey {
            key,
            value,
            location,
        }) => {
            let k = substitute_required(key, vmap)
                .map_err(|e| HttpError::Build(format!("api key name: {e}")))?;
            let v = substitute_required(value, vmap)
                .map_err(|e| HttpError::Build(format!("api key value for '{k}': {e}")))?;
            match location {
                ApiKeyLocation::Header => {
                    let name = HeaderName::from_bytes(k.as_bytes())
                        .map_err(|e| HttpError::Build(format!("bad api key header: {e}")))?;
                    let val = HeaderValue::from_str(&v)
                        .map_err(|e| HttpError::Build(format!("bad api key value: {e}")))?;
                    header_map.insert(name, val);
                }
                ApiKeyLocation::Query => query.push((k, v)),
            }
        }
        Some(Auth::OAuth2(cfg)) => {
            let token = oauth::fetch_token(&CLIENT, cfg, vmap)
                .await
                .map_err(|e| HttpError::Build(format!("oauth2: {e}")))?;
            let val = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| HttpError::Build(format!("bad oauth2 token: {e}")))?;
            header_map.insert(AUTHORIZATION, val);
        }
    }

    if !query.is_empty() {
        builder = builder.query(&query);
    }
    builder = builder.headers(header_map);

    // Body
    let mut body_is_empty = true;
    if let Some(body) = &req.body {
        match body {
            RequestBody::None => {}
            RequestBody::Raw(s) => {
                body_is_empty = false;
                let s = substitute_required(s, vmap)
                    .map_err(|e| HttpError::Build(format!("raw body: {e}")))?;
                builder = builder.body(s);
            }
            RequestBody::Json(v) => {
                body_is_empty = false;
                let s = serde_json::to_string(v)
                    .map_err(|e| HttpError::Build(format!("json serialize: {e}")))?;
                let s = substitute_required(&s, vmap)
                    .map_err(|e| HttpError::Build(format!("json body: {e}")))?;
                // Re-parse so reqwest sends correct content-type
                let v: serde_json::Value = serde_json::from_str(&s)
                    .map_err(|e| HttpError::Build(format!("json reparse: {e}")))?;
                builder = builder.json(&v);
            }
            RequestBody::Form(map) => {
                body_is_empty = false;
                let mut substituted: Vec<(String, String)> = Vec::new();
                for (k, v) in map {
                    let raw_k = k;
                    let k = substitute_required(raw_k, vmap)
                        .map_err(|e| HttpError::Build(format!("form key '{raw_k}': {e}")))?;
                    let v = substitute_required(v, vmap)
                        .map_err(|e| HttpError::Build(format!("form value for '{k}': {e}")))?;
                    substituted.push((k, v));
                }
                builder = builder.form(&substituted);
            }
            RequestBody::Multipart(fields) => {
                body_is_empty = false;
                let mut form = reqwest::multipart::Form::new();
                for f in fields {
                    let raw_key = &f.key;
                    let key = substitute_required(raw_key, vmap)
                        .map_err(|e| HttpError::Build(format!("multipart key '{raw_key}': {e}")))?;
                    match &f.value {
                        FormValue::Text(t) => {
                            let v = substitute_required(t, vmap)
                                .map_err(|e| HttpError::Build(format!("multipart value for '{key}': {e}")))?;
                            form = form.text(key, v);
                        }
                        FormValue::File(file_ref) => {
                            let path = substitute_required(&file_ref.path, vmap)
                                .map_err(|e| HttpError::Build(format!("file path for '{key}': {e}")))?;
                            if path.is_empty() {
                                return Err(HttpError::Build(format!(
                                    "file path for '{key}' is empty"
                                )));
                            }
                            let bytes = tokio::fs::read(&path)
                                .await
                                .map_err(|e| HttpError::Build(format!("read file {path}: {e}")))?;
                            let mut part =
                                reqwest::multipart::Part::bytes(bytes).file_name(path.clone());
                            if let Some(mime) = &file_ref.mime_type {
                                let mime = substitute_required(mime, vmap)
                                    .map_err(|e| HttpError::Build(format!("mime for '{key}': {e}")))?;
                                part = part
                                    .mime_str(&mime)
                                    .map_err(|e| HttpError::Build(format!("bad mime: {e}")))?;
                            }
                            form = form.part(key, part);
                        }
                    }
                }
                builder = builder.multipart(form);
            }
        }
    }

    if body_is_empty
        && !has_content_length
        && matches!(
            request_type,
            RequestType::Post | RequestType::Put | RequestType::Patch | RequestType::Delete
        )
    {
        builder = builder
            .header(CONTENT_LENGTH, HeaderValue::from_static("0"))
            .body(Vec::<u8>::new());
    }

    let start = Instant::now();
    let resp = builder
        .send()
        .await
        .map_err(|e| HttpError::Send(stringify_error(&e)))?;
    let status_code = resp.status();
    let status = status_code.as_u16();
    let status_text = status_code.canonical_reason().unwrap_or("").to_string();
    let headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let content_type = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(CONTENT_TYPE.as_str()))
        .map(|(_, v)| v.clone());
    let body = resp
        .bytes()
        .await
        .map_err(|e| HttpError::Body(stringify_error(&e)))?
        .to_vec();
    let elapsed_ms = start.elapsed().as_millis() as u64;

    Ok(ExecutedResponse {
        status,
        status_text,
        headers,
        body,
        content_type,
        elapsed_ms,
    })
}
