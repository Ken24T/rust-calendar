use super::countdown::OpenEventDialogRequest;
use super::CalendarApp;
use crate::services::countdown::RgbaColor;
use crate::services::event::EventService;
use crate::ui_egui::dialogs::backup_manager::render_backup_manager_dialog;
use crate::ui_egui::dialogs::search_dialog::{render_search_dialog, SearchDialogAction};
use crate::ui_egui::dialogs::theme_creator::{render_theme_creator, ThemeCreatorAction};
use crate::ui_egui::dialogs::theme_dialog::{render_theme_dialog, ThemeDialogAction};
use crate::ui_egui::event_dialog::{
    render_event_dialog, CountdownCardChanges, EventDialogResult, EventDialogState,
};
use crate::ui_egui::settings_dialog::render_settings_dialog;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::CountdownRequest;
use chrono::{Local, NaiveDateTime, TimeZone};

impl CalendarApp {
    pub(super) fn handle_dialogs(&mut self, ctx: &egui::Context) {
        self.capture_root_geometry(ctx);

        if self.show_event_dialog {
            self.ensure_event_dialog_state();
            self.render_event_dialog(ctx);
        }

        if self.show_settings_dialog {
            self.render_settings_dialog(ctx);
        }

        if self.state.show_search_dialog {
            self.render_search_dialog(ctx);
        }

        self.render_theme_dialog(ctx);
        self.render_theme_creator(ctx);

        let should_reload_db =
            render_backup_manager_dialog(ctx, &mut self.state.backup_manager_state);
        if should_reload_db {
            log::info!("Database restored. Application should be restarted.");
            // TODO: Implement graceful restart or reload mechanism
        }
    }

    fn ensure_event_dialog_state(&mut self) {
        if self.event_dialog_state.is_some() {
            return;
        }

        if let Some(event_id) = self.event_to_edit {
            let service = self.context.event_service();
            if let Ok(Some(event)) = service.get(event_id) {
                let mut state = EventDialogState::from_event(&event, &self.settings);
                
                // Auto-link countdown card if one exists for this event
                if let Some(card) = self.context.countdown_service().find_card_by_event_id(event_id) {
                    state.link_countdown_card(card.id, card.visuals.clone());
                }
                
                self.event_dialog_state = Some(state);
            } else {
                self.event_dialog_state = Some(EventDialogState::new_event(
                    self.event_dialog_date.unwrap_or(self.current_date),
                    &self.settings,
                ));
            }
        } else {
            self.event_dialog_state = Some(EventDialogState::new_event_with_time(
                self.event_dialog_date.unwrap_or(self.current_date),
                self.event_dialog_time,
                &self.settings,
            ));

            if let (Some(ref mut state), Some(ref rrule)) =
                (&mut self.event_dialog_state, &self.event_dialog_recurrence)
            {
                state.is_recurring = true;
                if rrule.contains("WEEKLY") {
                    state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Weekly;
                } else if rrule.contains("MONTHLY") {
                    state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Monthly;
                } else if rrule.contains("YEARLY") {
                    state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Yearly;
                } else {
                    state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Daily;
                }
            }
        }
    }

    pub(super) fn render_event_dialog(&mut self, ctx: &egui::Context) {
        if self.event_dialog_state.is_none() {
            self.show_event_dialog = false;
            return;
        }

        let (saved_event, card_changes, auto_create_card, was_new_event, event_saved) = {
            let state = self
                .event_dialog_state
                .as_mut()
                .expect("dialog state just checked");
            let EventDialogResult {
                saved_event,
                card_changes,
            } = render_event_dialog(
                ctx,
                state,
                self.context.database(),
                &self.settings,
                &mut self.show_event_dialog,
            );

            let auto_create_card = state.create_countdown 
                && state.event_id.is_none()
                && state.date > Local::now().date_naive();
            let was_new_event = state.event_id.is_none();
            let event_saved = saved_event.is_some();
            (
                saved_event,
                card_changes,
                auto_create_card,
                was_new_event,
                event_saved,
            )
        };

        // Apply card changes if any
        if let Some(changes) = card_changes {
            self.apply_countdown_card_changes(changes);
        }

        if let Some(event) = saved_event {
            if auto_create_card {
                self.consume_countdown_requests(vec![CountdownRequest::from_event(&event)]);
            }
            self.sync_cards_from_event(&event);

            if was_new_event {
                self.focus_on_event(&event);
            }
        }

        if event_saved || !self.show_event_dialog {
            self.event_dialog_state = None;
            self.event_dialog_time = None;
        }
    }

