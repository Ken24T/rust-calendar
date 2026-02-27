use std::time::Duration as StdDuration;

use super::CalendarApp;

impl CalendarApp {
    pub(super) fn run_calendar_sync_scheduler(&mut self, ctx: &egui::Context) {
        let scheduler_result = self
            .calendar_sync_scheduler
            .tick(self.context.database().connection());

        match scheduler_result {
            Ok(result) => {
                if result.attempted_count() > 0 {
                    log::info!(
                        "Scheduled calendar sync tick: attempted={}, successful={}, failed={}",
                        result.attempted_count(),
                        result.successful.len(),
                        result.failed_sources.len()
                    );

                    if !result.failed_sources.is_empty() {
                        for (source_id, error) in &result.failed_sources {
                            log::warn!(
                                "Scheduled calendar sync failed for source {}: {}",
                                source_id,
                                error
                            );
                        }
                    }

                    ctx.request_repaint();
                }

                let wait = result
                    .next_due_in
                    .unwrap_or_else(|| StdDuration::from_secs(60));
                ctx.request_repaint_after(wait.min(StdDuration::from_secs(60)));
            }
            Err(err) => {
                log::error!("Scheduled calendar sync tick failed: {}", err);
                ctx.request_repaint_after(StdDuration::from_secs(60));
            }
        }
    }
}
