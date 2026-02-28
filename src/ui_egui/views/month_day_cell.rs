//! Day cell rendering for the month view.
//!
//! Extracted from `month_view.rs` — renders individual day cells including
//! event indicators, context menus, tooltips, and click handling.

use chrono::{Local, NaiveDate};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::month_context_menu;
use super::palette::CalendarCellPalette;
use super::week_shared::{parse_color, DeleteConfirmRequest};
use super::{is_synced_event, CountdownRequest};
use crate::models::event::Event;
use crate::services::database::Database;

use super::month_view::{MonthView, MonthViewAction};

impl MonthView {
    /// Truncate text to fit within a given pixel width, using binary search
    /// and appending "…" when truncation is needed.
    pub(super) fn truncate_single_line_to_width(
        ui: &egui::Ui,
        text: &str,
        font_id: &egui::FontId,
        color: Color32,
        max_width: f32,
    ) -> String {
        if max_width <= 0.0 {
            return String::new();
        }

        let measure_width = |candidate: &str| {
            let layout_job = egui::text::LayoutJob::simple(
                candidate.to_string(),
                font_id.clone(),
                color,
                f32::INFINITY,
            );
            ui.fonts(|f| f.layout_job(layout_job).size().x)
        };

        if measure_width(text) <= max_width {
            return text.to_string();
        }

        let ellipsis = "…";
        if measure_width(ellipsis) > max_width {
            return String::new();
        }

        let mut char_boundaries: Vec<usize> = text.char_indices().map(|(idx, _)| idx).collect();
        char_boundaries.push(text.len());

        let mut low = 0usize;
        let mut high = char_boundaries.len().saturating_sub(1);

        while low < high {
            let mid = (low + high).div_ceil(2);
            let prefix = &text[..char_boundaries[mid]];
            let candidate = format!("{}{}", prefix, ellipsis);

            if measure_width(&candidate) <= max_width {
                low = mid;
            } else {
                high = mid.saturating_sub(1);
            }
        }

        if low == 0 {
            ellipsis.to_string()
        } else {
            format!("{}{}", &text[..char_boundaries[low]], ellipsis)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_day_cell(
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
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
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
                .and_then(parse_color)
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

            // Event title constrained to a single truncated line
            let font_id = egui::FontId::proportional(11.0);
            let available_width = event_rect.width() - 6.0;
            let single_line_title = Self::truncate_single_line_to_width(
                ui,
                &title_text,
                &font_id,
                text_color,
                available_width,
            );

            ui.painter().text(
                Pos2::new(
                    event_rect.left() + 3.0,
                    event_rect.center().y,
                ),
                egui::Align2::LEFT_CENTER,
                single_line_title,
                font_id,
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

        // Context menu (extracted to month_context_menu module)
        let context_result = month_context_menu::render_cell_context_menu(
            ui,
            &response,
            rect,
            date,
            events,
            &pointer_event,
            &single_event_fallback,
            &pointer_hit,
            synced_event_ids,
            countdown_requests,
            active_countdown_events,
            database,
            show_event_dialog,
            event_dialog_date,
            event_dialog_recurrence,
            event_to_edit,
        );

        let delete_confirm_request = context_result.delete_confirm_request;

        // Handle pending template selection (return action)
        if let Some(template_action) = context_result.template_action {
            return (template_action, None, delete_confirm_request);
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
}
