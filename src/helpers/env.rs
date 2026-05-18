use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

/// Matches `{{ var_name }}` placeholders. Names allow `[A-Za-z0-9_.-]`.
/// Whitespace inside the braces is tolerated.
static VAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\{\s*([A-Za-z0-9_.\-]+)\s*\}\}").unwrap());

/// Expands `{{name}}` placeholders in `s` using the provided variable map.
/// Missing variables expand to an empty string, never `panic!()`.
pub fn substitute(s: &str, vars: &HashMap<String, String>) -> String {
    VAR_RE
        .replace_all(s, |c: &regex::Captures| {
            vars.get(&c[1]).cloned().unwrap_or_default()
        })
        .into_owned()
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
}
