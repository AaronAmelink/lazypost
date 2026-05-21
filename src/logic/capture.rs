use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

/// A whole-string placeholder like `%var_name%`. Names are [A-Za-z0-9_-].
static WHOLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^%([A-Za-z0-9_\-]+)%$").unwrap());

/// Same placeholder syntax but anywhere inside a string. Used to detect
/// templates like `"Bearer %key%"` and to drive the literal/capture split.
static INNER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"%([A-Za-z0-9_\-]+)%").unwrap());

/// Walks `template` and `actual` in parallel. Wherever `template` has a string
/// containing `%name%` placeholders, captures the matching value(s) from
/// `actual` into the returned list of (name, value) pairs.
///
/// - A pure `"%name%"` (whole-string placeholder) captures any JSON value at
///   that position, stringified.
/// - A mixed string like `"Bearer %key%"` only matches when the actual JSON
///   value at that position is a string with the same literal prefix/suffix.
///   Multiple placeholders per string are supported.
///
/// Mismatched structure (object vs. array, missing keys, length differences)
/// is tolerated — captures that can't be resolved are simply skipped.
pub fn extract_captures(template: &Value, actual: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk(template, actual, &mut out);
    out
}

fn walk(template: &Value, actual: &Value, out: &mut Vec<(String, String)>) {
    match (template, actual) {
        (Value::String(s), _) => {
            if let Some(caps) = WHOLE_RE.captures(s) {
                let name = caps.get(1).unwrap().as_str().to_string();
                out.push((name, value_to_env_string(actual)));
            } else if INNER_RE.is_match(s)
                && let Value::String(actual_s) = actual
            {
                capture_partial(s, actual_s, out);
            }
            // Static string (no placeholders), or mixed pattern against a
            // non-string actual: nothing to capture, no recursion.
        }
        (Value::Object(t_map), Value::Object(a_map)) => {
            for (k, t_val) in t_map {
                if let Some(a_val) = a_map.get(k) {
                    walk(t_val, a_val, out);
                }
            }
        }
        (Value::Array(t_arr), Value::Array(a_arr)) => {
            for (t_val, a_val) in t_arr.iter().zip(a_arr.iter()) {
                walk(t_val, a_val, out);
            }
        }
        _ => {
            // Type mismatch (e.g. template number vs actual string). Skip.
        }
    }
}

/// Builds a regex from `template` by escaping literal segments and turning
/// each `%name%` into a non-greedy capture group, anchored to whole string.
fn capture_partial(template: &str, actual: &str, out: &mut Vec<(String, String)>) {
    let mut pattern = String::from("^");
    let mut names = Vec::new();
    let mut last = 0;
    for cap in INNER_RE.captures_iter(template) {
        let m = cap.get(0).unwrap();
        let name = cap.get(1).unwrap().as_str();
        pattern.push_str(&regex::escape(&template[last..m.start()]));
        pattern.push_str("(.*?)");
        names.push(name.to_string());
        last = m.end();
    }
    pattern.push_str(&regex::escape(&template[last..]));
    pattern.push('$');
    let Ok(re) = Regex::new(&pattern) else {
        return;
    };
    if let Some(caps) = re.captures(actual) {
        for (i, name) in names.iter().enumerate() {
            if let Some(m) = caps.get(i + 1) {
                out.push((name.clone(), m.as_str().to_string()));
            }
        }
    }
}

fn value_to_env_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Object(_) | Value::Array(_) => v.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn captures_nested_string() {
        let t = json!({"item": {"a": "%item_key%"}});
        let a = json!({"item": {"a": "abc123"}});
        let out = extract_captures(&t, &a);
        assert_eq!(out, vec![("item_key".to_string(), "abc123".to_string())]);
    }

    #[test]
    fn captures_number_as_string() {
        let t = json!({"count": "%n%"});
        let a = json!({"count": 42});
        let out = extract_captures(&t, &a);
        assert_eq!(out, vec![("n".to_string(), "42".to_string())]);
    }

    #[test]
    fn array_index_match() {
        let t = json!([{"id": "%first_id%"}, {"id": "%second_id%"}]);
        let a = json!([{"id": "A"}, {"id": "B"}]);
        let out = extract_captures(&t, &a);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn ignores_static_mismatch() {
        let t = json!({"k": "static"});
        let a = json!({"k": "different"});
        assert!(extract_captures(&t, &a).is_empty());
    }

    #[test]
    fn bearer_prefix_captures_token() {
        let t = json!({"auth": "Bearer %key%"});
        let a = json!({"auth": "Bearer abc123"});
        let out = extract_captures(&t, &a);
        assert_eq!(out, vec![("key".to_string(), "abc123".to_string())]);
    }

    #[test]
    fn multiple_placeholders_in_one_string() {
        let t = json!({"creds": "%user%:%pass%"});
        let a = json!({"creds": "alice:hunter2"});
        let out = extract_captures(&t, &a);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], ("user".to_string(), "alice".to_string()));
        assert_eq!(out[1], ("pass".to_string(), "hunter2".to_string()));
    }

    #[test]
    fn partial_template_against_non_string_is_skipped() {
        let t = json!({"x": "id-%n%"});
        let a = json!({"x": 42});
        assert!(extract_captures(&t, &a).is_empty());
    }

    #[test]
    fn partial_template_literal_mismatch_skips() {
        let t = json!({"auth": "Bearer %key%"});
        let a = json!({"auth": "Basic xyz"});
        assert!(extract_captures(&t, &a).is_empty());
    }
}
