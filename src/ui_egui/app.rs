#[path = "app/context.rs"]
mod context;
mod countdown;
mod lifecycle;
mod state;
mod views;

use self::context::AppContext;
use self::countdown::CountdownUiState;
use self::state::{AppState, ViewType};
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::countdown::{CountdownCardGeometry, CountdownService, CountdownWarningState};
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::dialogs::theme_creator::{render_theme_creator, ThemeCreatorAction};
use crate::ui_egui::dialogs::theme_dialog::{render_theme_dialog, ThemeDialogAction};
use crate::ui_egui::event_dialog::{render_event_dialog, EventDialogResult, EventDialogState};
use crate::ui_egui::settings_dialog::render_settings_dialog;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::week_view::WeekView;
use crate::ui_egui::views::workweek_view::WorkWeekView;
use crate::ui_egui::views::{AutoFocusRequest, CountdownRequest};
use chrono::{Local, NaiveDate};

const MIN_ROOT_WIDTH: f32 = 320.0;
const MIN_ROOT_HEIGHT: f32 = 220.0;

pub struct CalendarApp {
    /// Shared access to leaked database and supporting services
    context: AppContext,
    /// Core application settings/stateful data
    settings: Settings,
    current_view: ViewType,
    current_date: NaiveDate,
    show_event_dialog: bool,
    show_settings_dialog: bool,
    /// Ribbon toggle mirrors persisted settings
    show_ribbon: bool,
    /// Currently applied theme colors
    active_theme: CalendarTheme,
    /// Event dialog state management
    event_dialog_state: Option<EventDialogState>,
    event_dialog_date: Option<NaiveDate>,
    event_dialog_time: Option<chrono::NaiveTime>,
    event_dialog_recurrence: Option<String>,
    event_to_edit: Option<i64>,
    pending_focus: Option<AutoFocusRequest>,
    /// Countdown window and dialogs
    countdown_ui: CountdownUiState,
    /// Aggregated dialog/control state
    state: AppState,
}

impl eframe::App for CalendarApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_update(ctx, frame);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.handle_exit(gl);
    }
}

impl CalendarApp {
    /// Generate notification message and urgency based on countdown warning state
    fn notification_message_for_state(
        state: CountdownWarningState,
        event_time: chrono::DateTime<Local>,
        now: chrono::DateTime<Local>,
    ) -> (String, crate::services::notification::NotificationUrgency) {
        use crate::services::notification::NotificationUrgency;

        let remaining = event_time.signed_duration_since(now);

        match state {
            CountdownWarningState::Critical => {
                let minutes = remaining.num_minutes();
                let message = if minutes > 0 {
                    format!(
                        "Starting in {} minute{}",
                        minutes,
                        if minutes == 1 { "" } else { "s" }
                    )
                } else {
                    "Starting very soon!".to_string()
                };
                (message, NotificationUrgency::Critical)
            }
            CountdownWarningState::Imminent => {
                let hours = remaining.num_hours();
                let minutes = remaining.num_minutes() % 60;
                let message = if hours > 0 {
                    format!(
                        "Starting in {} hour{} {} minute{}",
                        hours,
                        if hours == 1 { "" } else { "s" },
                        minutes,
                        if minutes == 1 { "" } else { "s" }
                    )
                } else {
                    format!(
                        "Starting in {} minute{}",
                        minutes,
                        if minutes == 1 { "" } else { "s" }
                    )
                };
                (message, NotificationUrgency::Critical)
            }
            CountdownWarningState::Approaching => {
                let hours = remaining.num_hours();
                let message = format!(
                    "Starting in {} hour{}",
                    hours,
                    if hours == 1 { "" } else { "s" }
                );
                (message, NotificationUrgency::Normal)
            }
            CountdownWarningState::Starting => (
                "Event is starting now!".to_string(),
                NotificationUrgency::Critical,
            ),
            CountdownWarningState::Normal => {
                ("Event approaching".to_string(), NotificationUrgency::Normal)
            }
        }
    }

