use chrono::{Local, NaiveDate, NaiveTime};
use egui::{Color32, CursorIcon, Margin, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::palette::DayStripPalette;
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::drag::{DragContext, DragManager, DragView};
use crate::ui_egui::views::CountdownRequest;

pub struct DayView;

impl DayView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
    ) -> Option<Event> {
        let today = Local::now().date_naive();
        let is_today = *current_date == today;
        let day_strip_palette = DayStripPalette::from_ui(ui);

        // Get events for this day
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_day(&event_service, *current_date);

        // Day header
        let day_name = current_date.format("%A").to_string();
        let date_label = current_date.format("%B %d, %Y").to_string();
        let header_frame = egui::Frame::none()
            .fill(day_strip_palette.strip_bg)
            .rounding(egui::Rounding::same(12.0))
            .stroke(Stroke::new(1.0, day_strip_palette.strip_border))
            .inner_margin(Margin::symmetric(16.0, 12.0));

        let header_response = header_frame.show(ui, |strip_ui| {
            strip_ui.horizontal(|row_ui| {
                row_ui.vertical(|text_ui| {
                    let heading_color = if is_today {
                        day_strip_palette.today_text
                    } else {
                        day_strip_palette.text
                    };
                    let date_color = if is_today {
                        day_strip_palette.today_date_text
                    } else {
                        day_strip_palette.date_text
                    };

                    text_ui.label(
                        egui::RichText::new(&day_name)
                            .size(24.0)
                            .color(heading_color)
                            .strong(),
                    );
                    text_ui.label(
                        egui::RichText::new(&date_label)
                            .size(14.0)
                            .color(date_color),
                    );
                });

                let remaining_width = row_ui.available_width();
                row_ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |today_ui| {
                        today_ui.set_width(remaining_width);
                        if is_today {
                            egui::Frame::none()
                                .fill(day_strip_palette.badge_bg)
                                .rounding(egui::Rounding::same(10.0))
                                .inner_margin(Margin::symmetric(12.0, 6.0))
                                .show(today_ui, |badge_ui| {
                                    badge_ui.label(
                                        egui::RichText::new("Today")
                                            .color(day_strip_palette.badge_text)
                                            .size(12.0)
                                            .strong(),
                                    );
                                });
                        }
                    },
                );
            });
        });

        let header_rect = header_response.response.rect;
        ui.painter().hline(
            header_rect.x_range(),
            header_rect.bottom(),
            Stroke::new(1.0, day_strip_palette.accent_line),
        );

        ui.add_space(8.0);

        // Scrollable time slots
        let mut clicked_event = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(event) = Self::render_time_slots(
                    ui,
                    *current_date,
                    &events,
                    settings,
                    database,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                    countdown_requests,
                    active_countdown_events,
                ) {
                    clicked_event = Some(event);
                }
            });

        clicked_event
    }

    fn render_time_slots(
        ui: &mut egui::Ui,
        date: NaiveDate,
        events: &[Event],
        _settings: &Settings,
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
    ) -> Option<Event> {
        // Always render 15-minute intervals (4 slots per hour)
        const SLOT_INTERVAL: i64 = 15;

        let mut clicked_event: Option<Event> = None;

        // Draw 24 hours with 4 slots each
        for hour in 0..24 {
            for slot in 0..4 {
                let minute = slot * SLOT_INTERVAL;
                let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0).unwrap();
                let is_hour_start = slot == 0;

                // Calculate slot time range
                let slot_start = time;
                let slot_end = {
                    let total_minutes = hour * 60 + (minute + SLOT_INTERVAL);
                    let end_hour = (total_minutes / 60) as u32;
                    let end_minute = (total_minutes % 60) as u32;
                    if end_hour >= 24 {
                        NaiveTime::from_hms_opt(23, 59, 59).unwrap()
                    } else {
                        NaiveTime::from_hms_opt(end_hour, end_minute, 0).unwrap()
                    }
                };

                // Categorize events for this slot:
                // 1. Events that START in this slot (render full details)
                // 2. Events that are CONTINUING through this slot (render colored block only)
                let mut starting_events: Vec<&Event> = Vec::new();
                let mut continuing_events: Vec<&Event> = Vec::new();

                for event in events.iter() {
                    let event_start = event.start.time();
                    let event_end = event.end.time();

                    // Check if event overlaps with this slot
                    if event_start < slot_end && event_end > slot_start {
                        // Does it start in this slot?
                        if event_start >= slot_start && event_start < slot_end {
                            starting_events.push(event);
                        } else if event_start < slot_start {
                            // It started earlier and is continuing through this slot
                            continuing_events.push(event);
                        }
                    }
                }

                if let Some(event) = Self::render_time_slot(
                    ui,
                    date,
                    time,
                    hour,
                    is_hour_start,
                    &starting_events,
                    &continuing_events,
                    database,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                    countdown_requests,
                    active_countdown_events,
                ) {
                    clicked_event = Some(event);
                }
            }
        }

        clicked_event
    }

    fn render_time_slot(
        ui: &mut egui::Ui,
        date: NaiveDate,
        time: NaiveTime,
        hour: i64,
        is_hour_start: bool,
        starting_events: &[&Event],   // Events that start in this slot
        continuing_events: &[&Event], // Events continuing through this slot
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
    ) -> Option<Event> {
        let mut clicked_event: Option<Event> = None;

        ui.horizontal(|ui| {
            // Time label with fixed width (only on hour starts)
            ui.allocate_ui_with_layout(
                Vec2::new(50.0, 40.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if is_hour_start {
                        let time_str = format!("{:02}:00", hour);
                        ui.add_space(5.0); // Small padding on the right
                        ui.label(
                            egui::RichText::new(time_str)
                                .size(12.0)
                                .color(Color32::GRAY),
                        );
                    }
                },
            );

            // Time slot area
            let desired_size = Vec2::new(ui.available_width(), 40.0);
            let drag_sense = Sense::click_and_drag().union(Sense::hover());
            let (rect, response) = ui.allocate_exact_size(desired_size, drag_sense);

            let dark_mode = ui.style().visuals.dark_mode;
            let (hour_bg, regular_bg, hour_line_color, slot_line_color, hover_overlay) =
                if dark_mode {
                    (
                        Color32::from_gray(45),
                        Color32::from_gray(40),
                        Color32::from_gray(60),
                        Color32::from_gray(50),
                        Color32::from_rgba_unmultiplied(100, 150, 255, 30),
                    )
                } else {
                    (
                        Color32::from_rgb(235, 235, 235),
                        Color32::from_rgb(245, 245, 245),
                        Color32::from_rgb(210, 210, 210),
                        Color32::from_rgb(230, 230, 230),
                        Color32::from_rgba_unmultiplied(80, 120, 200, 25),
                    )
                };

            // Background
            let bg_color = if is_hour_start { hour_bg } else { regular_bg };
            ui.painter().rect_filled(rect, 0.0, bg_color);

            // Horizontal grid line
            let line_color = if is_hour_start {
                hour_line_color
            } else {
                slot_line_color
            };
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left(), rect.top()),
                    Pos2::new(rect.right(), rect.top()),
                ],
                Stroke::new(1.0, line_color),
            );

            // Hover effect
            if response.hovered() {
                ui.painter().rect_filled(rect, 0.0, hover_overlay);
            }

            let mut event_hitboxes: Vec<(Rect, Event)> = Vec::new();

            // Draw continuing events first (colored blocks only)
            for event in continuing_events {
                let event_rect = Self::render_event_continuation(ui, rect, event);
                event_hitboxes.push((event_rect, (*event).clone()));
            }

            // Draw starting events (full details)
            for event in starting_events {
                let event_rect = Self::render_event_in_slot(ui, rect, event);
                event_hitboxes.push((event_rect, (*event).clone()));
            }

            let pointer_pos = response.interact_pointer_pos();
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

            let pointer_for_hover = ui
                .ctx()
                .pointer_interact_pos()
                .or_else(|| ui.input(|i| i.pointer.hover_pos()));
            if let Some(pointer) = pointer_for_hover {
                if rect.contains(pointer) {
                    DragManager::update_hover(ui.ctx(), date, time, rect, pointer);
                    if DragManager::is_active_for_view(ui.ctx(), DragView::Day) {
                        ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                        ui.ctx().request_repaint();
                    }
                }
            }

            if let Some(drag_state) = DragManager::active_for_view(ui.ctx(), DragView::Day) {
                if drag_state.hovered_date == Some(date) && drag_state.hovered_time == Some(time) {
                    let highlight = rect.shrink2(Vec2::new(5.0, 4.0));
                    ui.painter().rect_filled(
                        highlight,
                        2.0,
                        Color32::from_rgba_unmultiplied(120, 200, 120, 35),
                    );
                    ui.painter().rect_stroke(
                        highlight,
                        2.0,
                        Stroke::new(1.5, Color32::from_rgb(120, 200, 120)),
                    );
                }
            }

            // Manual context menu handling - store popup state in egui memory
            let mut context_clicked_event: Option<Event> = None;
            let mut context_menu_event: Option<Event> = None;
            let popup_id = response
                .id
                .with(format!("context_menu_{}_{:?}", date, time));

            // Derive a narrower anchor rect from the slot so the popup doesn't stretch full width
            let mut popup_anchor_response = response.clone();
            popup_anchor_response.rect = Rect::from_min_size(
                Pos2::new(rect.left() + 55.0, rect.top()),
                Vec2::new(200.0, rect.height()),
            );

            if response.secondary_clicked() {
                context_menu_event = pointer_event.clone();
                ui.memory_mut(|mem| mem.open_popup(popup_id));
            }

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
                        ui.label(format!("Event: {}", event.title));
                        ui.separator();

                        if ui.button("✏ Edit").clicked() {
                            context_clicked_event = Some(event.clone());
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        if ui.button("🗑 Delete").clicked() {
                            if let Some(id) = event.id {
                                let service = EventService::new(database.connection());
                                let _ = service.delete(id);
                            }
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        if event.start > Local::now() {
                            let timer_exists = event
                                .id
                                .map(|id| active_countdown_events.contains(&id))
                                .unwrap_or(false);
                            if timer_exists {
                                ui.label(
                                    egui::RichText::new("Countdown already exists")
                                        .italics()
                                        .color(Color32::from_gray(150))
                                        .size(11.0),
                                );
                            } else if ui.button("⏱ Create Countdown").clicked() {
                                countdown_requests.push(CountdownRequest::from_event(&event));
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        }
                    } else {
                        ui.label("Create event");
                        ui.separator();

                        if ui.button("📅 New Event").clicked() {
                            *show_event_dialog = true;
                            *event_dialog_date = Some(date);
                            *event_dialog_time = Some(time);
                            *event_dialog_recurrence = None;
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        if ui.button("🔄 New Recurring Event").clicked() {
                            *show_event_dialog = true;
                            *event_dialog_date = Some(date);
                            *event_dialog_time = Some(time);
                            *event_dialog_recurrence = Some("FREQ=DAILY".to_string());
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                    }
                },
            );

            let mut clicked_from_ui: Option<Event> = context_clicked_event;

            // Handle click - check if we clicked on an event first
            if clicked_from_ui.is_none() && response.clicked() {
                if let Some(event) = pointer_event.clone() {
                    clicked_from_ui = Some(event);
                }

                if clicked_from_ui.is_none() {
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_time = Some(time); // Use the clicked time slot
                    *event_dialog_recurrence = None; // Default to non-recurring
                }
            }

            // Handle double-click for recurring event
            if response.double_clicked() {
                *show_event_dialog = true;
                *event_dialog_date = Some(date);
                *event_dialog_time = Some(time); // Use the clicked time slot
                *event_dialog_recurrence = Some("FREQ=DAILY".to_string());
            }

            if response.drag_started() {
                if let Some((hit_rect, event)) = pointer_hit.clone() {
                    if event.recurrence_rule.is_none() {
                        if let Some(drag_context) = DragContext::from_event(
                            &event,
                            pointer_pos
                                .map(|pos| pos - hit_rect.min)
                                .unwrap_or(Vec2::ZERO),
                            DragView::Day,
                        ) {
                            DragManager::begin(ui.ctx(), drag_context);
                            ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                        }
                    }
                }
            }

            if response.dragged() {
                ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
            }

            if response.drag_stopped() {
                if let Some(drag_context) = DragManager::finish_for_view(ui.ctx(), DragView::Day) {
                    if let Some(target_start) = drag_context
                        .hovered_start()
                        .or_else(|| date.and_time(time).and_local_timezone(Local).single())
                    {
                        let new_end = target_start + drag_context.duration;
                        let event_service = EventService::new(database.connection());
                        if let Ok(Some(mut event)) = event_service.get(drag_context.event_id) {
                            event.start = target_start;
                            event.end = new_end;
                            if let Err(err) = event_service.update(&event) {
                                eprintln!(
                                    "Failed to move event {}: {}",
                                    drag_context.event_id, err
                                );
                            }
                        }
                    }
                }
            }

            clicked_event = clicked_from_ui;
        });

        clicked_event
    }

    fn render_event_in_slot(ui: &mut egui::Ui, slot_rect: Rect, event: &Event) -> Rect {
        let event_color = event
            .color
            .as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));

        // Event background - fill the slot area (after the time label)
        let bg_rect = Rect::from_min_size(
            Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
            Vec2::new(slot_rect.width() - 60.0, slot_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bg_rect, 2.0, event_color);

        // Event bar (left side) - darker accent
        let bar_rect = Rect::from_min_size(
            Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
            Vec2::new(4.0, slot_rect.height() - 4.0),
        );
        ui.painter()
            .rect_filled(bar_rect, 2.0, event_color.linear_multiply(0.7));

        // Event title
        let text_rect = Rect::from_min_size(
            Pos2::new(bar_rect.right() + 5.0, slot_rect.top() + 2.0),
            Vec2::new(slot_rect.width() - 70.0, slot_rect.height() - 4.0),
        );

        // Use egui's layout system to properly truncate text
        let font_id = egui::FontId::proportional(13.0);
        let available_width = text_rect.width();

        let layout_job = egui::text::LayoutJob::simple(
            event.title.clone(),
            font_id.clone(),
            Color32::WHITE,
            available_width,
        );

        let galley = ui.fonts(|f| f.layout_job(layout_job));

        ui.painter().galley(
            Pos2::new(
                text_rect.left(),
                text_rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            Color32::WHITE,
        );

        // Time range
        let time_str = format!(
            "{} - {}",
            event.start.format("%H:%M"),
            event.end.format("%H:%M")
        );
        ui.painter().text(
            Pos2::new(text_rect.left(), text_rect.top() + 2.0),
            egui::Align2::LEFT_TOP,
            time_str,
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );

        bg_rect
    }

    fn render_event_continuation(ui: &mut egui::Ui, slot_rect: Rect, event: &Event) -> Rect {
        let event_color = event
            .color
            .as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));

        // Just render a colored bar to show the event continues through this slot
        let bar_rect = Rect::from_min_size(
            Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
            Vec2::new(4.0, slot_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bar_rect, 2.0, event_color);

        // Optional: render a lighter background to show continuation
        let bg_rect = Rect::from_min_size(
            Pos2::new(bar_rect.right() + 5.0, slot_rect.top() + 2.0),
            Vec2::new(slot_rect.width() - 70.0, slot_rect.height() - 4.0),
        );
        ui.painter()
            .rect_filled(bg_rect, 2.0, event_color.linear_multiply(0.3));

        bg_rect
    }

    fn get_events_for_day(event_service: &EventService, date: NaiveDate) -> Vec<Event> {
        use chrono::{Local, TimeZone};

        let start = Local
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end = Local
            .from_local_datetime(&date.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();

        event_service
            .expand_recurring_events(start, end)
            .unwrap_or_default()
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
