use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

/// Matches `{{ var_name }}` placeholders. Names allow `[A-Za-z0-9_.-]`.
/// Whitespace inside the braces is tolerated.
static VAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\{\s*([A-Za-z0-9_.\-]+)\s*\}\}").unwrap());

/// Expands `{{name}}` placeholders, but returns an error if any referenced
/// variable is missing or empty.
pub fn substitute_required(s: &str, vars: &HashMap<String, String>) -> Result<String, String> {
    use std::collections::BTreeSet;

    let mut missing: BTreeSet<String> = BTreeSet::new();
    let mut empty: BTreeSet<String> = BTreeSet::new();

    let mut out = String::with_capacity(s.len());
    let mut last = 0usize;
    for caps in VAR_RE.captures_iter(s) {
        let m = caps.get(0).unwrap();
        out.push_str(&s[last..m.start()]);
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
    out.push_str(&s[last..]);

    if missing.is_empty() && empty.is_empty() {
        Ok(out)
    } else {
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("missing env vars: {}", missing.into_iter().collect::<Vec<_>>().join(", ")));
        }
        if !empty.is_empty() {
            parts.push(format!("empty env vars: {}", empty.into_iter().collect::<Vec<_>>().join(", ")));
        }
        Err(parts.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn empty_env_missing_var_becomes_empty() {
        let empty = HashMap::new();
        assert_eq!(substitute("{{x}}", &empty), "");
    }

    #[test]
    fn substitutes_known_variable() {
        let v = vars(&[("host", "api.example.com")]);
        assert_eq!(
            substitute("https://{{host}}/v1", &v),
            "https://api.example.com/v1",
        );
    }

    #[test]
    fn missing_variable_becomes_empty() {
        let v = vars(&[("a", "1")]);
        assert_eq!(substitute("{{a}}-{{b}}", &v), "1-");
    }

    #[test]
    fn allows_internal_whitespace() {
        let v = vars(&[("x", "ok")]);
        assert_eq!(substitute("[{{  x  }}]", &v), "[ok]");
    }

    #[test]
    fn ignores_malformed_placeholders() {
        let v = vars(&[("x", "ok")]);
        assert_eq!(substitute("{x} {{x", &v), "{x} {{x");
    }

    #[test]
    fn required_missing_var_errors() {
        let empty = HashMap::new();
        assert!(substitute_required("{{x}}", &empty).is_err());
    }

    #[test]
    fn required_empty_var_errors() {
        let v = vars(&[("x", "")]);
        assert!(substitute_required("{{x}}", &v).is_err());
    }

    #[test]
    fn required_substitutes_known_variable() {
        let v = vars(&[("x", "1"), ("y", "2")]);
        assert_eq!(substitute_required("a{{x}}b{{y}}", &v).unwrap(), "a1b2");
    }
}
