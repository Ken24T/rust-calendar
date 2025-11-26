use std::path::{Path, PathBuf};

use crate::services::countdown::CountdownService;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::services::notification::NotificationService;
use crate::services::settings::SettingsService;
use crate::services::theme::ThemeService;

/// Shared access point for services and resources that multiple app modules need.
pub struct AppContext {
    database: &'static Database,
    /// Path for legacy JSON countdown storage. Kept for migration from JSON to database.
    #[allow(dead_code)]
    countdown_storage_path: PathBuf,
    countdown_service: CountdownService,
    notification_service: NotificationService,
}

impl AppContext {
    pub fn new(
        database: &'static Database,
        countdown_service: CountdownService,
        countdown_storage_path: PathBuf,
        notification_service: NotificationService,
    ) -> Self {
        Self {
            database,
            countdown_storage_path,
            countdown_service,
            notification_service,
        }
    }

    pub fn database(&self) -> &'static Database {
        self.database
    }

    pub fn countdown_service(&self) -> &CountdownService {
        &self.countdown_service
    }

    pub fn countdown_service_mut(&mut self) -> &mut CountdownService {
        &mut self.countdown_service
    }

    /// Path for legacy JSON countdown storage. Kept for potential future use.
    #[allow(dead_code)]
    pub fn countdown_storage_path(&self) -> &Path {
        &self.countdown_storage_path
    }

    pub fn notification_service_mut(&mut self) -> &mut NotificationService {
        &mut self.notification_service
    }

    pub fn settings_service(&self) -> SettingsService<'_> {
        SettingsService::new(self.database)
    }

    pub fn theme_service(&self) -> ThemeService<'_> {
        ThemeService::new(self.database)
    }

    pub fn event_service(&self) -> EventService<'_> {
        EventService::new(self.database.connection())
    }
}
