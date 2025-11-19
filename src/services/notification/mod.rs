use anyhow::Result;
use notify_rust::{Notification, Timeout};

/// Service for displaying system notifications
pub struct NotificationService {
    enabled: bool,
}

impl NotificationService {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Check if notifications are enabled
    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable notifications
    #[allow(dead_code)]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Show a countdown alert notification for an event approaching or starting
    pub fn show_countdown_alert(
        &self,
        event_title: &str,
        message: &str,
        urgency: NotificationUrgency,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let timeout = match urgency {
            NotificationUrgency::Normal => Timeout::Milliseconds(5000),
            NotificationUrgency::Critical => Timeout::Milliseconds(10000),
        };

        Notification::new()
            .summary(event_title)
            .body(message)
            .timeout(timeout)
            .show()
            .map_err(|e| anyhow::anyhow!("Failed to show notification: {}", e))?;

        Ok(())
    }

    /// Show a simple notification with a title and body
    #[allow(dead_code)]
    pub fn show_simple(&self, title: &str, body: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary(title)
            .body(body)
            .timeout(Timeout::Milliseconds(5000))
            .show()
            .map_err(|e| anyhow::anyhow!("Failed to show notification: {}", e))?;

        Ok(())
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

/// Notification urgency level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationUrgency {
    Normal,
    Critical,
}
