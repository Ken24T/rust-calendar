use chrono::{DateTime, Local};
use rusqlite::{self, Result};

pub(crate) fn serialize_exceptions(exceptions: Option<&Vec<DateTime<Local>>>) -> Option<String> {
    exceptions.map(|dates| {
        let serialized: Vec<String> = dates.iter().map(|dt| dt.to_rfc3339()).collect();
        serde_json::to_string(&serialized).unwrap_or_default()
    })
}

pub(crate) fn deserialize_exceptions(json: Option<String>) -> Result<Option<Vec<DateTime<Local>>>> {
    let Some(json) = json else {
        return Ok(None);
    };

    let dates: Vec<String> = serde_json::from_str(&json)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let parsed = dates
        .into_iter()
        .filter_map(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|dt| dt.with_timezone(&Local))
        .collect();

    Ok(Some(parsed))
}

pub(crate) fn to_local_datetime(value: String) -> Result<DateTime<Local>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
}
