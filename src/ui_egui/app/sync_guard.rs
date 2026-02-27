use super::CalendarApp;
use crate::services::calendar_sync::mapping::EventSyncMapService;

impl CalendarApp {
    pub(super) fn is_synced_event_id(&self, event_id: i64) -> bool {
        let service = EventSyncMapService::new(self.context.database().connection());
        service.is_synced_local_event(event_id).unwrap_or_else(|err| {
            log::warn!("Failed to check synced status for event {}: {}", event_id, err);
            false
        })
    }

    pub(super) fn notify_synced_event_read_only(&mut self) {
        self.toast_manager
            .warning("Synced events are read-only and cannot be edited or deleted");
    }
}