    fn navigate_previous(&mut self) {
        use chrono::Datelike;

        self.current_date = match self.current_view {
            ViewType::Day => self.current_date - chrono::Duration::days(1),
            ViewType::Week | ViewType::WorkWeek => self.current_date - chrono::Duration::weeks(1),
            ViewType::Month => {
                let prev_month = if self.current_date.month() == 1 {
                    12
                } else {
                    self.current_date.month() - 1
                };
                let year = if self.current_date.month() == 1 {
                    self.current_date.year() - 1
                } else {
                    self.current_date.year()
                };
                NaiveDate::from_ymd_opt(year, prev_month, 1).unwrap()
            }
        };
    }

    fn navigate_next(&mut self) {
        use chrono::Datelike;

        self.current_date = match self.current_view {
            ViewType::Day => self.current_date + chrono::Duration::days(1),
            ViewType::Week | ViewType::WorkWeek => self.current_date + chrono::Duration::weeks(1),
            ViewType::Month => {
                let next_month = if self.current_date.month() == 12 {
                    1
                } else {
                    self.current_date.month() + 1
                };
                let year = if self.current_date.month() == 12 {
                    self.current_date.year() + 1
                } else {
                    self.current_date.year()
                };
                NaiveDate::from_ymd_opt(year, next_month, 1).unwrap()
            }
        };
    }

    fn handle_ics_import(&mut self, events: Vec<Event>, source_label: &str) {
        if events.is_empty() {
            log::info!("No events found in {} import", source_label);
            return;
        }

        let mut existing_events = self
            .context
            .event_service()
            .list_all()
            .unwrap_or_else(|err| {
                log::error!(
                    "Failed to list existing events before {} import: {}",
                    source_label,
                    err
                );
                Vec::new()
            });

        if self.settings.edit_before_import {
            let first_event = events[0].clone();
            let remaining = events.len().saturating_sub(1);

            if Self::is_duplicate_event(&existing_events, &first_event) {
                log::info!(
                    "Skipping duplicate event (edit mode) from {}: '{}'",
                    source_label,
                    first_event.title
                );
            } else {
                match self.context.event_service().create(first_event.clone()) {
                    Ok(created_event) => {
                        self.focus_on_event(&created_event);
                        if let Some(event_id) = created_event.id {
                            self.event_to_edit = Some(event_id);
                            self.show_event_dialog = true;
                            log::info!(
                                "Opening event '{}' for editing from {}",
                                created_event.title,
                                source_label
                            );
                        }
                        existing_events.push(created_event);
                    }
                    Err(err) => {
                        log::error!(
                            "Failed to create event for editing from {}: {}",
                            source_label,
                            err
                        );
                    }
                }
            }

            if remaining > 0 {
                log::info!(
                    "Note: Only the first event was opened for editing from {}. {} other event(s) were not imported.",
                    source_label,
                    remaining
                );
            }

            return;
        }

        let mut imported_count = 0;
        let mut failed_count = 0;
        let mut duplicate_count = 0;

        for event in events {
            let event_title = event.title.clone();

            if Self::is_duplicate_event(&existing_events, &event) {
                log::info!(
                    "Skipping duplicate event from {}: '{}'",
                    source_label,
                    event_title
                );
                duplicate_count += 1;
                continue;
            }

            match self.context.event_service().create(event) {
                Ok(created_event) => {
                    self.focus_on_event(&created_event);
                    imported_count += 1;

                    if self.settings.auto_create_countdown_on_import
                        && created_event.start > Local::now()
                    {
                        if let Some(event_id) = created_event.id {
                            use crate::services::countdown::RgbaColor;

                            let event_color = created_event.color.as_ref().and_then(|hex| {
                                if hex.starts_with('#') && hex.len() == 7 {
                                    u32::from_str_radix(&hex[1..], 16).ok().map(|rgb| {
                                        let r = ((rgb >> 16) & 0xFF) as u8;
                                        let g = ((rgb >> 8) & 0xFF) as u8;
                                        let b = (rgb & 0xFF) as u8;
                                        RgbaColor::new(r, g, b, 255)
                                    })
                                } else {
                                    None
                                }
                            });

                            let location_label = created_event
                                .location
                                .as_deref()
                                .map(str::trim)
                                .filter(|loc| !loc.is_empty())
                                .map(|loc| loc.to_string());

                            let card_id = self.context.countdown_service_mut().create_card(
                                Some(event_id),
                                created_event.title.clone(),
                                created_event.start,
                                event_color,
                                created_event.description.clone(),
                                self.settings.default_card_width,
                                self.settings.default_card_height,
                            );

                            if let Some(label) = location_label {
                                self.context
                                    .countdown_service_mut()
                                    .set_auto_title_override(card_id, Some(label));
                            }
                        }
                    }

                    existing_events.push(created_event.clone());
                }
                Err(err) => {
                    log::error!(
                        "Failed to import event '{}' from {}: {}",
                        event_title,
                        source_label,
                        err
                    );
                    failed_count += 1;
                }
            }
        }

        if duplicate_count > 0 {
            log::info!(
                "{} import complete: {} events imported, {} duplicates skipped, {} failed",
                source_label,
                imported_count,
                duplicate_count,
                failed_count
            );
        } else {
            log::info!(
                "{} import complete: {} events imported, {} failed",
                source_label,
                imported_count,
                failed_count
            );
        }
    }

