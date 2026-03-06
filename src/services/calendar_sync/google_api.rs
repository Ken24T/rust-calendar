use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration as StdDuration;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime};
use reqwest::blocking::Client;
use serde::Deserialize;
use thiserror::Error;

use crate::models::calendar_source::CalendarSource;
use crate::models::event::Event;

const GOOGLE_CALENDAR_EVENTS_ENDPOINT: &str = "https://www.googleapis.com/calendar/v3/calendars";

#[derive(Debug, Error)]
pub enum GoogleCalendarApiError {
    #[error("Google API sync token expired")]
    SyncTokenExpired,
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

        builder.build().map_err(|err| anyhow!(err))
    }
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
    use super::GoogleCalendarApiClient;

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
}