    /// Apply changes from the unified event dialog to the linked countdown card
    fn apply_countdown_card_changes(&mut self, changes: CountdownCardChanges) {
        let service = self.context.countdown_service_mut();

        // Update title override if different from event title
        service.set_title_override(changes.card_id, None); // Clear override, use event title

        // Update comment (synced with event description)
        service.set_comment(changes.card_id, changes.description);

        // Update start_at from date + time
        let naive_dt = NaiveDateTime::new(changes.start_date, changes.start_time);
        if let Some(start_at) = Local.from_local_datetime(&naive_dt).single() {
            service.set_start_at(changes.card_id, start_at);
        }

        // Update color if provided
        if let Some(color_hex) = changes.color {
            if let Some(rgba) = RgbaColor::from_hex_str(&color_hex) {
                service.set_title_bg_color(changes.card_id, rgba);
            }
        }

        // Update card-specific visuals
        service.set_always_on_top(changes.card_id, changes.always_on_top);
        service.set_compact_mode(changes.card_id, changes.compact_mode);
        service.set_title_font_size(changes.card_id, changes.title_font_size);
        service.set_days_font_size(changes.card_id, changes.days_font_size);

        log::info!(
            "Applied countdown card changes from event dialog for card {:?}",
            changes.card_id
        );
    }

    /// Open the event dialog for a countdown card, linking the card for bidirectional updates
    pub(super) fn open_event_dialog_for_card(&mut self, request: OpenEventDialogRequest) {
        // Load the event from the database
        let service = EventService::new(self.context.database().connection());
        let event = match service.get(request.event_id) {
            Ok(Some(event)) => event,
            Ok(None) => {
                log::warn!(
                    "Event {} not found for countdown card {:?}",
                    request.event_id,
                    request.card_id
                );
                return;
            }
            Err(e) => {
                log::error!("Failed to load event {}: {}", request.event_id, e);
                return;
            }
        };

        // Create event dialog state from the event
        let mut state = EventDialogState::from_event(&event, &self.settings);

        // Link the countdown card
        state.link_countdown_card(request.card_id, request.visuals);

        // Open the dialog
        self.event_dialog_state = Some(state);
        self.show_event_dialog = true;

        log::info!(
            "Opened event dialog for card {:?} (event {})",
            request.card_id,
            request.event_id
        );
    }

    pub(super) fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        let response = render_settings_dialog(
            ctx,
            &mut self.settings,
            self.context.database(),
            &mut self.show_settings_dialog,
        );

        if response.show_ribbon_changed || response.saved {
            self.show_ribbon = self.settings.show_ribbon;
        }

