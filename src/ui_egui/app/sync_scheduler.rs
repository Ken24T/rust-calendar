use std::time::Duration as StdDuration;
use std::{sync::mpsc, thread};
use std::time::Instant;

use super::CalendarApp;
use crate::services::calendar_sync::scheduler::SchedulerTickResult;
use crate::services::database::Database;

impl CalendarApp {
    pub(super) fn run_calendar_sync_scheduler(&mut self, ctx: &egui::Context) {
        let now = Instant::now();

        if let Some(rx) = &self.calendar_sync_result_rx {
            match rx.try_recv() {
                Ok(Ok(result)) => {
                    self.calendar_sync_in_progress = false;
                    self.calendar_sync_result_rx = None;
                    self.apply_scheduler_result(result, ctx);
                    return;
                }
                Ok(Err(err)) => {
                    log::error!("Scheduled calendar sync tick failed: {}", err);
                    self.calendar_sync_in_progress = false;
                    self.calendar_sync_result_rx = None;
                    self.calendar_sync_status_is_error = true;
                    self.calendar_sync_status_message =
                        Some("⚠ Calendar sync scheduler error".to_string());
                    self.calendar_sync_next_due_in = Some(StdDuration::from_secs(60));
                    self.calendar_sync_poll_due_at = Some(now + StdDuration::from_secs(60));
                    ctx.request_repaint_after(StdDuration::from_secs(60));
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.calendar_sync_in_progress = true;
                    self.calendar_sync_status_is_error = false;
                    if self.calendar_sync_status_message.is_none() {
                        self.calendar_sync_status_message =
                            Some("↻ Calendar sync running…".to_string());
                    }
                    ctx.request_repaint_after(StdDuration::from_millis(500));
                    return;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.calendar_sync_in_progress = false;
                    self.calendar_sync_result_rx = None;
                    self.calendar_sync_status_is_error = true;
                    self.calendar_sync_status_message =
                        Some("⚠ Calendar sync scheduler worker disconnected".to_string());
                    self.calendar_sync_poll_due_at = Some(now + StdDuration::from_secs(60));
                    ctx.request_repaint_after(StdDuration::from_secs(60));
                    return;
                }
            }
        }

        if self.calendar_sync_in_progress {
            ctx.request_repaint_after(StdDuration::from_millis(500));
            return;
        }

        if let Some(due_at) = self.calendar_sync_poll_due_at {
            if now < due_at {
                let remaining = due_at.duration_since(now);
                self.calendar_sync_next_due_in = Some(remaining);
                ctx.request_repaint_after(remaining.min(StdDuration::from_secs(1)));
                return;
            }
        }

        let scheduler = self.calendar_sync_scheduler.clone();
        let db_path = self.context.database().path().to_string();
        let (tx, rx) = mpsc::channel::<Result<SchedulerTickResult, String>>();
        self.calendar_sync_result_rx = Some(rx);
        self.calendar_sync_in_progress = true;
        self.calendar_sync_status_is_error = false;
        self.calendar_sync_status_message = Some("↻ Calendar sync running…".to_string());

        thread::spawn(move || {
            let result = (|| -> Result<SchedulerTickResult, String> {
                let db = Database::new(&db_path).map_err(|err| err.to_string())?;
                let mut scheduler = scheduler
                    .lock()
                    .map_err(|_| "Calendar sync scheduler lock poisoned".to_string())?;
                scheduler.tick(db.connection()).map_err(|err| err.to_string())
            })();

            let _ = tx.send(result);
        });

        ctx.request_repaint_after(StdDuration::from_millis(500));
    }

    fn apply_scheduler_result(&mut self, result: SchedulerTickResult, ctx: &egui::Context) {
        let wait = result.next_due_in.unwrap_or_else(|| StdDuration::from_secs(60));
        self.calendar_sync_next_due_in = Some(wait);
        self.calendar_sync_poll_due_at = Some(Instant::now() + wait);

        if result.attempted_count() > 0 {
            log::info!(
                "Scheduled calendar sync tick: attempted={}, successful={}, failed={}",
                result.attempted_count(),
                result.successful.len(),
                result.failed_sources.len()
            );

            if !result.failed_sources.is_empty() {
                self.calendar_sync_status_is_error = true;
                self.calendar_sync_status_message = Some(format!(
                    "⚠ Calendar sync: {} succeeded, {} failed",
                    result.successful.len(),
                    result.failed_sources.len()
                ));

                for (source_id, error) in &result.failed_sources {
                    log::warn!(
                        "Scheduled calendar sync failed for source {}: {}",
                        source_id,
                        error
                    );
                }
            } else {
                let created: usize = result.successful.iter().map(|item| item.created).sum();
                let updated: usize = result.successful.iter().map(|item| item.updated).sum();
                let deleted: usize = result.successful.iter().map(|item| item.deleted).sum();
                self.calendar_sync_status_is_error = false;
                self.calendar_sync_status_message = Some(format!(
                    "✓ Calendar sync: {} source(s) (+{} ~{} -{})",
                    result.successful.len(),
                    created,
                    updated,
                    deleted
                ));
            }

            ctx.request_repaint();
        }

        ctx.request_repaint_after(wait.min(StdDuration::from_secs(1)));
    }
}
