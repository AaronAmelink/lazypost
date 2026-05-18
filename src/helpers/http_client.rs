use std::collections::HashMap;
use std::time::Instant;

use once_cell::sync::Lazy;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};

use crate::helpers::env::substitute;
use crate::helpers::items::{ApiKeyLocation, Auth, FormValue, Request, RequestBody, RequestType};
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
    let url = substitute(&req.url, vmap);
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(HttpError::Build("URL is empty".into()));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(HttpError::Build(format!(
            "URL must start with http:// or https:// (got '{trimmed}')"
        )));
    }

    let mut builder = CLIENT.request(method_of(&req.request_type), trimmed);

    // Headers
    let mut header_map = HeaderMap::new();
    if let Some(headers) = &req.headers {
        for (k, v) in headers {
            let k = substitute(k, vmap);
            let v = substitute(v, vmap);
            let name = HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| HttpError::Build(format!("bad header name {k}: {e}")))?;
            let value = HeaderValue::from_str(&v)
                .map_err(|e| HttpError::Build(format!("bad header value: {e}")))?;
            header_map.insert(name, value);
        }
    }

    // Query params (skip disabled)
    let mut query: Vec<(String, String)> = Vec::new();
    if let Some(params) = &req.params {
        for p in params {
            if !p.enabled {
                continue;
            }
            query.push((substitute(&p.key, vmap), substitute(&p.value, vmap)));
        }
    }

    // Auth
    match &req.auth {
        None | Some(Auth::None) => {}
        Some(Auth::Bearer { token }) => {
            let token = substitute(token, vmap);
            let val = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| HttpError::Build(format!("bad bearer token: {e}")))?;
            header_map.insert(AUTHORIZATION, val);
        }
        Some(Auth::Basic { username, password }) => {
            builder =
                builder.basic_auth(substitute(username, vmap), Some(substitute(password, vmap)));
        }
        Some(Auth::ApiKey {
            key,
            value,
            location,
        }) => {
            let k = substitute(key, vmap);
            let v = substitute(value, vmap);
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
    if let Some(body) = &req.body {
        match body {
            RequestBody::None => {}
            RequestBody::Raw(s) => {
                let s = substitute(s, vmap);
                builder = builder.body(s);
            }
            RequestBody::Json(v) => {
                let s = serde_json::to_string(v)
                    .map_err(|e| HttpError::Build(format!("json serialize: {e}")))?;
                let s = substitute(&s, vmap);
                // Re-parse so reqwest sends correct content-type
                let v: serde_json::Value = serde_json::from_str(&s)
                    .map_err(|e| HttpError::Build(format!("json reparse: {e}")))?;
                builder = builder.json(&v);
            }
            RequestBody::Form(map) => {
                let substituted: Vec<(String, String)> = map
                    .iter()
                    .map(|(k, v)| (substitute(k, vmap), substitute(v, vmap)))
                    .collect();
                builder = builder.form(&substituted);
            }
            RequestBody::Multipart(fields) => {
                let mut form = reqwest::multipart::Form::new();
                for f in fields {
                    let key = substitute(&f.key, vmap);
                    match &f.value {
                        FormValue::Text(t) => {
                            form = form.text(key, substitute(t, vmap));
                        }
                        FormValue::File(file_ref) => {
                            let path = substitute(&file_ref.path, vmap);
                            let bytes = tokio::fs::read(&path)
                                .await
                                .map_err(|e| HttpError::Build(format!("read file {path}: {e}")))?;
                            let mut part =
                                reqwest::multipart::Part::bytes(bytes).file_name(path.clone());
                            if let Some(mime) = &file_ref.mime_type {
                                part = part
                                    .mime_str(mime)
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