    fn is_duplicate_event(existing_events: &[Event], candidate: &Event) -> bool {
        existing_events.iter().any(|event| {
            event.title == candidate.title
                && event.start == candidate.start
                && event.end == candidate.end
        })
    }

    // Placeholder dialog renderers
    fn render_event_dialog(&mut self, ctx: &egui::Context) {
        if self.event_dialog_state.is_none() {
            // No state - shouldn't happen, but close dialog if it does
            self.show_event_dialog = false;
            return;
        }

        let (saved_event, auto_create_card, was_new_event, event_saved) = {
            let state = self
                .event_dialog_state
                .as_mut()
                .expect("dialog state just checked");
            let EventDialogResult { saved_event } = render_event_dialog(
                ctx,
                state,
                self.context.database(),
                &self.settings,
                &mut self.show_event_dialog,
            );

            let auto_create_card = state.create_countdown && state.event_id.is_none();
            let was_new_event = state.event_id.is_none();
            let event_saved = saved_event.is_some();
            (saved_event, auto_create_card, was_new_event, event_saved)
        };

        if let Some(event) = saved_event {
            if auto_create_card {
                self.consume_countdown_requests(vec![CountdownRequest::from_event(&event)]);
            }
            self.sync_cards_from_event(&event);

            if was_new_event {
                self.focus_on_event(&event);
            }
        }

        // If saved, clear the dialog state
        if event_saved || !self.show_event_dialog {
            self.event_dialog_state = None;
            self.event_dialog_time = None;
        }
    }

    fn hydrate_countdown_titles_from_events(
        countdown_service: &mut CountdownService,
        database: &'static Database,
    ) {
        let mut seen_ids = std::collections::HashSet::new();
        let mut event_ids = Vec::new();

        for card in countdown_service.cards() {
            if let Some(event_id) = card.event_id {
                if seen_ids.insert(event_id) {
                    event_ids.push(event_id);
                }
            }
        }

        if event_ids.is_empty() {
            return;
        }

        let event_service = EventService::new(database.connection());
        for event_id in event_ids {
            match event_service.get(event_id) {
                Ok(Some(event)) => {
                    let location_label = event
                        .location
                        .as_deref()
                        .map(str::trim)
                        .filter(|loc| !loc.is_empty())
                        .map(|loc| loc.to_string());

                    countdown_service.sync_title_for_event(event_id, event.title.clone());
                    countdown_service.sync_title_override_for_event(event_id, location_label);
                }
                Ok(None) => {
                    log::warn!(
                        "Countdown card references missing event id {} while syncing titles",
                        event_id
                    );
                }
                Err(err) => {
                    log::error!(
                        "Failed to load event {} while syncing countdown titles: {}",
                        event_id,
                        err
                    );
                }
            }
        }
    }

