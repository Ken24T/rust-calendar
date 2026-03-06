use crate::models::event::Event;
use anyhow::Result;
use chrono::{Duration, Local};

use super::utils::{parse_date, parse_datetime_with_tzid, unescape_text};

#[derive(Debug, Clone)]
pub struct ImportedIcsEvent {
    pub event: Event,
    pub uid: Option<String>,
    pub recurrence_id: Option<String>,
    pub raw_last_modified: Option<String>,
    pub has_start: bool,
    pub has_end: bool,
}

pub fn from_str(ics_content: &str) -> Result<Vec<Event>> {
    let imported = from_str_with_metadata(ics_content)?;
    Ok(imported.into_iter().map(|item| item.event).collect())
}

pub fn from_str_with_metadata(ics_content: &str) -> Result<Vec<ImportedIcsEvent>> {
    let mut events = Vec::new();
    let lines = unfold_ics_lines(ics_content);

    let mut in_event = false;
    let mut current_event: Option<ImportedIcsEvent> = None;

    for line in lines {
        let line = line.trim();

        if line == "BEGIN:VEVENT" {
            in_event = true;
            current_event = Some(ImportedIcsEvent {
                event: blank_event(),
                uid: None,
                recurrence_id: None,
                raw_last_modified: None,
                has_start: false,
                has_end: false,
            });
        } else if line == "END:VEVENT" {
            in_event = false;
            if let Some(mut event) = current_event.take() {
                finalize_event_times(&mut event);
                if !event.event.title.is_empty() {
                    events.push(event);
                }
            }
        } else if in_event {
            if let Some(event) = current_event.as_mut() {
                parse_event_property(line, event)?;
            }
        }
    }

    Ok(events)
}

fn unfold_ics_lines(ics_content: &str) -> Vec<String> {
    let mut unfolded: Vec<String> = Vec::new();

    for raw_line in ics_content.lines() {
        let is_single_space_continuation = raw_line.starts_with(' ') && !raw_line.starts_with("  ");
        let is_tab_continuation = raw_line.starts_with('\t');

        if is_single_space_continuation || is_tab_continuation {
            if let Some(last) = unfolded.last_mut() {
                last.push_str(raw_line[1..].trim_end());
            }
            continue;
        }

        unfolded.push(raw_line.trim_end().to_string());
    }

    unfolded
}

fn parse_event_property(line: &str, imported: &mut ImportedIcsEvent) -> Result<()> {
    if let Some(colon_pos) = line.find(':') {
        let (key_part, value) = line.split_at(colon_pos);
        let value = &value[1..];
        let tzid = extract_tzid(key_part);

        let key = if let Some(semicolon) = key_part.find(';') {
            &key_part[..semicolon]
        } else {
            key_part
        };

        match key {
            "UID" => {
                imported.uid = Some(unescape_text(value));
            }
            "RECURRENCE-ID" => {
                let parsed = if key_part.contains("VALUE=DATE") {
                    parse_date(value)?
                } else {
                    parse_datetime_with_tzid(value, tzid)?
                };
                imported.recurrence_id = Some(parsed.to_rfc3339());
            }
            "SUMMARY" => {
                imported.event.title = unescape_text(value);
            }
            "DESCRIPTION" => {
                imported.event.description = Some(unescape_text(value));
            }
            "LOCATION" => {
                imported.event.location = Some(unescape_text(value));
            }
            "CATEGORIES" => {
                imported.event.category = Some(unescape_text(value));
            }
            "X-APPLE-CALENDAR-COLOR" => {
                imported.event.color = Some(value.to_string());
            }
            "DTSTART" => {
                if key_part.contains("VALUE=DATE") {
                    imported.event.all_day = true;
                    imported.event.start = parse_date(value)?;
                } else {
                    imported.event.start = parse_datetime_with_tzid(value, tzid)?;
                }
                imported.has_start = true;
            }
            "DTEND" => {
                if key_part.contains("VALUE=DATE") {
                    imported.event.end = parse_date(value)?;
                } else {
                    imported.event.end = parse_datetime_with_tzid(value, tzid)?;
                }
                imported.has_end = true;
            }
            "RRULE" => {
                imported.event.recurrence_rule = Some(value.to_string());
            }
            "EXDATE" => {
                let dates: Result<Vec<_>> = value
                    .split(',')
                    .map(|s| {
                        let exdate = s.trim();
                        if key_part.contains("VALUE=DATE") {
                            parse_date(exdate)
                        } else {
                            parse_datetime_with_tzid(exdate, tzid)
                        }
                    })
                    .collect();
                imported.event.recurrence_exceptions = Some(dates?);
            }
            "CREATED" => {
                imported.event.created_at = Some(parse_datetime_with_tzid(value, tzid)?);
            }
            "LAST-MODIFIED" => {
                imported.raw_last_modified = Some(value.to_string());
                imported.event.updated_at = Some(parse_datetime_with_tzid(value, tzid)?);
            }
            _ => {}
        }
    }

    Ok(())
}