        if response.saved {
            self.apply_theme_from_db(ctx);
        }
    }

    pub(super) fn render_search_dialog(&mut self, ctx: &egui::Context) {
        let action = render_search_dialog(
            ctx,
            &mut self.state.search_dialog_state,
            self.context.database(),
            &self.active_theme,
            &mut self.state.show_search_dialog,
        );

        match action {
            SearchDialogAction::None => {}
            SearchDialogAction::NavigateToDate(date) => {
                self.current_date = date;
                self.state.show_search_dialog = false;
            }
            SearchDialogAction::EditEvent(event_id) => {
                self.event_to_edit = Some(event_id);
                self.show_event_dialog = true;
                self.state.show_search_dialog = false;
            }
            SearchDialogAction::Close => {
                self.state.show_search_dialog = false;
            }
        }
    }

    pub(super) fn render_theme_dialog(&mut self, ctx: &egui::Context) {
        let theme_service = self.context.theme_service();
        let available_themes = theme_service.list_themes().unwrap_or_default();

        // Cache custom theme colors for preview swatches
        for theme_name in &available_themes {
            if !CalendarTheme::is_builtin(theme_name) 
                && !self.state.theme_dialog_state.custom_theme_colors.contains_key(theme_name) 
            {
                if let Ok(theme) = theme_service.get_theme(theme_name) {
                    self.state.theme_dialog_state.cache_theme_colors(theme_name, theme.preview_colors());
                }
            }
        }

        let action = render_theme_dialog(
            ctx,
            &mut self.state.theme_dialog_state,
            &available_themes,
            &self.settings.theme,
        );

        match action {
            ThemeDialogAction::None => {}
            ThemeDialogAction::CreateTheme => {
                let base_theme = theme_service
                    .get_theme(&self.settings.theme)
                    .unwrap_or_else(|_| CalendarTheme::light());
                self.state.theme_creator_state.open_create(base_theme);
            }
            ThemeDialogAction::EditTheme(name) => {
                if let Ok(theme) = theme_service.get_theme(&name) {
                    self.state.theme_creator_state.open_edit(name, theme);
                }
            }
            ThemeDialogAction::DeleteTheme(name) => {
                if let Err(e) = theme_service.delete_theme(&name) {
                    eprintln!("Failed to delete theme: {}", e);
                } else {
                    log::info!("Successfully deleted theme: {}", name);
                    // Clear cached colors
                    self.state.theme_dialog_state.custom_theme_colors.remove(&name);
                    // If we deleted the current theme, switch to Light
                    if self.settings.theme.eq_ignore_ascii_case(&name) {
                        self.settings.theme = "Light".to_string();
                        let theme = CalendarTheme::light();
                        theme.apply_to_context(ctx);
                        self.active_theme = theme;
                        let settings_service = self.context.settings_service();
                        let _ = settings_service.update(&self.settings);
                    }
                }
            }
            ThemeDialogAction::ApplyTheme(name) => {
                self.settings.theme = name.clone();
                self.state.theme_dialog_state.preview_theme = None;

                if let Ok(theme) = theme_service.get_theme(&name) {
                    theme.apply_to_context(ctx);
                    self.active_theme = theme;
                } else {
                    let fallback = Self::fallback_theme_for_settings(&self.settings);
                    fallback.apply_to_context(ctx);
                    self.active_theme = fallback;
                }

                let settings_service = self.context.settings_service();
                if let Err(e) = settings_service.update(&self.settings) {
                    eprintln!("Failed to save theme setting: {}", e);
                }
            }
            ThemeDialogAction::PreviewTheme(name) => {
                // Temporarily apply theme for preview (don't save)
                if let Ok(theme) = theme_service.get_theme(&name) {
                    theme.apply_to_context(ctx);
                    self.state.theme_dialog_state.preview_theme = Some(name);
                }
            }
            ThemeDialogAction::RevertPreview => {
                // Revert to the original theme
                if let Some(original) = &self.state.theme_dialog_state.original_theme {
                    if let Ok(theme) = theme_service.get_theme(original) {
                        theme.apply_to_context(ctx);
                    }
                }
                self.state.theme_dialog_state.preview_theme = None;
            }
            ThemeDialogAction::DuplicateTheme { source, new_name } => {
                if let Err(e) = theme_service.duplicate_theme(&source, &new_name) {
                    eprintln!("Failed to duplicate theme: {}", e);
                } else {
                    log::info!("Successfully duplicated theme '{}' to '{}'", source, new_name);
                    // Cache colors for the new theme
                    if let Ok(theme) = theme_service.get_theme(&new_name) {
                        self.state.theme_dialog_state.cache_theme_colors(&new_name, theme.preview_colors());
                    }
                }
            }
            ThemeDialogAction::ExportTheme(name) => {
                // Use native file dialog to save
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Export Theme")
                    .set_file_name(&format!("{}.toml", name))
                    .add_filter("TOML files", &["toml"])
                    .save_file()
                {
                    if let Err(e) = theme_service.export_theme(&name, &path) {
                        eprintln!("Failed to export theme: {}", e);
                    } else {
                        log::info!("Successfully exported theme to {:?}", path);
                    }
                }
            }
            ThemeDialogAction::ImportTheme => {
                // Use native file dialog to open
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Import Theme")
                    .add_filter("TOML files", &["toml"])
                    .pick_file()
                {
                    match theme_service.import_theme(&path) {
                        Ok(name) => {
                            log::info!("Successfully imported theme: {}", name);
                            // Cache colors for the imported theme
                            if let Ok(theme) = theme_service.get_theme(&name) {
                                self.state.theme_dialog_state.cache_theme_colors(&name, theme.preview_colors());
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to import theme: {}", e);
                        }
                    }
                }
            }
            ThemeDialogAction::Close => {
                self.state.theme_dialog_state.close();
            }
        }
    }

    pub(super) fn render_theme_creator(&mut self, ctx: &egui::Context) {
        let action = render_theme_creator(ctx, &mut self.state.theme_creator_state);

        match action {
            ThemeCreatorAction::None => {}
            ThemeCreatorAction::Save(name, theme) => {
                let theme_service = self.context.theme_service();
                if let Err(e) = theme_service.save_theme(&theme, &name) {
                    eprintln!("Failed to save theme: {}", e);
                    self.state.theme_creator_state.validation_error =
                        Some(format!("Failed to save: {}", e));
                    self.state.theme_creator_state.is_open = true;
                } else {
                    eprintln!("Successfully saved theme: {}", name);

                    self.settings.theme = name.clone();
                    theme.apply_to_context(ctx);
                    self.active_theme = theme.clone();

                    let settings_service = self.context.settings_service();
                    if let Err(e) = settings_service.update(&self.settings) {
                        eprintln!("Failed to save settings: {}", e);
                    }

                    self.state.theme_creator_state.close();
                }
            }
            ThemeCreatorAction::Cancel => {
                self.state.theme_creator_state.close();
            }
        }
    }
}
