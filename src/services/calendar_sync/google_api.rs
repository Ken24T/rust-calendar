use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::time::Duration as StdDuration;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use reqwest::blocking::Client;
use reqwest::header::RETRY_AFTER;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use thiserror::Error;

use crate::models::calendar_source::CalendarSource;
use crate::models::event::Event;

const GOOGLE_CALENDAR_EVENTS_ENDPOINT: &str = "https://www.googleapis.com/calendar/v3/calendars";
const DEFAULT_GOOGLE_API_BACKOFF_MINUTES: i64 = 15;

#[derive(Debug, Error)]
pub enum GoogleCalendarApiError {
    #[error("Google API sync token expired")]
    SyncTokenExpired,
    #[error(
        "Google Calendar API requested backoff ({status_code}); retry after {retry_after_minutes} minute(s)"
    )]
    RetryAfter {
        status_code: u16,
        retry_after_minutes: i64,
    },
}

impl GoogleCalendarApiError {
    pub fn is_sync_token_expired(&self) -> bool {
        matches!(self, Self::SyncTokenExpired)
    }

    pub fn retry_after_minutes(&self) -> Option<i64> {
        match self {
            Self::RetryAfter {
                retry_after_minutes,
                ..
            } => Some(*retry_after_minutes),
            Self::SyncTokenExpired => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoogleRemoteEvent {
    pub remote_event_id: String,
    pub external_uid: String,
    pub etag: Option<String>,
    pub updated_at: Option<String>,
    pub payload_hash: String,
    pub status: Option<String>,
    pub event: Option<Event>,
}

impl GoogleRemoteEvent {
    pub fn is_cancelled(&self) -> bool {
        self.status.as_deref() == Some("cancelled")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoogleEventsSyncPayload {
    pub items: Vec<GoogleRemoteEvent>,
    pub next_sync_token: Option<String>,
}

pub struct GoogleCalendarApiClient {
    client: Client,
    access_token: String,
}

pub(crate) trait GoogleOutboundWriter {
    fn update_event(
        &self,
        source: &CalendarSource,
        remote_event_id: &str,
        payload_json: &str,
    ) -> Result<GoogleRemoteEvent>;

    fn delete_event(&self, source: &CalendarSource, remote_event_id: &str) -> Result<()>;

    fn patch_detached_instance(
        &self,
        source: &CalendarSource,
        parent_remote_event_id: &str,
        detached_external_uid: &str,
        payload_json: &str,
    ) -> Result<GoogleRemoteEvent>;
}

impl GoogleCalendarApiClient {
    pub fn new(access_token: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(StdDuration::from_secs(30))
            .build()
            .context("Failed to build Google Calendar API HTTP client")?;

        Ok(Self {
            client,
            access_token: access_token.into(),
        })
    }

    pub fn fetch_events_incremental(
        &self,
        source: &CalendarSource,
    ) -> Result<GoogleEventsSyncPayload> {
        let calendar_id = source.google_calendar_id().ok_or_else(|| {
            anyhow!("Calendar source does not contain a valid Google calendar ID")
        })?;

        let mut page_token: Option<String> = None;
        let mut items = Vec::new();

        loop {
            let url = format!(
                "{}/{}/events",
                GOOGLE_CALENDAR_EVENTS_ENDPOINT,
                urlencoding::encode(&calendar_id)
            );

            let mut request = self
                .client
                .get(url)
                .bearer_auth(&self.access_token)
                .query(&[("showDeleted", "true"), ("maxResults", "2500")]);

            if let Some(sync_token) = source.api_sync_token.as_deref() {
                request = request.query(&[("syncToken", sync_token)]);
            } else {
                let now = Local::now();
                let time_min = (now - Duration::days(source.sync_past_days.max(0))).to_rfc3339();
                let time_max = (now + Duration::days(source.sync_future_days.max(1))).to_rfc3339();
                request = request.query(&[
                    ("timeMin", time_min.as_str()),
                    ("timeMax", time_max.as_str()),
                ]);
            }

            if let Some(page) = page_token.as_deref() {
                request = request.query(&[("pageToken", page)]);
            }

            let response = request
                .send()
                .context("Failed to call Google Calendar events API")?;

            if response.status().as_u16() == 410 {
                return Err(GoogleCalendarApiError::SyncTokenExpired.into());
            }

            if matches!(response.status().as_u16(), 429 | 503) {
                let status_code = response.status().as_u16();
                let retry_after_minutes = parse_retry_after_minutes(
                    response
                        .headers()
                        .get(RETRY_AFTER)
                        .and_then(|value| value.to_str().ok()),
                )
                .unwrap_or(DEFAULT_GOOGLE_API_BACKOFF_MINUTES);
                return Err(GoogleCalendarApiError::RetryAfter {
                    status_code,
                    retry_after_minutes,
                }
                .into());
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().unwrap_or_default();
                return Err(anyhow!(
                    "Google Calendar events API failed ({status}): {}",
                    body.trim()
                ));
            }

            let payload: GoogleEventsResponse = response
                .json()
                .context("Failed to parse Google Calendar events response")?;

            items.extend(payload.to_remote_events()?);
            page_token = payload.next_page_token;
            if page_token.is_none() {
                return Ok(GoogleEventsSyncPayload {
                    items,
                    next_sync_token: payload.next_sync_token,
                });
            }
        }
    }

    pub fn parse_events_response_body(body: &str) -> Result<GoogleEventsSyncPayload> {
        let payload: GoogleEventsResponse =
            serde_json::from_str(body).context("Failed to parse Google events response body")?;

        Ok(GoogleEventsSyncPayload {
            items: payload.to_remote_events()?,
            next_sync_token: payload.next_sync_token,
        })
    }

    fn parse_single_event_response_body(body: &str) -> Result<GoogleRemoteEvent> {
        let item: GoogleEventItem =
            serde_json::from_str(body).context("Failed to parse Google event response body")?;

        item.try_into_remote_event()?
            .ok_or_else(|| anyhow!("Google event response did not contain a usable event identity"))
    }

    fn calendar_id(source: &CalendarSource) -> Result<String> {
        source
            .google_calendar_id()
            .ok_or_else(|| anyhow!("Calendar source does not contain a valid Google calendar ID"))
    }

    fn event_request_url(source: &CalendarSource, remote_event_id: &str) -> Result<String> {
        let calendar_id = Self::calendar_id(source)?;
        Ok(format!(
            "{}/{}/events/{}",
            GOOGLE_CALENDAR_EVENTS_ENDPOINT,
            urlencoding::encode(&calendar_id),
            urlencoding::encode(remote_event_id)
        ))
    }

    fn instances_request_url(source: &CalendarSource, remote_event_id: &str) -> Result<String> {
        let calendar_id = Self::calendar_id(source)?;
        Ok(format!(
            "{}/{}/events/{}/instances",
            GOOGLE_CALENDAR_EVENTS_ENDPOINT,
            urlencoding::encode(&calendar_id),
            urlencoding::encode(remote_event_id)
        ))
    }

    fn find_detached_instance_remote(
        &self,
        source: &CalendarSource,
        parent_remote_event_id: &str,
        detached_external_uid: &str,
    ) -> Result<GoogleRemoteEvent> {
        let url = Self::instances_request_url(source, parent_remote_event_id)?;
        let recurrence_token = detached_external_uid
            .split("::RID::")
            .nth(1)
            .ok_or_else(|| {
                anyhow!(
                    "Detached instance identity '{}' is missing recurrence token",
                    detached_external_uid
                )
            })?;
        let (time_min, time_max) = detached_instance_window(recurrence_token)?;

        let response = self
            .client
            .get(url)
            .bearer_auth(&self.access_token)
            .query(&[
                ("showDeleted", "true"),
                ("timeMin", time_min.as_str()),
                ("timeMax", time_max.as_str()),
                ("maxResults", "250"),
            ])
            .send()
            .context("Failed to call Google Calendar instances API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Google Calendar instances API failed ({status}): {}",
                body.trim()
            ));
        }

        let payload: GoogleEventsResponse = response
            .json()
            .context("Failed to parse Google Calendar instances response")?;

        payload
            .to_remote_events()?
            .into_iter()
            .find(|event| event.external_uid == detached_external_uid)
            .ok_or_else(|| {
                anyhow!(
                    "Could not find Google instance matching detached identity '{}'",
                    detached_external_uid
                )
            })
    }
}

impl GoogleOutboundWriter for GoogleCalendarApiClient {
    fn update_event(
        &self,
        source: &CalendarSource,
        remote_event_id: &str,
        payload_json: &str,
    ) -> Result<GoogleRemoteEvent> {
        let url = Self::event_request_url(source, remote_event_id)?;
        let body = build_google_event_request_body(payload_json)?;
        let response = self
            .client
            .patch(url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .context("Failed to call Google Calendar event update API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Google Calendar event update failed ({status}): {}",
                body.trim()
            ));
        }

        Self::parse_single_event_response_body(&response.text().unwrap_or_default())
    }

    fn delete_event(&self, source: &CalendarSource, remote_event_id: &str) -> Result<()> {
        let url = Self::event_request_url(source, remote_event_id)?;
        let response = self
            .client
            .delete(url)
            .bearer_auth(&self.access_token)
            .send()
            .context("Failed to call Google Calendar event delete API")?;

        if response.status().as_u16() == 404 {
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Google Calendar event delete failed ({status}): {}",
                body.trim()
            ));
        }

        Ok(())
    }

