use super::CalendarApp;
use crate::services::calendar_sync::mapping::EventSyncMapService;

impl CalendarApp {
    pub(super) fn is_read_only_synced_event_id(&self, event_id: i64) -> bool {
        let service = EventSyncMapService::new(self.context.database().connection());
        service
            .is_read_only_synced_local_event(event_id)
            .unwrap_or_else(|err| {
                log::warn!(
                    "Failed to check read-only synced status for event {}: {}",
                    event_id,
                    err
                );
                false
            })
    }

    pub(super) fn notify_synced_event_read_only(&mut self) {
        self.toast_manager.warning(
            "Synced events are read-only in Rust Calendar. Edit or delete them in Google Calendar, then re-sync. You can still create countdown cards from event menus or Search.",
        );
    }

    pub(super) fn notify_synced_event_read_only_for(&mut self, event_id: i64) {
        if let Some(source_name) = self.synced_event_source_name(event_id) {
            self.toast_manager.warning(format!(
                "Synced event from '{}' is read-only in Rust Calendar. Edit or delete it in Google Calendar, then re-sync. You can still create countdown cards.",
                source_name
            ));
            return;
        }

        self.notify_synced_event_read_only();
    }

    pub(super) fn synced_event_source_name(&self, event_id: i64) -> Option<String> {
        let service = EventSyncMapService::new(self.context.database().connection());
        match service.get_source_name_for_local_event(event_id) {
            Ok(name) => name,
            Err(err) => {
                log::warn!(
                    "Failed to resolve synced source name for event {}: {}",
                    event_id,
                    err
                );
                None
            }
        }
    }
}
