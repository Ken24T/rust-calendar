use chrono::{Datelike, Local, NaiveDate};
use egui::{Color32, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::palette::{CalendarCellPalette, DayStripPalette};
use super::week_shared::DeleteConfirmRequest;
use super::{filter_events_by_category, is_synced_event, load_synced_event_ids};
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::models::template::EventTemplate;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::services::template::TemplateService;
use crate::ui_egui::theme::CalendarTheme;

/// Width of the week number column
const WEEK_NUMBER_WIDTH: f32 = 35.0;

/// Result returned from month view
pub struct MonthViewResult {
    /// Action to perform
    pub action: MonthViewAction,
    /// Request to show delete confirmation dialog
    pub delete_confirm_request: Option<DeleteConfirmRequest>,
}

impl Default for MonthViewResult {
    fn default() -> Self {
        Self {
            action: MonthViewAction::None,
            delete_confirm_request: None,
        }
    }
}

/// Action returned from month view
pub enum MonthViewAction {
    /// No action
    None,
    /// Switch to day view for a specific date
    SwitchToDayView(NaiveDate),
    /// Switch to user's default view for a specific date
    SwitchToDefaultView(NaiveDate),
    /// Create event from template (template_id, date)
    CreateFromTemplate(i64, NaiveDate),
}

/// Blend header color for weekend columns (slightly darker/lighter)
fn blend_header_weekend(header_bg: Color32, is_dark: bool) -> Color32 {
    let factor = if is_dark { 1.15 } else { 0.92 };
    Color32::from_rgb(
        ((header_bg.r() as f32 * factor).min(255.0)) as u8,
        ((header_bg.g() as f32 * factor).min(255.0)) as u8,
        ((header_bg.b() as f32 * factor).min(255.0)) as u8,
    )
}

pub struct MonthView;

impl MonthView {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        theme: &CalendarTheme,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
        event_to_edit: &mut Option<i64>,
        category_filter: Option<&str>,
    ) -> MonthViewResult {
        let today = Local::now().date_naive();
        let mut result = MonthViewResult::default();

        // Get events for the month
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_month(&event_service, *current_date);
        let events = filter_events_by_category(events, category_filter);
        let synced_event_ids = load_synced_event_ids(database);

        // Day of week headers - use Grid to match column widths below
        let day_names = Self::get_day_names(settings.first_day_of_week);
        let spacing = 2.0;
        let show_week_numbers = settings.show_week_numbers;
        let week_col_extra = if show_week_numbers { WEEK_NUMBER_WIDTH + spacing } else { 0.0 };
        let total_spacing = spacing * 6.0; // 6 gaps between 7 columns
        let col_width = (ui.available_width() - total_spacing - week_col_extra) / 7.0;

        let day_strip_palette = DayStripPalette::from_theme(theme);
        egui::Grid::new("month_header_grid")
            .spacing([spacing, spacing])
            .show(ui, |ui| {
                // Week number header (empty)
                if show_week_numbers {
                    ui.allocate_ui_with_layout(
                        Vec2::new(WEEK_NUMBER_WIDTH, 30.0),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            egui::Frame::none()
                                .fill(day_strip_palette.header_bg)
                                .rounding(egui::Rounding::same(6.0))
                                .stroke(Stroke::new(1.0, day_strip_palette.strip_border))
                                .inner_margin(Margin::symmetric(4.0, 6.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Wk")
                                            .size(12.0)
                                            .color(day_strip_palette.header_text)
                                            .strong(),
                                    );
                                });
                        },
                    );
                }
                
                for (idx, day) in day_names.iter().enumerate() {
                    let weekday = (settings.first_day_of_week as usize + idx) % 7;
                    let is_weekend = weekday == 0 || weekday == 6;
                    let header_bg = if is_weekend {
                        // Slightly darker/lighter for weekend headers
                        blend_header_weekend(day_strip_palette.header_bg, theme.is_dark)
                    } else {
                        day_strip_palette.header_bg
                    };
                    let text_color = day_strip_palette.header_text;

                    ui.allocate_ui_with_layout(
                        Vec2::new(col_width, 30.0),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            egui::Frame::none()
                                .fill(header_bg)
                                .rounding(egui::Rounding::same(6.0))
                                .stroke(Stroke::new(1.0, day_strip_palette.strip_border))
                                .inner_margin(Margin::symmetric(8.0, 6.0))
                                .show(ui, |cell_ui| {
                                    cell_ui.centered_and_justified(|label_ui| {
                                        label_ui.label(
                                            egui::RichText::new(*day)
                                                .size(14.0)
                                                .color(text_color)
                                                .strong(),
                                        );
                                    });
                                });
                        },
                    );
                }
            });

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);

        // Calculate calendar grid
        let first_of_month = current_date.with_day(1).unwrap();
        let first_weekday = (first_of_month.weekday().num_days_from_sunday() as i32
            - settings.first_day_of_week as i32
            + 7)
            % 7;
        let days_in_month = Self::get_days_in_month(current_date.year(), current_date.month());

        // Calculate how many weeks are needed for this month
        // Total cells needed = days before month start + days in month
        let total_cells = first_weekday + days_in_month;
        let weeks_needed = (total_cells + 6) / 7; // Ceiling division

        // Build calendar grid (dynamic number of rows based on month)
        let mut day_counter = 1 - first_weekday;

        let palette = CalendarCellPalette::from_theme(theme);

        egui::Grid::new("month_grid")
            .spacing([spacing, spacing])
            .show(ui, |ui| {
                for _week_row in 0..weeks_needed {
                    // Week number column
                    if show_week_numbers {
                        // Calculate the date for this row (use middle of week for reliability)
                        let row_day = day_counter + 3; // Middle of week
                        let week_date = if row_day >= 1 && row_day <= days_in_month {
                            NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                row_day as u32,
                            )
                        } else if row_day < 1 {
                            // Previous month - use day 1
                            NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                1,
                            )
                        } else {
                            // Next month - use last day
                            NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                days_in_month as u32,
                            )
                        };
                        
                        if let Some(date) = week_date {
                            let week_num = date.iso_week().week();
                            ui.allocate_ui_with_layout(
                                Vec2::new(WEEK_NUMBER_WIDTH, 80.0),
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    egui::Frame::none()
                                        .fill(palette.empty_bg)
                                        .rounding(egui::Rounding::same(4.0))
                                        .inner_margin(Margin::symmetric(2.0, 4.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{}", week_num))
                                                    .size(11.0)
                                                    .color(palette.text.gamma_multiply(0.7)),
                                            );
                                        });
                                },
                            );
                        }
                    }

                    for _day_of_week in 0..7 {
                        if day_counter < 1 || day_counter > days_in_month {
                            // Empty cell for days outside current month
                            let (rect, _response) = ui.allocate_exact_size(
                                Vec2::new(col_width, 80.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 2.0, palette.empty_bg);
                        } else {
                            // Day cell
                            let date = NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                day_counter as u32,
                            )
                            .unwrap();

                            let is_today = date == today;

                            // Calculate weekend based on first_day_of_week
                            // If Sunday is first day (0), weekend is days 0 and 6
                            // If Monday is first day (1), weekend is days 5 and 6 (Sat, Sun)
                            let day_of_week = (date.weekday().num_days_from_sunday() as i32
                                - settings.first_day_of_week as i32
                                + 7)
                                % 7;
                            let is_weekend = day_of_week == 5 || day_of_week == 6;

                            // Get events for this day
                            let day_events: Vec<&Event> = events
                                .iter()
                                .filter(|e| {
                                    if e.all_day {
                                        let start_date = e.start.date_naive();
                                        let end_date = e.end.date_naive();
                                        date >= start_date && date <= end_date
                                    } else {
                                        e.start.date_naive() == date
                                    }
                                })
                                .collect();

                            let (cell_action, _clicked_event, delete_request) = Self::render_day_cell(
                                ui,
                                day_counter,
                                date,
                                is_today,
                                is_weekend,
                                &day_events,
                                &synced_event_ids,
                                database,
                                show_event_dialog,
                                event_dialog_date,
                                event_dialog_recurrence,
                                event_to_edit,
                                palette,
                                col_width,
                            );
                            
                            // Check if we need to switch views
                            if !matches!(cell_action, MonthViewAction::None) {
                                result.action = cell_action;
                            }
                            
                            // Check if there's a delete confirmation request
                            if delete_request.is_some() {
                                result.delete_confirm_request = delete_request;
                            }
                        }
                        day_counter += 1;
                    }
                    ui.end_row();
                }
            });
        
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn render_day_cell(
        ui: &mut egui::Ui,
        day: i32,
        date: NaiveDate,
        is_today: bool,
        is_weekend: bool,
        events: &[&Event],
        synced_event_ids: &std::collections::HashSet<i64>,
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
        event_to_edit: &mut Option<i64>,
        palette: CalendarCellPalette,
        col_width: f32,
    ) -> (MonthViewAction, Option<Event>, Option<DeleteConfirmRequest>) {
        let desired_size = Vec2::new(col_width, 80.0);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, Sense::click().union(Sense::hover()));

        // Background
        let bg_color = if is_today {
            palette.today_bg
        } else if is_weekend {
            palette.weekend_bg
        } else {
            palette.regular_bg
        };
        ui.painter().rect_filled(rect, 2.0, bg_color);

        // Border
        let border_color = if is_today {
            palette.today_border
        } else {
            palette.border
        };
        ui.painter()
            .rect_stroke(rect, 2.0, Stroke::new(1.0, border_color));

        // Hover emphasis with cursor change
        if response.hovered() {
            ui.painter()
                .rect_stroke(rect, 2.0, Stroke::new(2.0, palette.hover_border));
            ui.painter()
                .rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(100, 150, 200, 30));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Day number label - clickable to switch to day view
        let day_text = format!("{}", day);
        let text_color = if is_today {
            palette.today_text
        } else {
            palette.text
        };
        
        // Define clickable area for the day number (don't allocate, just check pointer)
        let day_number_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 2.0, rect.top() + 2.0),
            Vec2::new(30.0, 20.0),
        );
        
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        let day_number_hovered = pointer_pos.is_some_and(|pos| day_number_rect.contains(pos));
        let day_number_clicked = response.clicked() && day_number_hovered;
        
        // Underline on hover to indicate clickability
        if day_number_hovered {
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left() + 5.0, rect.top() + 18.0),
                    Pos2::new(rect.left() + 25.0, rect.top() + 18.0),
                ],
                Stroke::new(1.0, text_color),
            );
        }
        
        ui.painter().text(
            Pos2::new(rect.left() + 5.0, rect.top() + 5.0),
            egui::Align2::LEFT_TOP,
            &day_text,
            egui::FontId::proportional(14.0),
            text_color,
        );
        
        // Return action if day number clicked
        if day_number_clicked {
            return (MonthViewAction::SwitchToDayView(date), None, None);
        }

        let mut event_hitboxes: Vec<(Rect, Event)> = Vec::new();
        let mut y_offset = 24.0;
        let now = Local::now();
        
        for &event in events.iter().take(3) {
            let event_is_synced = is_synced_event(event.id, synced_event_ids);
            let is_past = event.end < now;
            
            let base_color = event
                .color
                .as_deref()
                .and_then(Self::parse_color)
                .unwrap_or(Color32::from_rgb(100, 150, 200));
            
            // Dim past events with stronger dimming for visibility (matching week view)
            let event_color = if is_past {
                Color32::from_rgba_unmultiplied(
                    (base_color.r() as f32 * 0.4) as u8,
                    (base_color.g() as f32 * 0.4) as u8,
                    (base_color.b() as f32 * 0.4) as u8,
                    140,
                )
            } else {
                base_color
            };

            // Event indicator bar
            let event_rect = Rect::from_min_size(
                Pos2::new(rect.left() + 3.0, rect.top() + y_offset),
                Vec2::new(rect.width() - 6.0, 16.0),
            );

            ui.painter().rect_filled(event_rect, 2.0, event_color);
            event_hitboxes.push((event_rect, event.clone()));

            // Dim text for past events (matching week view)
            let text_color = if is_past {
                Color32::from_rgba_unmultiplied(255, 255, 255, 150)
            } else {
                Color32::WHITE
            };

            // Build title text with location icon and category badge if present
            let location_icon = if event.location.as_ref().map(|l| !l.is_empty()).unwrap_or(false) {
                "📍"
            } else {
                ""
            };
            
            let title_text = if let Some(category) = &event.category {
                format!(
                    "{}{}{} [{}]",
                    if event_is_synced { "🔒 " } else { "" },
                    location_icon,
                    event.title,
                    category
                )
            } else {
                format!(
                    "{}{}{}",
                    if event_is_synced { "🔒 " } else { "" },
                    location_icon,
                    event.title
                )
            };

            // Event title with truncation
            let font_id = egui::FontId::proportional(11.0);
            let available_width = event_rect.width() - 6.0;
            let layout_job = egui::text::LayoutJob::simple(
                title_text,
                font_id.clone(),
                text_color,
                available_width,
            );
            let galley = ui.fonts(|f| f.layout_job(layout_job));
            ui.painter().galley(
                Pos2::new(
                    event_rect.left() + 3.0,
                    event_rect.center().y - galley.size().y / 2.0,
                ),
                galley,
                text_color,
            );

            y_offset += 18.0;
        }

        let pointer_pos = response.interact_pointer_pos()
            .or_else(|| ui.input(|i| i.pointer.hover_pos()));
        let pointer_hit = pointer_pos.and_then(|pos| {
            event_hitboxes
                .iter()
                .rev()
                .find(|(hit_rect, _)| hit_rect.contains(pos))
                .map(|(hit_rect, event)| (*hit_rect, event.clone()))
        });
        let pointer_event = pointer_hit.as_ref().map(|(_, event)| event.clone());
        let single_event_fallback = if event_hitboxes.len() == 1 {
            Some(event_hitboxes[0].1.clone())
        } else {
            None
        };

        // Show tooltip when hovering over an event and draw hover highlight
        if let Some((hit_rect, hovered_event)) = &pointer_hit {
            if response.hovered() && hit_rect.contains(pointer_pos.unwrap_or_default()) {
                // Draw subtle hover highlight on the event
                ui.painter().rect_stroke(
                    hit_rect.expand(1.0),
                    3.0,
                    Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 180)),
                );
                
                // Show pointer cursor
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                
                let tooltip_text = super::week_shared::format_event_tooltip(
                    hovered_event,
                    is_synced_event(hovered_event.id, synced_event_ids),
                );
                response.clone().on_hover_ui_at_pointer(|ui| {
                    ui.label(tooltip_text);
                });
            }
        } else if response.hovered() {
            // Show hint tooltip when hovering on empty space in day cell
            response.clone().on_hover_text("Click to view this day\nDouble-click to create event\nRight-click for more options");
        }

        // Show "+N more" if there are more events - make it clickable
        let mut more_clicked = false;
        if events.len() > 3 {
            let more_text = format!("+{} more", events.len() - 3);
            let more_rect = Rect::from_min_size(
                Pos2::new(rect.left() + 3.0, rect.top() + y_offset),
                Vec2::new(rect.width() - 6.0, 14.0),
            );
            
            // Check hover/click via pointer position (don't use allocate_rect as it breaks Grid)
            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
            let is_hovered = pointer_pos.is_some_and(|pos| more_rect.contains(pos));
            
            // Highlight on hover
            if is_hovered {
                ui.painter().rect_filled(
                    more_rect,
                    2.0,
                    Color32::from_rgba_unmultiplied(palette.today_border.r(), palette.today_border.g(), palette.today_border.b(), 50),
                );
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            
            // Draw text with underline on hover
            let text_color = if is_hovered {
                palette.text
            } else {
                Color32::GRAY
            };
            
            ui.painter().text(
                Pos2::new(rect.left() + 5.0, rect.top() + y_offset),
                egui::Align2::LEFT_TOP,
                &more_text,
                egui::FontId::proportional(10.0),
                text_color,
            );
            
            if is_hovered {
                // Underline on hover
                let text_width = ui.fonts(|f| {
                    f.glyph_width(&egui::FontId::proportional(10.0), ' ') * more_text.len() as f32 * 0.6
                });
                ui.painter().line_segment(
                    [
                        Pos2::new(rect.left() + 5.0, rect.top() + y_offset + 12.0),
                        Pos2::new(rect.left() + 5.0 + text_width, rect.top() + y_offset + 12.0),
                    ],
                    egui::Stroke::new(1.0, text_color),
                );
                
                // Show tooltip with hidden events using egui's popup system
                let tooltip_id = egui::Id::new("more_events_tooltip").with(date);
                egui::containers::popup::show_tooltip(ui.ctx(), egui::LayerId::new(egui::Order::Tooltip, tooltip_id), tooltip_id, |ui| {
                    ui.label(egui::RichText::new("Hidden events:").strong());
                    for event in events.iter().skip(3) {
                        let time_str = if event.all_day {
                            "All day".to_string()
                        } else {
                            event.start.format("%H:%M").to_string()
                        };
                        ui.label(format!("• {} - {}", time_str, event.title));
                    }
                    ui.label(egui::RichText::new("\nClick to view day").small().weak());
                });
            }
            
            // Check if clicked on "+X more"
            if response.clicked() && is_hovered {
                more_clicked = true;
            }
        }

        // Manual context menu handling
        let popup_id = response.id.with(format!("month_context_menu_{}", date));
        let mut popup_anchor_response = response.clone();
        popup_anchor_response.rect = Rect::from_min_size(
            Pos2::new(rect.left() + 5.0, rect.top()),
            Vec2::new(200.0, 30.0),
        );

        let mut context_menu_event: Option<Event> = None;
        let mut delete_confirm_request: Option<DeleteConfirmRequest> = None;
        
        // Check for pending delete request from previous frame
        let pending_delete_id = ui.ctx().memory_mut(|mem| {
            mem.data.remove_temp::<(i64, String)>(popup_id.with("pending_delete"))
        });
        if let Some((event_id, event_title)) = pending_delete_id {
            delete_confirm_request = Some(DeleteConfirmRequest {
                event_id,
                event_title,
                occurrence_only: false,
                occurrence_date: None,
            });
        }
        
        // Check for pending template selection from previous frame
        let pending_template = ui.ctx().memory_mut(|mem| {
            mem.data.remove_temp::<i64>(popup_id.with("pending_template"))
        });
        
        if response.secondary_clicked() {
            context_menu_event = pointer_event.clone();
            ui.memory_mut(|mem| mem.open_popup(popup_id));
        }
        
        // Load templates for context menu
        let templates: Vec<EventTemplate> = TemplateService::new(database.connection())
            .list_all()
            .unwrap_or_default();

        egui::popup::popup_above_or_below_widget(
            ui,
            popup_id,
            &popup_anchor_response,
            egui::AboveOrBelow::Below,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.set_width(190.0);

                let popup_event = context_menu_event
                    .clone()
                    .or_else(|| single_event_fallback.clone());

                if let Some(event) = popup_event {
                    let event_is_synced = is_synced_event(event.id, synced_event_ids);
                    ui.label(format!("Event: {}", event.title));
                    ui.separator();

                    if event_is_synced {
                        ui.label(
                            egui::RichText::new("🔒 Synced read-only event")
                                .italics()
                                .size(11.0),
                        );
                        ui.add_enabled(false, egui::Button::new("✏ Edit"));
                    } else if ui.button("✏ Edit").clicked() {
                        if let Some(id) = event.id {
                            *event_to_edit = Some(id);
                            *show_event_dialog = true;
                            *event_dialog_date = Some(date);
                        }
                        ui.memory_mut(|mem| mem.close_popup());
                    }

                    if event_is_synced {
                        ui.add_enabled(false, egui::Button::new("🗑 Delete"));
                    } else if ui.button("🗑 Delete").clicked() {
                        if let Some(id) = event.id {
                            // Store delete request in temp memory for next frame
                            ui.ctx().memory_mut(|mem| {
                                mem.data.insert_temp(popup_id.with("pending_delete"), (id, event.title.clone()));
                            });
                        }
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                } else {
                    ui.label("Create event");
                    ui.separator();

                    if ui.button("📅 New Event").clicked() {
                        *show_event_dialog = true;
                        *event_dialog_date = Some(date);
                        *event_dialog_recurrence = None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }

                    if ui.button("🔄 New Recurring Event").clicked() {
                        *show_event_dialog = true;
                        *event_dialog_date = Some(date);
                        *event_dialog_recurrence = Some("FREQ=MONTHLY".to_string());
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                    
                    // Template submenu
                    if !templates.is_empty() {
                        ui.separator();
                        ui.menu_button("📋 From Template", |ui| {
                            for template in &templates {
                                let label = template.name.to_string();
                                if ui.button(&label).on_hover_text(format!(
                                    "Create '{}' event\nDuration: {}",
                                    template.title,
                                    if template.all_day {
                                        "All day".to_string()
                                    } else {
                                        let h = template.duration_minutes / 60;
                                        let m = template.duration_minutes % 60;
                                        if h > 0 && m > 0 {
                                            format!("{}h {}m", h, m)
                                        } else if h > 0 {
                                            format!("{}h", h)
                                        } else {
                                            format!("{}m", m)
                                        }
                                    }
                                )).clicked() {
                                    if let Some(id) = template.id {
                                        ui.ctx().memory_mut(|mem| {
                                            mem.data.insert_temp(popup_id.with("pending_template"), id);
                                        });
                                    }
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                            }
                        });
                    }
                }
            },
        );
        
        // Handle pending template selection (return action)
        if let Some(template_id) = pending_template {
            return (MonthViewAction::CreateFromTemplate(template_id, date), None, delete_confirm_request);
        }

        // Double-click on event opens edit dialog, on empty space creates new event
        if response.double_clicked() {
            if let Some(event) = pointer_event.clone() {
                // Double-click on event - edit it
                if let Some(id) = event.id {
                    if is_synced_event(Some(id), synced_event_ids) {
                        return (MonthViewAction::None, Some(event), delete_confirm_request);
                    }
                    *show_event_dialog = true;
                    *event_to_edit = Some(id);
                    *event_dialog_date = Some(date);
                }
                return (MonthViewAction::None, Some(event), delete_confirm_request);
            } else {
                // Double-click on empty space - create new event for this date
                *show_event_dialog = true;
                *event_dialog_date = Some(date);
                *event_dialog_recurrence = None;
                return (MonthViewAction::None, None, delete_confirm_request);
            }
        }

        // Single left-click anywhere in day cell switches to default view for that date
        if response.clicked() {
            return (MonthViewAction::SwitchToDefaultView(date), None, delete_confirm_request);
        }
        
        // Handle "+X more" click to switch to day view
        if more_clicked {
            return (MonthViewAction::SwitchToDayView(date), None, delete_confirm_request);
        }
        
        (MonthViewAction::None, None, delete_confirm_request)
    }

    fn get_events_for_month(event_service: &EventService, date: NaiveDate) -> Vec<Event> {
        use chrono::{Local, TimeZone};

        let start_of_month = date.with_day(1).unwrap();
        let start = Local
            .from_local_datetime(&start_of_month.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();

        // Get last day of month
        let days_in_month = Self::get_days_in_month(date.year(), date.month());
        let end_of_month = date.with_day(days_in_month as u32).unwrap();
        let end = Local
            .from_local_datetime(&end_of_month.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();

        event_service
            .expand_recurring_events(start, end)
            .unwrap_or_default()
    }

    fn get_days_in_month(year: i32, month: u32) -> i32 {
        NaiveDate::from_ymd_opt(
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 },
            1,
        )
        .unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as i32
    }

    fn get_day_names(first_day_of_week: u8) -> Vec<&'static str> {
        let all_days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let start = first_day_of_week as usize;
        let mut result = Vec::with_capacity(7);
        for i in 0..7 {
            result.push(all_days[(start + i) % 7]);
        }
        result
    }

    fn parse_color(hex: &str) -> Option<Color32> {
        if hex.is_empty() {
            return None;
        }

        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some(Color32::from_rgb(r, g, b))
    }
}
