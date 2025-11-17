mod render;
mod settings;
mod state;

pub(super) use state::CountdownUiState;

use super::CalendarApp;
use crate::services::countdown::RgbaColor;
use crate::ui_egui::views::CountdownRequest;
use chrono::Local;
use directories::ProjectDirs;
use std::path::PathBuf;

impl CalendarApp {
    pub(super) fn resolve_countdown_storage_path() -> PathBuf {
        if let Some(dirs) = ProjectDirs::from("com", "RustCalendar", "CalendarApp") {
            let dir = dirs.data_dir();
            std::fs::create_dir_all(dir).ok();
            dir.join("countdowns.json")
        } else {
            log::warn!("Unable to resolve project directory; using current dir for countdowns");
            PathBuf::from("countdowns.json")
        }
    }

    pub(super) fn persist_countdowns_if_needed(&mut self) {
        if !self.countdown_service.is_dirty() {
            return;
        }

        if let Err(err) = self
            .countdown_service
            .save_to_disk(&self.countdown_storage_path)
        {
            log::error!("Failed to persist countdown cards: {err:?}");
        } else {
            self.countdown_service.mark_clean();
        }
    }

    pub(super) fn consume_countdown_requests(&mut self, requests: Vec<CountdownRequest>) {
        let now = Local::now();
        for request in requests {
            let CountdownRequest {
                event_id,
                title,
                start_at,
                end_at,
                color,
                body,
            } = request;

            let target_at = if start_at > now {
                start_at
            } else if end_at > now {
                end_at
            } else {
                log::info!(
                    "Skipping countdown for finished event {:?} ({}): start {:?}, end {:?}",
                    event_id,
                    title,
                    start_at,
                    end_at
                );
                continue;
            };

            let event_color = color.as_deref().and_then(RgbaColor::from_hex_str);
            let event_body = body.and_then(|text| {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            });

            let card_id = self.countdown_service.create_card(
                event_id,
                title,
                target_at,
                event_color,
                event_body,
                self.settings.default_card_width,
                self.settings.default_card_height,
            );
            self.countdown_ui.mark_card_pending(card_id);
            log::info!("created countdown card {:?}", card_id);
        }
    }
}
