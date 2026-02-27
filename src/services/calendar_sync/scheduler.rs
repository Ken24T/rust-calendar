#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration as StdDuration;

use anyhow::Result;
use chrono::{DateTime, Duration, Local};
use rusqlite::Connection;

use super::engine::{CalendarSyncEngine, SyncRunResult};
use super::CalendarSourceService;

#[derive(Debug, Clone, Default)]
struct SourceScheduleState {
    next_run_at: Option<DateTime<Local>>,
    consecutive_failures: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SchedulerTickResult {
    pub attempted_source_ids: Vec<i64>,
    pub successful: Vec<SyncRunResult>,
    pub failed_sources: Vec<(i64, String)>,
    pub next_due_in: Option<StdDuration>,
}

impl SchedulerTickResult {
    pub fn attempted_count(&self) -> usize {
        self.attempted_source_ids.len()
    }
}

pub struct CalendarSyncScheduler {
    source_state: HashMap<i64, SourceScheduleState>,
    max_backoff_minutes: i64,
    startup_sync_ready_at: Option<DateTime<Local>>,
}

impl Default for CalendarSyncScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl CalendarSyncScheduler {
    pub fn new() -> Self {
        Self::with_startup_delay(Duration::seconds(20))
    }

    pub fn with_startup_delay(startup_delay: Duration) -> Self {
        let startup_sync_ready_at = if startup_delay <= Duration::zero() {
            None
        } else {
            Some(Local::now() + startup_delay)
        };

        Self {
            source_state: HashMap::new(),
            max_backoff_minutes: 60,
            startup_sync_ready_at,
        }
    }

    pub fn tick(&mut self, conn: &Connection) -> Result<SchedulerTickResult> {
        let now = Local::now();
        let engine = CalendarSyncEngine::new(conn)?;
        self.tick_with_runner_at(conn, now, |source_id| engine.sync_source(source_id))
    }

    pub fn tick_with_runner_at<F>(
        &mut self,
        conn: &Connection,
        now: DateTime<Local>,
        mut runner: F,
    ) -> Result<SchedulerTickResult>
    where
        F: FnMut(i64) -> Result<SyncRunResult>,
    {
        if let Some(ready_at) = self.startup_sync_ready_at {
            if now < ready_at {
                let wait = (ready_at - now)
                    .to_std()
                    .unwrap_or_else(|_| StdDuration::from_secs(0));
                return Ok(SchedulerTickResult {
                    next_due_in: Some(wait),
                    ..SchedulerTickResult::default()
                });
            }

            self.startup_sync_ready_at = None;
        }

        let source_service = CalendarSourceService::new(conn);
        let sources = source_service.list_all()?;

        let enabled_sources = sources
            .into_iter()
            .filter(|source| source.enabled)
            .collect::<Vec<_>>();

        self.source_state.retain(|source_id, _| {
            enabled_sources
                .iter()
                .any(|source| source.id == Some(*source_id))
        });

        let mut result = SchedulerTickResult::default();

        for source in enabled_sources {
            let Some(source_id) = source.id else {
                continue;
            };

            let state = self.source_state.entry(source_id).or_default();
            let is_due = state.next_run_at.is_none_or(|next_run_at| now >= next_run_at);
            if !is_due {
                continue;
            }

            result.attempted_source_ids.push(source_id);

            match runner(source_id) {
                Ok(sync_result) => {
                    state.consecutive_failures = 0;
                    state.next_run_at = Some(now + Duration::minutes(source.poll_interval_minutes.max(1)));
                    result.successful.push(sync_result);
                }
                Err(err) => {
                    state.consecutive_failures = state.consecutive_failures.saturating_add(1);
                    let backoff_minutes = Self::calculate_backoff_minutes(
                        source.poll_interval_minutes,
                        state.consecutive_failures,
                        self.max_backoff_minutes,
                    );
                    state.next_run_at = Some(now + Duration::minutes(backoff_minutes));

                    let redacted_error = Self::redact_error_message(&err.to_string(), &source.ics_url);
                    result.failed_sources.push((source_id, redacted_error));
                }
            }
        }

        let next_due_at = self
            .source_state
            .values()
            .filter_map(|state| state.next_run_at)
            .min();

        result.next_due_in = next_due_at.map(|next_due_at| {
            let delta = next_due_at - now;
            if delta <= Duration::zero() {
                StdDuration::from_secs(0)
            } else {
                delta.to_std().unwrap_or_else(|_| StdDuration::from_secs(0))
            }
        });

        Ok(result)
    }

    fn calculate_backoff_minutes(base_poll_minutes: i64, failures: u32, max_backoff_minutes: i64) -> i64 {
        let base = base_poll_minutes.max(1);
        if failures == 0 {
            return base;
        }

        let factor = 2_i64.saturating_pow(failures.min(10));
        let backoff = base.saturating_mul(factor);
        backoff.min(max_backoff_minutes.max(base))
    }

