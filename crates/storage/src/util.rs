use chrono::{DateTime, NaiveDate, Utc};
use roadboard_core::{Error as CoreError, Result};

pub fn map_sqlx_err(e: sqlx::Error) -> CoreError {
    match &e {
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("2067") => {
            CoreError::Conflict(db_err.message().to_string())
        }
        _ => CoreError::Storage(e.to_string()),
    }
}

pub fn enum_to_str<T: serde::Serialize>(val: &T) -> Result<String> {
    serde_json::to_value(val)
        .map_err(|e| CoreError::Storage(e.to_string()))
        .and_then(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| CoreError::Storage("enum serialized to non-string".to_string()))
        })
}

pub fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| CoreError::Storage(format!("invalid enum value '{}': {}", s, e)))
}

pub fn parse_ts(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| CoreError::Storage(format!("invalid timestamp '{}': {}", s, e)))
}

pub fn parse_ts_opt(s: Option<String>) -> Result<Option<DateTime<Utc>>> {
    s.as_deref().map(parse_ts).transpose()
}

pub fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| CoreError::Storage(format!("invalid date '{}': {}", s, e)))
}

pub fn parse_date_opt(s: Option<String>) -> Result<Option<NaiveDate>> {
    s.as_deref()
        .map(parse_date)
        .transpose()
}

/// Unwraps a NOT NULL column that sqlx returns as Option (SQLite dynamic typing).
pub fn req(opt: Option<String>, field: &'static str) -> Result<String> {
    opt.ok_or_else(|| {
        CoreError::Storage(format!("unexpected NULL for NOT NULL field '{}'", field))
    })
}
