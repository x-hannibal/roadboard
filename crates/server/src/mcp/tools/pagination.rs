use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::Value;
use std::collections::HashMap;

use super::super::error::ToolError;

pub fn encode_cursor(last_id: &str) -> String {
    let json = serde_json::json!({ "last_id": last_id });
    URL_SAFE_NO_PAD.encode(json.to_string().as_bytes())
}

pub fn parse_cursor(args: &Value) -> Result<Option<String>, ToolError> {
    match args.get("cursor") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => {
            let s = v.as_str().ok_or(ToolError::InvalidCursor)?;
            let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|_| ToolError::InvalidCursor)?;
            let json: Value =
                serde_json::from_slice(&bytes).map_err(|_| ToolError::InvalidCursor)?;
            let last_id = json["last_id"].as_str().ok_or(ToolError::InvalidCursor)?;
            Ok(Some(last_id.to_string()))
        }
    }
}

pub fn parse_limit(args: &Value) -> Result<i64, ToolError> {
    match args.get("limit") {
        None | Some(Value::Null) => Ok(50),
        Some(v) => {
            let n = v
                .as_i64()
                .or_else(|| v.as_f64().map(|f| f as i64))
                .ok_or_else(|| {
                    let mut fields = HashMap::new();
                    fields.insert("limit".to_string(), "must be an integer".to_string());
                    ToolError::ValidationError { fields }
                })?;
            if !(1..=200).contains(&n) {
                let mut fields = HashMap::new();
                fields.insert("limit".to_string(), "must be between 1 and 200".to_string());
                return Err(ToolError::ValidationError { fields });
            }
            Ok(n)
        }
    }
}

pub fn build_page(items: Vec<Value>, limit: i64) -> Value {
    let next_cursor = if items.len() == limit as usize {
        items.last().and_then(|item| item["id"].as_str()).map(encode_cursor)
    } else {
        None
    };
    serde_json::json!({
        "schema_version": 1,
        "items": items,
        "next_cursor": next_cursor
    })
}