    fn redact_error_message(message: &str, source_url: &str) -> String {
        if source_url.is_empty() {
            return message.to_string();
        }

        message.replace(source_url, "***redacted-url***")
    }
}

#[cfg(test)]
mod tests {
    use super::CalendarSyncScheduler;
    use crate::services::calendar_sync::engine::SyncRunResult;
    use crate::services::database::Database;
    use chrono::{Local, TimeZone};
    use rusqlite::params;

    fn create_source(conn: &rusqlite::Connection, name: &str, poll_minutes: i64, enabled: bool) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                name,
                "google_ics",
                format!("https://calendar.google.com/calendar/ical/{}%40gmail.com/private-token/basic.ics", name),
                enabled as i32,
                poll_minutes,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn tick_respects_poll_intervals() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();

        let s1 = create_source(conn, "source1", 15, true);
        let s2 = create_source(conn, "source2", 30, true);

        let mut scheduler = CalendarSyncScheduler::with_startup_delay(chrono::Duration::zero());
        let now = Local.with_ymd_and_hms(2026, 2, 27, 10, 0, 0).unwrap();

        let first = scheduler
            .tick_with_runner_at(conn, now, |source_id| {
                Ok(SyncRunResult {
                    source_id,
                    ..SyncRunResult::default()
                })
            })
            .unwrap();

        assert_eq!(first.attempted_count(), 2);

        let second = scheduler
            .tick_with_runner_at(conn, now + chrono::Duration::minutes(5), |source_id| {
                Ok(SyncRunResult {
                    source_id,
                    ..SyncRunResult::default()
                })
            })
            .unwrap();

        assert_eq!(second.attempted_count(), 0);

        let third = scheduler
            .tick_with_runner_at(conn, now + chrono::Duration::minutes(16), |source_id| {
                Ok(SyncRunResult {
                    source_id,
                    ..SyncRunResult::default()
                })
            })
            .unwrap();

        assert_eq!(third.attempted_source_ids, vec![s1]);
        assert!(!third.attempted_source_ids.contains(&s2));
    }

    #[test]
    fn tick_isolates_failures_with_backoff() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();

        let failing = create_source(conn, "failing", 10, true);
        let healthy = create_source(conn, "healthy", 10, true);

        let mut scheduler = CalendarSyncScheduler::with_startup_delay(chrono::Duration::zero());
        let now = Local.with_ymd_and_hms(2026, 2, 27, 11, 0, 0).unwrap();

        let first = scheduler
            .tick_with_runner_at(conn, now, |source_id| {
                if source_id == failing {
                    anyhow::bail!("fetch failed for source {}", source_id);
                }
                Ok(SyncRunResult {
                    source_id,
                    ..SyncRunResult::default()
                })
            })
            .unwrap();

        assert_eq!(first.attempted_count(), 2);
        assert_eq!(first.failed_sources.len(), 1);

        let second = scheduler
            .tick_with_runner_at(conn, now + chrono::Duration::minutes(11), |source_id| {
                Ok(SyncRunResult {
                    source_id,
                    ..SyncRunResult::default()
                })
            })
            .unwrap();

        assert_eq!(second.attempted_source_ids, vec![healthy]);

        let third = scheduler
            .tick_with_runner_at(conn, now + chrono::Duration::minutes(21), |source_id| {
                Ok(SyncRunResult {
                    source_id,
                    ..SyncRunResult::default()
                })
            })
            .unwrap();

        assert!(third.attempted_source_ids.contains(&failing));
    }

    #[test]
    fn redact_error_hides_source_url() {
        let message = "Failed to fetch https://calendar.google.com/calendar/ical/a%40gmail.com/private-token/basic.ics";
        let source_url = "https://calendar.google.com/calendar/ical/a%40gmail.com/private-token/basic.ics";
        let redacted = CalendarSyncScheduler::redact_error_message(message, source_url);

        assert!(!redacted.contains(source_url));
        assert!(redacted.contains("***redacted-url***"));
    }

    #[test]
    fn tick_defers_initial_sync_until_startup_delay_elapses() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();

        let source_id = create_source(conn, "source1", 15, true);

        let mut scheduler = CalendarSyncScheduler::with_startup_delay(chrono::Duration::seconds(20));
        let now = Local.with_ymd_and_hms(2026, 2, 27, 12, 0, 0).unwrap();

        scheduler.startup_sync_ready_at = Some(now + chrono::Duration::seconds(20));

        let before_ready = scheduler
            .tick_with_runner_at(conn, now, |_source_id| Ok(SyncRunResult::default()))
            .unwrap();

        assert_eq!(before_ready.attempted_count(), 0);
        assert!(before_ready.next_due_in.is_some());

        let after_ready = scheduler
            .tick_with_runner_at(conn, now + chrono::Duration::seconds(21), |sid| {
                Ok(SyncRunResult {
                    source_id: sid,
                    ..SyncRunResult::default()
                })
            })
            .unwrap();

        assert_eq!(after_ready.attempted_source_ids, vec![source_id]);
    }
}