    fn flush_pending_event_bodies(&mut self) {
        let updates = self.countdown_ui.drain_pending_event_bodies();
        if updates.is_empty() {
            return;
        }

        for (event_id, body) in updates {
            match self.context.event_service().get(event_id) {
                Ok(Some(mut event)) => {
                    event.description = body.clone();
                    if let Err(err) = self.context.event_service().update(&event) {
                        log::error!(
                            "Failed to update event {} body from countdown settings: {err}",
                            event_id
                        );
                        continue;
                    }
                    self.context
                        .countdown_service_mut()
                        .sync_comment_for_event(event_id, body.clone());
                }
                Ok(None) => {
                    log::warn!(
                        "Countdown requested update for missing event id {}",
                        event_id
                    );
                }
                Err(err) => {
                    log::error!(
                        "Failed to load event {} for countdown body sync: {err}",
                        event_id
                    );
                }
            }
        }
    }

    fn sync_cards_from_event(&mut self, event: &Event) {
        if let Some(event_id) = event.id {
            let location_label = event
                .location
                .as_deref()
                .map(str::trim)
                .filter(|loc| !loc.is_empty())
                .map(|loc| loc.to_string());

            let countdown_service = self.context.countdown_service_mut();
            countdown_service.sync_title_for_event(event_id, event.title.clone());
            countdown_service.sync_title_override_for_event(event_id, location_label);
            countdown_service.sync_comment_for_event(event_id, event.description.clone());
        }
    }

    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        let response = render_settings_dialog(
            ctx,
            &mut self.settings,
            self.context.database(),
            &mut self.show_settings_dialog,
        );

        if response.show_ribbon_changed || response.saved {
            self.show_ribbon = self.settings.show_ribbon;
        }

