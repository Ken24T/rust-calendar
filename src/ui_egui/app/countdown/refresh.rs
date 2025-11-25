use super::super::CalendarApp;
use chrono::Local;

impl CalendarApp {
    /// Refresh countdown card days remaining and handle auto-dismiss.
    /// Returns true if any UI changes occurred that require a repaint.
    pub(in super::super) fn refresh_countdowns(&mut self, ctx: &egui::Context) {
        let now = Local::now();

        // Periodically refresh countdown cards even before their UI arrives.
        let changed_counts = self
            .context
            .countdown_service_mut()
            .refresh_days_remaining(now);
        if !changed_counts.is_empty() {
            ctx.request_repaint();
        }

        // Check for auto-dismiss
        let dismissed_cards = self.context.countdown_service_mut().check_auto_dismiss(now);
        if !dismissed_cards.is_empty() {
            log::info!("Auto-dismissed {} countdown card(s)", dismissed_cards.len());
            ctx.request_repaint();
        }
    }
}
