//! Reading request bodies and query strings the way the Python API (`FastAPI` and pydantic) did,
//! with the same 400 messages: `missing report_id`, `invalid amount_lost_inr`,
//! `body is not valid JSON`. Messages name fields only, never the values sent.

use crate::errors::ApiError;
use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use serde_json::{Map, Value};

/// The JSON object of a request body.
///
/// Like `FastAPI`: a body with no content type is read as JSON; with a content type, only
/// `application/json` or `application/*+json` is; anything else cannot match the fields.
///
/// # Errors
/// An empty body, invalid JSON, or anything but a JSON object.
pub fn json_object(headers: &HeaderMap, body: &[u8]) -> Result<Map<String, Value>, ApiError> {
    if body.is_empty() {
        return Err(ApiError::bad_request("missing body"));
    }
    let is_json = headers.get(CONTENT_TYPE).is_none_or(|value| {
        let media = value.to_str().unwrap_or_default();
        let media = media
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        media.split_once('/').is_some_and(|(main, sub)| {
            main == "application" && (sub == "json" || sub.ends_with("+json"))
        })
    });
    if !is_json {
        return Err(ApiError::bad_request("invalid body"));
    }
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(ApiError::bad_request("invalid body")),
        Err(_) => Err(ApiError::bad_request("body is not valid JSON")),
    }
}

/// Reads fields one by one, collecting a problem for each bad one, in field order.
pub struct Fields<'a> {
    map: &'a Map<String, Value>,
    problems: Vec<String>,
}

impl<'a> Fields<'a> {
    pub fn new(map: &'a Map<String, Value>) -> Self {
        Self {
            map,
            problems: Vec::new(),
        }
    }

    fn missing(&mut self, name: &str) {
        self.problems.push(format!("missing {name}"));
    }

    fn invalid(&mut self, name: &str) {
        self.problems.push(format!("invalid {name}"));
    }

    /// A required string with at least `min_chars` characters.
    pub fn required_str(&mut self, name: &str, min_chars: usize) -> String {
        self.bounded_str(name, min_chars, usize::MAX)
    }

    /// A required string with between `min_chars` and `max_chars` characters.
    pub fn bounded_str(&mut self, name: &str, min_chars: usize, max_chars: usize) -> String {
        match self.map.get(name) {
            None => self.missing(name),
            Some(Value::String(s)) if (min_chars..=max_chars).contains(&s.chars().count()) => {
                return s.clone();
            }
            Some(_) => self.invalid(name),
        }
        String::new()
    }

    /// An optional number in `range`, with a default; numeric strings count, as in pydantic.
    pub fn number_or(&mut self, name: &str, default: f64, ok: impl Fn(f64) -> bool) -> f64 {
        let Some(value) = self.map.get(name) else {
            return default;
        };
        let parsed = match value {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        match parsed {
            Some(x) if ok(x) => x,
            _ => {
                self.invalid(name);
                default
            }
        }
    }

    /// An optional string: absent and `null` are both `None`.
    pub fn optional_str(&mut self, name: &str) -> Option<String> {
        match self.map.get(name) {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                self.invalid(name);
                None
            }
        }
    }

    /// A string with a default when absent; `null` is invalid, as in pydantic.
    pub fn str_or(&mut self, name: &str, default: &str) -> String {
        match self.map.get(name) {
            None => default.to_owned(),
            Some(Value::String(s)) => s.clone(),
            Some(_) => {
                self.invalid(name);
                default.to_owned()
            }
        }
    }

    /// A bool with a default, accepting what pydantic's lax mode accepts from JSON.
    pub fn bool_or(&mut self, name: &str, default: bool) -> bool {
        let Some(value) = self.map.get(name) else {
            return default;
        };
        let parsed = match value {
            Value::Bool(b) => Some(*b),
            Value::Number(n) => match n.as_f64() {
                Some(0.0) => Some(false),
                Some(x) if (x - 1.0).abs() < f64::EPSILON => Some(true),
                _ => None,
            },
            Value::String(s) => match s.to_ascii_lowercase().as_str() {
                "0" | "off" | "f" | "false" | "n" | "no" => Some(false),
                "1" | "on" | "t" | "true" | "y" | "yes" => Some(true),
                _ => None,
            },
            _ => None,
        };
        parsed.unwrap_or_else(|| {
            self.invalid(name);
            default
        })
    }