        // If settings were saved, apply theme
        if response.saved {
            self.apply_theme_from_db(ctx);
        }
    }

    fn render_theme_dialog(&mut self, ctx: &egui::Context) {
        // Get available themes from database
        let theme_service = self.context.theme_service();
        let available_themes = theme_service.list_themes().unwrap_or_default();

        let action = render_theme_dialog(
            ctx,
            &mut self.state.theme_dialog_state,
            &available_themes,
            &self.settings.theme,
        );

        match action {
            ThemeDialogAction::None => {}
            ThemeDialogAction::CreateTheme => {
                // Open theme creator with current theme as base
                let base_theme = theme_service
                    .get_theme(&self.settings.theme)
                    .unwrap_or_else(|_| CalendarTheme::light());
                self.state.theme_creator_state.open_create(base_theme);
            }
            ThemeDialogAction::EditTheme(name) => {
                // Load and edit the theme
                if let Ok(theme) = theme_service.get_theme(&name) {
                    self.state.theme_creator_state.open_edit(name, theme);
                }
            }
            ThemeDialogAction::DeleteTheme(name) => {
                // Delete the theme
                if let Err(e) = theme_service.delete_theme(&name) {
                    eprintln!("Failed to delete theme: {}", e);
                } else {
                    eprintln!("Successfully deleted theme: {}", name);
                }
            }
            ThemeDialogAction::ApplyTheme(name) => {
                // Apply the selected theme
                self.settings.theme = name.clone();

                // Apply the custom theme or built-in theme
                if let Ok(theme) = theme_service.get_theme(&name) {
                    theme.apply_to_context(ctx);
                    self.active_theme = theme;
                } else {
                    let fallback = Self::fallback_theme_for_settings(&self.settings);
                    fallback.apply_to_context(ctx);
                    self.active_theme = fallback;
                }

                // Save to database
                let settings_service = self.context.settings_service();
                if let Err(e) = settings_service.update(&self.settings) {
                    eprintln!("Failed to save theme setting: {}", e);
                }
            }
            ThemeDialogAction::Close => {
                self.state.theme_dialog_state.close();
            }
        }
    }

    fn render_theme_creator(&mut self, ctx: &egui::Context) {
        let action = render_theme_creator(ctx, &mut self.state.theme_creator_state);

        match action {
            ThemeCreatorAction::None => {}
            ThemeCreatorAction::Save(name, theme) => {
                // Save the theme to database
                let theme_service = self.context.theme_service();
                if let Err(e) = theme_service.save_theme(&theme, &name) {
                    eprintln!("Failed to save theme: {}", e);
                    self.state.theme_creator_state.validation_error =
                        Some(format!("Failed to save: {}", e));
                    self.state.theme_creator_state.is_open = true; // Reopen to show error
                } else {
                    eprintln!("Successfully saved theme: {}", name);

                    // Apply the new theme
                    self.settings.theme = name.clone();
                    theme.apply_to_context(ctx);
                    self.active_theme = theme.clone();

                    // Save settings
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

    fn apply_pending_root_geometry(&mut self, ctx: &egui::Context) {
        if let Some(geometry) = self.state.pending_root_geometry.take() {
            if !Self::is_plausible_root_geometry(&geometry) {
                log::warn!(
                    "Ignoring persisted root geometry due to implausible size: {:?}",
                    geometry
                );
                return;
            }
            log::debug!("Applying persisted root geometry: {:?}", geometry);
            if geometry.width > 40.0 && geometry.height > 40.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    geometry.x, geometry.y,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    geometry.width,
                    geometry.height,
                )));
            }
        }
    }

    fn capture_root_geometry(&mut self, ctx: &egui::Context) {
        if let Some(info) = viewport_info(ctx, egui::ViewportId::ROOT) {
            if let Some(geometry) = geometry_from_viewport_info(&info) {
                if !Self::is_plausible_root_geometry(&geometry) {
                    log::debug!(
                        "Skipping root geometry capture due to implausible size: {:?}",
                        geometry
                    );
                    return;
                }
                let needs_update = match self.context.countdown_service().app_window_geometry() {
                    Some(current) => geometry_changed(current, geometry),
                    None => true,
                };
                if needs_update {
                    log::debug!("Captured new root geometry: {:?}", geometry);
                    self.context
                        .countdown_service_mut()
                        .update_app_window_geometry(geometry);
                }
            }
        }
    }

    fn focus_on_event(&mut self, event: &Event) {
        self.current_date = event.start.date_naive();
        if matches!(
            self.current_view,
            ViewType::Day | ViewType::Week | ViewType::WorkWeek
        ) {
            self.pending_focus = Some(AutoFocusRequest::from_event(event));
        }
    }

    fn focus_on_current_time_if_visible(&mut self) {
        if !matches!(
            self.current_view,
            ViewType::Day | ViewType::Week | ViewType::WorkWeek
        ) {
            return;
        }

        let now = Local::now();
        let today = now.date_naive();

        let should_focus = match self.current_view {
            ViewType::Day => self.current_date == today,
            ViewType::Week => {
                let week_start =
                    WeekView::get_week_start(self.current_date, self.settings.first_day_of_week);
                let week_end = week_start + chrono::Duration::days(6);
                today >= week_start && today <= week_end
            }
            ViewType::WorkWeek => {
                let week_start = WorkWeekView::get_week_start(
                    self.current_date,
                    self.settings.first_day_of_week,
                );
                let work_week_dates = WorkWeekView::get_work_week_dates(week_start, &self.settings);
                work_week_dates.contains(&today)
            }
            ViewType::Month => false,
        };

        if should_focus {
            self.pending_focus = Some(AutoFocusRequest {
                date: today,
                time: Some(now.time()),
            });
        }
    }

    fn is_plausible_root_geometry(geometry: &CountdownCardGeometry) -> bool {
        geometry.width >= MIN_ROOT_WIDTH && geometry.height >= MIN_ROOT_HEIGHT
    }
}

fn viewport_info(ctx: &egui::Context, viewport_id: egui::ViewportId) -> Option<egui::ViewportInfo> {
    ctx.input(|input| input.raw.viewports.get(&viewport_id).cloned())
}

fn geometry_from_viewport_info(info: &egui::ViewportInfo) -> Option<CountdownCardGeometry> {
    let inner = match info.inner_rect {
        Some(rect) => rect,
        None => return None,
    };
    let (outer_left, outer_top) = info
        .outer_rect
        .map(|outer| (outer.left(), outer.top()))
        .unwrap_or((inner.left(), inner.top()));

    Some(CountdownCardGeometry {
        x: outer_left,
        y: outer_top,
        width: inner.width(),
        height: inner.height(),
    })
}

fn geometry_changed(a: CountdownCardGeometry, b: CountdownCardGeometry) -> bool {
    (a.x - b.x).abs() > 2.0
        || (a.y - b.y).abs() > 2.0
        || (a.width - b.width).abs() > 1.0
        || (a.height - b.height).abs() > 1.0
}
