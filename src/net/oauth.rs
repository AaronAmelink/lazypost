use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use reqwest::Client;
use serde::Deserialize;

use crate::logic::env::substitute_required;
use crate::model::items::{OAuth2Config, OAuth2Grant};

#[derive(Debug, Clone)]
pub struct OAuth2Error(pub String);

impl std::fmt::Display for OAuth2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Option<Instant>,
}

impl CachedToken {
    fn still_valid(&self) -> bool {
        match self.expires_at {
            // 30s safety margin so a token doesn't expire mid-flight.
            Some(t) => Instant::now() + Duration::from_secs(30) < t,
            None => true,
        }
    }
}

/// Process-lifetime token cache keyed by (token_url, client_id, grant).
/// Matches the session-only cookie jar: nothing on disk, gone on exit.
static CACHE: Lazy<Mutex<HashMap<String, CachedToken>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn cache_key(token_url: &str, client_id: &str, grant: OAuth2Grant) -> String {
    let grant_tag = match grant {
        OAuth2Grant::ClientCredentials => "cc",
        OAuth2Grant::RefreshToken => "rt",
    };
    format!("{grant_tag}|{client_id}|{token_url}")
}

/// Fetches an access token, using the cache when possible. Variables are
/// substituted in every OAuth2 field so users can reference {{secret}} etc.
pub async fn fetch_token(
    client: &Client,
    config: &OAuth2Config,
    vars: &HashMap<String, String>,
) -> Result<String, OAuth2Error> {
    let client_id = substitute_required(&config.client_id, vars)
        .map_err(|e| OAuth2Error(format!("client_id: {e}")))?;
    let client_secret = substitute_required(&config.client_secret, vars)
        .map_err(|e| OAuth2Error(format!("client_secret: {e}")))?;
    let token_url = substitute_required(&config.token_url, vars)
        .map_err(|e| OAuth2Error(format!("token_url: {e}")))?;
    let mut scopes: Vec<String> = Vec::new();
    for s in &config.scopes {
        let s = substitute_required(s, vars)
            .map_err(|e| OAuth2Error(format!("scope: {e}")))?;
        if !s.is_empty() {
            scopes.push(s);
        }
    }
    let scope = scopes.join(" ");

    if token_url.trim().is_empty() {
        return Err(OAuth2Error("OAuth2 token_url is empty".into()));
    }
    if client_id.trim().is_empty() {
        return Err(OAuth2Error("OAuth2 client_id is empty".into()));
    }

    let key = cache_key(&token_url, &client_id, config.grant);
    if let Some(hit) = CACHE.lock().ok().and_then(|m| m.get(&key).cloned())
        && hit.still_valid()
    {
        return Ok(hit.access_token);
    }

    let mut form: Vec<(&str, String)> = match config.grant {
        OAuth2Grant::ClientCredentials => vec![("grant_type", "client_credentials".into())],
        OAuth2Grant::RefreshToken => {
            let rt = config
                .refresh_token
                .as_ref()
                .map(|s| substitute_required(s, vars))
                .transpose()
                .map_err(|e| OAuth2Error(format!("refresh_token: {e}")))?
                .unwrap_or_default();
            if rt.trim().is_empty() {
                return Err(OAuth2Error(
                    "OAuth2 refresh_token is empty for refresh_token grant".into(),
                ));
            }
            vec![
                ("grant_type", "refresh_token".into()),
                ("refresh_token", rt),
            ]
        }
    };
    if !scope.is_empty() {
        form.push(("scope", scope));
    }

    let resp = client
        .post(&token_url)
        .basic_auth(&client_id, Some(&client_secret))
        .form(&form)
        .send()
        .await
        .map_err(|e| OAuth2Error(format!("token request failed: {e}")))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| OAuth2Error(format!("read token response: {e}")))?;
    if !status.is_success() {
        return Err(OAuth2Error(format!("token endpoint {status}: {body}")));
    }
    let parsed: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| OAuth2Error(format!("parse token response: {e} (body: {body})")))?;

    let cached = CachedToken {
        access_token: parsed.access_token.clone(),
        expires_at: parsed
            .expires_in
            .map(|secs| Instant::now() + Duration::from_secs(secs)),
    };
    if let Ok(mut m) = CACHE.lock() {
        m.insert(key, cached);
    }
    Ok(parsed.access_token)
}