    fn patch_detached_instance(
        &self,
        source: &CalendarSource,
        parent_remote_event_id: &str,
        detached_external_uid: &str,
        payload_json: &str,
    ) -> Result<GoogleRemoteEvent> {
        let instance = self.find_detached_instance_remote(
            source,
            parent_remote_event_id,
            detached_external_uid,
        )?;
        let remote_event_id = instance.remote_event_id.clone();
        self.update_event(source, &remote_event_id, payload_json)
    }
}

fn build_google_event_request_body(payload_json: &str) -> Result<Value> {
    let payload: Value =
        serde_json::from_str(payload_json).context("Failed to parse outbound payload JSON")?;

    let title = payload
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Untitled event");
    let start = payload
        .get("start")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Outbound payload missing start"))?;
    let end = payload
        .get("end")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Outbound payload missing end"))?;
    let all_day = payload
        .get("all_day")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut body = Map::new();
    body.insert("summary".to_string(), Value::String(title.to_string()));

    if let Some(description) = payload.get("description").and_then(Value::as_str) {
        if !description.trim().is_empty() {
            body.insert(
                "description".to_string(),
                Value::String(description.to_string()),
            );
        }
    }

    if let Some(location) = payload.get("location").and_then(Value::as_str) {
        if !location.trim().is_empty() {
            body.insert("location".to_string(), Value::String(location.to_string()));
        }
    }

    if all_day {
        let start_date = DateTime::parse_from_rfc3339(start)
            .with_context(|| format!("Invalid outbound start datetime '{}'", start))?
            .with_timezone(&Local)
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let end_date = DateTime::parse_from_rfc3339(end)
            .with_context(|| format!("Invalid outbound end datetime '{}'", end))?
            .with_timezone(&Local)
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        body.insert("start".to_string(), json!({ "date": start_date }));
        body.insert("end".to_string(), json!({ "date": end_date }));
    } else {
        body.insert("start".to_string(), json!({ "dateTime": start }));
        body.insert("end".to_string(), json!({ "dateTime": end }));
    }

    let mut recurrence = Vec::new();
    if let Some(rule) = payload.get("recurrence_rule").and_then(Value::as_str) {
        if !rule.trim().is_empty() {
            recurrence.push(Value::String(format!("RRULE:{}", rule)));
        }
    }

    if let Some(exceptions) = payload
        .get("recurrence_exceptions")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
    {
        let exdate_values = exceptions
            .iter()
            .filter_map(Value::as_str)
            .map(|value| {
                DateTime::parse_from_rfc3339(value)
                    .with_context(|| format!("Invalid outbound recurrence exception '{}'", value))
                    .map(|dt| {
                        if all_day {
                            dt.with_timezone(&Local).format("%Y%m%d").to_string()
                        } else {
                            dt.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ").to_string()
                        }
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        if !exdate_values.is_empty() {
            let exdate_prefix = if all_day {
                "EXDATE;VALUE=DATE:"
            } else {
                "EXDATE:"
            };
            recurrence.push(Value::String(format!(
                "{}{}",
                exdate_prefix,
                exdate_values.join(",")
            )));
        }
    }

    if !recurrence.is_empty() {
        body.insert("recurrence".to_string(), Value::Array(recurrence));
    }

    Ok(Value::Object(body))
}

fn parse_retry_after_minutes(value: Option<&str>) -> Option<i64> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }

    let seconds: i64 = raw.parse().ok()?;
    if seconds <= 0 {
        return Some(1);
    }

    Some(((seconds + 59) / 60).max(1))
}

fn detached_instance_window(recurrence_token: &str) -> Result<(String, String)> {
    if recurrence_token.len() == 8 {
        let date = NaiveDate::parse_from_str(recurrence_token, "%Y%m%d")
            .with_context(|| format!("Invalid detached recurrence token '{}'", recurrence_token))?;
        let time_min = date
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .and_local_timezone(Local)
            .earliest()
            .ok_or_else(|| {
                anyhow!(
                    "Detached recurrence token '{}' is invalid in local timezone",
                    recurrence_token
                )
            })?;
        let time_max = (date + Duration::days(1))
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .and_local_timezone(Local)
            .earliest()
            .ok_or_else(|| {
                anyhow!(
                    "Detached recurrence token '{}' is invalid in local timezone",
                    recurrence_token
                )
            })?;
        return Ok((time_min.to_rfc3339(), time_max.to_rfc3339()));
    }

    let parsed = NaiveDateTime::parse_from_str(recurrence_token, "%Y%m%dT%H%M%SZ")
        .with_context(|| format!("Invalid detached recurrence token '{}'", recurrence_token))?;
    let parsed = Utc.from_utc_datetime(&parsed);
    let time_min = (parsed - Duration::hours(12)).to_rfc3339();
    let time_max = (parsed + Duration::hours(12)).to_rfc3339();
    Ok((time_min, time_max))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleEventsResponse {
    items: Option<Vec<GoogleEventItem>>,
    next_page_token: Option<String>,
    next_sync_token: Option<String>,
}

impl GoogleEventsResponse {
    fn to_remote_events(&self) -> Result<Vec<GoogleRemoteEvent>> {
        self.items
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.try_into_remote_event().transpose())
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleEventItem {
    id: String,
    etag: Option<String>,
    status: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    updated: Option<String>,
    #[serde(rename = "iCalUID")]
    i_cal_uid: Option<String>,
    recurrence: Option<Vec<String>>,
    recurring_event_id: Option<String>,
    original_start_time: Option<GoogleEventDateTime>,
    start: Option<GoogleEventDateTime>,
    end: Option<GoogleEventDateTime>,
}

impl GoogleEventItem {
    fn try_into_remote_event(self) -> Result<Option<GoogleRemoteEvent>> {
        let external_uid = match self.external_uid() {
            Some(uid) => uid,
            None => return Ok(None),
        };

        let payload_hash = self.payload_hash()?;
        let is_cancelled = self.status.as_deref() == Some("cancelled");
        let event = if is_cancelled {
            None
        } else {
            Some(self.to_local_event()?)
        };

        Ok(Some(GoogleRemoteEvent {
            remote_event_id: self.id,
            external_uid,
            etag: self.etag,
            updated_at: self.updated,
            payload_hash,
            status: self.status,
            event,
        }))
    }

    fn external_uid(&self) -> Option<String> {
        let base_uid = self
            .i_cal_uid
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())?
            .to_string();

        match self
            .original_start_time
            .as_ref()
            .and_then(|value| value.recurrence_id_token())
        {
            Some(recurrence_id) => Some(format!("{}::RID::{}", base_uid, recurrence_id)),
            None => Some(base_uid),
        }
    }

    fn payload_hash(&self) -> Result<String> {
        let canonical = serde_json::to_string(&(
            &self.id,
            &self.etag,
            &self.status,
            &self.summary,
            &self.description,
            &self.location,
            &self.updated,
            &self.i_cal_uid,
            &self.recurrence,
            &self.recurring_event_id,
            &self.original_start_time,
            &self.start,
            &self.end,
        ))
        .context("Failed to serialize Google event payload for hashing")?;

        let mut hasher = DefaultHasher::new();
        canonical.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }

    fn to_local_event(&self) -> Result<Event> {
        let start = self
            .start
            .as_ref()
            .ok_or_else(|| anyhow!("Google event missing start time"))?;
        let end = self
            .end
            .as_ref()
            .ok_or_else(|| anyhow!("Google event missing end time"))?;

        let (start_local, all_day) = start.to_local_datetime()?;
        let (end_local, _) = end.to_local_datetime()?;

        let mut builder = Event::builder()
            .title(self.summary.as_deref().unwrap_or("Untitled Google event"))
            .start(start_local)
            .end(end_local)
            .all_day(all_day);

        if let Some(description) = self
            .description
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            builder = builder.description(description);
        }
        if let Some(location) = self
            .location
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            builder = builder.location(location);
        }
        if let Some(recurrence_rule) = self
            .recurrence
            .as_ref()
            .and_then(|rules| rules.iter().find(|rule| rule.starts_with("RRULE:")))
            .map(|rule| rule.trim_start_matches("RRULE:").to_string())
        {
            builder = builder.recurrence_rule(recurrence_rule);
        }

        let mut event = builder.build().map_err(|err| anyhow!(err))?;
        event.recurrence_exceptions =
            parse_google_recurrence_exceptions(self.recurrence.as_deref().unwrap_or(&[]))?;

        Ok(event)
    }
}

fn parse_google_recurrence_exceptions(
    recurrence: &[String],
) -> Result<Option<Vec<DateTime<Local>>>> {
    let mut exceptions = Vec::new();

    for entry in recurrence {
        if !entry.starts_with("EXDATE") {
            continue;
        }

        let (key_part, value_part) = entry.split_once(':').ok_or_else(|| {
            anyhow!(
                "Invalid Google EXDATE recurrence entry '{}': missing ':'",
                entry
            )
        })?;
        let tzid = extract_tzid(key_part);
        let is_value_date = key_part.contains("VALUE=DATE");

        for raw_value in value_part.split(',') {
            let exdate = raw_value.trim();
            if exdate.is_empty() {
                continue;
            }

            let parsed = if is_value_date {
                parse_google_date(exdate)?
            } else {
                parse_google_datetime_with_tzid(exdate, tzid)?
            };
            exceptions.push(parsed);
        }
    }

    if exceptions.is_empty() {
        return Ok(None);
    }

    exceptions.sort();
    exceptions.dedup();
    Ok(Some(exceptions))
}

fn extract_tzid(key_part: &str) -> Option<&str> {
    key_part
        .split(';')
        .find_map(|part| part.strip_prefix("TZID="))
}

fn parse_google_datetime_with_tzid(s: &str, tzid: Option<&str>) -> Result<DateTime<Local>> {
    let has_utc_suffix = s.ends_with('Z');
    let normalized = s.trim_end_matches('Z');

    if normalized.len() < 15 {
        return Err(anyhow!("Invalid Google recurrence datetime '{}'", s));
    }

    let naive = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(
            normalized[0..4].parse()?,
            normalized[4..6].parse()?,
            normalized[6..8].parse()?,
        )
        .ok_or_else(|| anyhow!("Invalid Google recurrence date '{}'", s))?,
        NaiveTime::from_hms_opt(
            normalized[9..11].parse()?,
            normalized[11..13].parse()?,
            normalized[13..15].parse()?,
        )
        .ok_or_else(|| anyhow!("Invalid Google recurrence time '{}'", s))?,
    );

    if has_utc_suffix {
        return Ok(Utc.from_utc_datetime(&naive).with_timezone(&Local));
    }

    if let Some(tz_name) = tzid {
        if let Ok(timezone) = Tz::from_str(tz_name) {
            if let Some(dt) = timezone.from_local_datetime(&naive).earliest() {
                return Ok(dt.with_timezone(&Local));
            }
        }
    }

    Local
        .from_local_datetime(&naive)
        .earliest()
        .ok_or_else(|| anyhow!("Invalid Google recurrence datetime '{}'", s))
}