fn extract_tzid(key_part: &str) -> Option<&str> {
    key_part
        .split(';')
        .find_map(|part| part.strip_prefix("TZID="))
}

fn finalize_event_times(imported: &mut ImportedIcsEvent) {
    if !imported.has_start {
        return;
    }

    if !imported.has_end {
        imported.event.end = if imported.event.all_day {
            imported.event.start + Duration::days(1)
        } else {
            imported.event.start + Duration::hours(1)
        };
        return;
    }

    if imported.event.end <= imported.event.start {
        imported.event.end = if imported.event.all_day {
            imported.event.start + Duration::days(1)
        } else {
            imported.event.start + Duration::hours(1)
        };
    }
}

fn blank_event() -> Event {
    Event {
        id: None,
        title: String::new(),
        description: None,
        location: None,
        start: Local::now(),
        end: Local::now(),
        all_day: false,
        category: None,
        color: None,
        recurrence_rule: None,
        recurrence_exceptions: None,
        created_at: None,
        updated_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::from_str_with_metadata;

    #[test]
    fn test_import_with_metadata_uid_and_last_modified() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-uid-123
DTSTART:20260227T090000
DTEND:20260227T100000
SUMMARY:Test Event
LAST-MODIFIED:20260227T010203Z
END:VEVENT
END:VCALENDAR"#;

        let imported = from_str_with_metadata(ics).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].uid.as_deref(), Some("test-uid-123"));
        assert_eq!(
            imported[0].raw_last_modified.as_deref(),
            Some("20260227T010203Z")
        );
    }

    #[test]
    fn test_import_with_tzid_datetime() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-uid-tz
DTSTART;TZID=Australia/Sydney:20260227T090000
DTEND;TZID=Australia/Sydney:20260227T100000
SUMMARY:TZ Event
END:VEVENT
END:VCALENDAR"#;

        let imported = from_str_with_metadata(ics).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].event.title, "TZ Event");
    }

    #[test]
    fn test_import_exdate_value_date() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-uid-exdate
DTSTART;VALUE=DATE:20260227
DTEND;VALUE=DATE:20260228
SUMMARY:All-day recurring event
RRULE:FREQ=DAILY;COUNT=3
EXDATE;VALUE=DATE:20260228
END:VEVENT
END:VCALENDAR"#;

        let imported = from_str_with_metadata(ics).unwrap();
        assert_eq!(imported.len(), 1);
        let exdates = imported[0]
            .event
            .recurrence_exceptions
            .as_ref()
            .expect("EXDATE should be parsed");
        assert_eq!(exdates.len(), 1);
    }

    #[test]
    fn test_import_missing_dtend_defaults_to_one_hour_for_timed_event() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-no-end-timed
DTSTART:20260227T090000
SUMMARY:No End Timed
END:VEVENT
END:VCALENDAR"#;

        let imported = from_str_with_metadata(ics).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(
            imported[0].event.end,
            imported[0].event.start + chrono::Duration::hours(1)
        );
    }

    #[test]
    fn test_import_missing_dtend_defaults_to_one_day_for_all_day_event() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-no-end-allday
DTSTART;VALUE=DATE:20260227
SUMMARY:No End All Day
END:VEVENT
END:VCALENDAR"#;

        let imported = from_str_with_metadata(ics).unwrap();
        assert_eq!(imported.len(), 1);
        assert!(imported[0].event.all_day);
        assert_eq!(
            imported[0].event.end,
            imported[0].event.start + chrono::Duration::days(1)
        );
    }

    #[test]
    fn test_import_unfolds_folded_property_lines() {
        let ics = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:folded-uid\r\nSUMMARY:Long\r\n continuation title\r\nDTSTART:20260227T090000\r\nDTEND:20260227T100000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

        let imported = from_str_with_metadata(ics).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].event.title, "Longcontinuation title");
    }

    #[test]
    fn test_import_parses_recurrence_id_with_tzid() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:series-123
RECURRENCE-ID;TZID=Australia/Sydney:20260310T090000
DTSTART;TZID=Australia/Sydney:20260310T100000
DTEND;TZID=Australia/Sydney:20260310T110000
SUMMARY:Moved Instance
END:VEVENT
END:VCALENDAR"#;

        let imported = from_str_with_metadata(ics).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].uid.as_deref(), Some("series-123"));
        assert!(imported[0].recurrence_id.is_some());
    }
}