    /// A number at least 0 with a default; numeric strings and booleans count, as in pydantic.
    pub fn non_negative_or(&mut self, name: &str, default: f64) -> f64 {
        let Some(value) = self.map.get(name) else {
            return default;
        };
        let parsed = match value {
            Value::Number(n) => n.as_f64(),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        match parsed {
            Some(x) if x >= 0.0 => x,
            _ => {
                self.invalid(name);
                default
            }
        }
    }

    /// A required string that must parse with `parse`.
    pub fn required<T>(&mut self, name: &str, parse: impl Fn(&str) -> Option<T>) -> Option<T> {
        match self.map.get(name) {
            None => {
                self.missing(name);
                None
            }
            Some(Value::String(s)) => {
                let parsed = parse(s);
                if parsed.is_none() {
                    self.invalid(name);
                }
                parsed
            }
            Some(_) => {
                self.invalid(name);
                None
            }
        }
    }

    /// Every problem found, joined as one 400 message; `Ok` when there were none.
    ///
    /// # Errors
    /// At least one field was missing or invalid.
    pub fn finish(self) -> Result<(), ApiError> {
        problems_to_error(self.problems)
    }
}

fn problems_to_error(problems: Vec<String>) -> Result<(), ApiError> {
    if problems.is_empty() {
        return Ok(());
    }
    let mut unique: Vec<String> = Vec::new();
    for p in problems {
        if !unique.contains(&p) {
            unique.push(p);
        }
    }
    Err(ApiError::bad_request(unique.join("; ")))
}

/// Query parameters as a JSON object of strings; a repeated name keeps its last value.
pub fn query_object(raw: Option<&str>) -> Map<String, Value> {
    let pairs: Vec<(String, String)> =
        serde_urlencoded::from_str(raw.unwrap_or_default()).unwrap_or_default();
    pairs
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use serde_json::json;

    fn headers(content_type: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Some(ct) = content_type {
            h.insert(CONTENT_TYPE, HeaderValue::from_str(ct).unwrap());
        }
        h
    }

    fn message(r: Result<Map<String, Value>, ApiError>) -> String {
        r.unwrap_err().message
    }

    #[test]
    fn body_rules_follow_fastapi() {
        let ok = br#"{"a": 1}"#;
        assert!(json_object(&headers(None), ok).is_ok());
        assert!(json_object(&headers(Some("application/json; charset=utf-8")), ok).is_ok());
        assert!(json_object(&headers(Some("application/vnd.x+json")), ok).is_ok());
        assert_eq!(
            message(json_object(&headers(Some("text/plain")), ok)),
            "invalid body"
        );
        assert_eq!(message(json_object(&headers(None), b"")), "missing body");
        assert_eq!(
            message(json_object(&headers(None), b"{oops")),
            "body is not valid JSON"
        );
        assert_eq!(message(json_object(&headers(None), b"[1]")), "invalid body");
    }

    #[test]
    fn fields_collect_problems_in_order() {
        let map = json!({"b": 5, "c": "", "d": "yes", "e": "-1"})
            .as_object()
            .unwrap()
            .clone();
        let mut f = Fields::new(&map);
        f.required_str("a", 1);
        f.required_str("b", 1);
        f.required_str("c", 1);
        assert!(f.bool_or("d", false));
        assert!((f.non_negative_or("e", 0.0)).abs() < f64::EPSILON);
        assert_eq!(
            f.finish().unwrap_err().message,
            "missing a; invalid b; invalid c; invalid e"
        );
    }

    #[test]
    fn lax_values() {
        let map = json!({"n": "12.5", "t": true, "s": null, "x": 2})
            .as_object()
            .unwrap()
            .clone();
        let mut f = Fields::new(&map);
        assert!((f.non_negative_or("n", 0.0) - 12.5).abs() < f64::EPSILON);
        assert!((f.non_negative_or("t", 0.0) - 1.0).abs() < f64::EPSILON);
        assert_eq!(f.optional_str("s"), None);
        assert_eq!(f.str_or("s", "d"), "d");
        assert!(!f.bool_or("x", false));
        assert_eq!(f.finish().unwrap_err().message, "invalid s; invalid x");
    }

    #[test]
    fn query_keeps_the_last_value() {
        let q = query_object(Some("type=phone&value=1&value=%2B91%2098"));
        assert_eq!(q["value"], "+91 98");
        assert_eq!(query_object(None).len(), 0);
    }
}