fn parse_google_date(s: &str) -> Result<DateTime<Local>> {
    let parsed = NaiveDate::parse_from_str(s, "%Y%m%d")
        .with_context(|| format!("Invalid Google recurrence date '{}'", s))?;

    parsed
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .and_local_timezone(Local)
        .earliest()
        .ok_or_else(|| {
            anyhow!(
                "Google recurrence date '{}' is invalid in local timezone",
                s
            )
        })
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleEventDateTime {
    date: Option<String>,
    date_time: Option<String>,
    time_zone: Option<String>,
}

impl GoogleEventDateTime {
    fn to_local_datetime(&self) -> Result<(DateTime<Local>, bool)> {
        if let Some(date_time) = self.date_time.as_deref() {
            let parsed = DateTime::parse_from_rfc3339(date_time)
                .with_context(|| format!("Invalid Google event datetime '{}'", date_time))?;
            return Ok((parsed.with_timezone(&Local), false));
        }

        if let Some(date) = self.date.as_deref() {
            let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .with_context(|| format!("Invalid Google event date '{}'", date))?;
            let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            let local = parsed
                .and_time(midnight)
                .and_local_timezone(Local)
                .single()
                .ok_or_else(|| anyhow!("Date '{}' is invalid in local timezone", date))?;
            return Ok((local, true));
        }

        Err(anyhow!("Google event missing date/dateTime field"))
    }

    fn recurrence_id_token(&self) -> Option<String> {
        if let Some(date_time) = self.date_time.as_deref() {
            let parsed = DateTime::parse_from_rfc3339(date_time).ok()?;
            return Some(
                parsed
                    .with_timezone(&chrono::Utc)
                    .format("%Y%m%dT%H%M%SZ")
                    .to_string(),
            );
        }

        self.date.as_deref().and_then(|date| {
            NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .ok()
                .map(|parsed| parsed.format("%Y%m%d").to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_google_event_request_body, detached_instance_window, parse_retry_after_minutes,
        GoogleCalendarApiClient, GoogleCalendarApiError,
    };
    use chrono::Utc;

    #[test]
    fn parse_google_events_response_maps_master_and_cancelled_items() {
        let body = r#"{
            "items": [
                {
                    "id": "remote-1",
                    "etag": "\"abc\"",
                    "status": "confirmed",
                    "summary": "API Event",
                    "description": "From Google",
                    "iCalUID": "uid-1",
                    "start": { "dateTime": "2026-03-10T09:00:00Z" },
                    "end": { "dateTime": "2026-03-10T10:00:00Z" }
                },
                {
                    "id": "remote-2",
                    "status": "cancelled",
                    "iCalUID": "uid-2"
                }
            ],
            "nextSyncToken": "sync-token-2"
        }"#;

        let parsed = GoogleCalendarApiClient::parse_events_response_body(body).unwrap();
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.next_sync_token.as_deref(), Some("sync-token-2"));
        assert_eq!(parsed.items[0].external_uid, "uid-1");
        assert!(parsed.items[0].event.is_some());
        assert!(parsed.items[1].is_cancelled());
    }

    #[test]
    fn parse_google_events_response_builds_instance_uid() {
        let body = r#"{
            "items": [
                {
                    "id": "remote-instance",
                    "status": "confirmed",
                    "iCalUID": "series-uid",
                    "originalStartTime": { "dateTime": "2026-03-11T09:00:00-05:00" },
                    "start": { "dateTime": "2026-03-11T12:00:00-05:00" },
                    "end": { "dateTime": "2026-03-11T13:00:00-05:00" },
                    "summary": "Moved Instance"
                }
            ]
        }"#;

        let parsed = GoogleCalendarApiClient::parse_events_response_body(body).unwrap();
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(
            parsed.items[0].external_uid,
            "series-uid::RID::20260311T140000Z"
        );
    }

    #[test]
    fn parse_google_events_response_preserves_timed_exdates() {
        let body = r#"{
            "items": [
                {
                    "id": "remote-master",
                    "status": "confirmed",
                    "iCalUID": "series-uid",
                    "summary": "Series With Exceptions",
                    "recurrence": [
                        "RRULE:FREQ=WEEKLY;BYDAY=TU",
                        "EXDATE:20260317T090000Z,20260324T090000Z"
                    ],
                    "start": { "dateTime": "2026-03-10T09:00:00Z" },
                    "end": { "dateTime": "2026-03-10T10:00:00Z" }
                }
            ]
        }"#;

        let parsed = GoogleCalendarApiClient::parse_events_response_body(body).unwrap();
        let event = parsed.items[0].event.as_ref().unwrap();
        let exceptions = event.recurrence_exceptions.as_ref().unwrap();

        assert_eq!(exceptions.len(), 2);
        assert_eq!(
            exceptions[0].with_timezone(&Utc).to_rfc3339(),
            "2026-03-17T09:00:00+00:00"
        );
        assert_eq!(
            exceptions[1].with_timezone(&Utc).to_rfc3339(),
            "2026-03-24T09:00:00+00:00"
        );
    }

    #[test]
    fn parse_google_events_response_preserves_all_day_exdates() {
        let body = r#"{
            "items": [
                {
                    "id": "remote-all-day",
                    "status": "confirmed",
                    "iCalUID": "all-day-uid",
                    "summary": "All Day Series",
                    "recurrence": [
                        "RRULE:FREQ=DAILY;COUNT=3",
                        "EXDATE;VALUE=DATE:20260311"
                    ],
                    "start": { "date": "2026-03-10" },
                    "end": { "date": "2026-03-11" }
                }
            ]
        }"#;

        let parsed = GoogleCalendarApiClient::parse_events_response_body(body).unwrap();
        let event = parsed.items[0].event.as_ref().unwrap();
        let exceptions = event.recurrence_exceptions.as_ref().unwrap();

        assert!(event.all_day);
        assert_eq!(exceptions.len(), 1);
        assert_eq!(exceptions[0].format("%Y%m%d").to_string(), "20260311");
    }

    #[test]
    fn build_google_event_request_body_includes_recurrence_and_exdates() {
        let body = build_google_event_request_body(
            r#"{
                "title":"Recurring",
                "start":"2026-03-10T09:00:00+00:00",
                "end":"2026-03-10T10:00:00+00:00",
                "all_day":false,
                "recurrence_rule":"FREQ=WEEKLY;BYDAY=TU",
                "recurrence_exceptions":["2026-03-17T09:00:00+00:00"]
            }"#,
        )
        .unwrap();

        assert_eq!(body["summary"], "Recurring");
        assert_eq!(body["start"]["dateTime"], "2026-03-10T09:00:00+00:00");
        assert_eq!(body["recurrence"][0], "RRULE:FREQ=WEEKLY;BYDAY=TU");
        assert_eq!(body["recurrence"][1], "EXDATE:20260317T090000Z");
    }

    #[test]
    fn build_google_event_request_body_includes_all_day_exdates() {
        let body = build_google_event_request_body(
            r#"{
                "title":"All Day Series",
                "start":"2026-03-04T00:00:00+00:00",
                "end":"2026-03-05T00:00:00+00:00",
                "all_day":true,
                "recurrence_rule":"FREQ=WEEKLY;BYDAY=WE",
                "recurrence_exceptions":["2026-03-11T00:00:00+00:00","2026-03-18T00:00:00+00:00"]
            }"#,
        )
        .unwrap();

        assert_eq!(body["recurrence"][0], "RRULE:FREQ=WEEKLY;BYDAY=WE");
        assert_eq!(body["recurrence"][1], "EXDATE;VALUE=DATE:20260311,20260318");
    }

    #[test]
    fn parse_retry_after_minutes_rounds_seconds_up_to_minutes() {
        assert_eq!(parse_retry_after_minutes(Some("1")), Some(1));
        assert_eq!(parse_retry_after_minutes(Some("61")), Some(2));
        assert_eq!(parse_retry_after_minutes(Some("120")), Some(2));
    }

    #[test]
    fn google_api_error_exposes_retry_after_minutes() {
        let error = GoogleCalendarApiError::RetryAfter {
            status_code: 429,
            retry_after_minutes: 25,
        };

        assert_eq!(error.retry_after_minutes(), Some(25));
        assert!(!error.is_sync_token_expired());
        assert!(GoogleCalendarApiError::SyncTokenExpired.is_sync_token_expired());
    }

    #[test]
    fn detached_instance_window_handles_timed_tokens() {
        let (time_min, time_max) = detached_instance_window("20260407T090000Z").unwrap();
        assert_eq!(time_min, "2026-04-06T21:00:00+00:00");
        assert_eq!(time_max, "2026-04-07T21:00:00+00:00");
    }
}
