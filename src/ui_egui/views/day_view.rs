use chrono::{Local, NaiveDate, NaiveTime, TimeZone, Timelike};
use egui::{Color32, CursorIcon, Margin, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::palette::{DayStripPalette, TimeGridPalette};
use super::week_shared::{maybe_focus_slot, parse_color, DeleteConfirmRequest, EventInteractionResult};
use super::{AutoFocusRequest, CountdownRequest};
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::models::template::EventTemplate;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::services::template::TemplateService;
use crate::ui_egui::drag::{DragContext, DragManager, DragView};
use crate::ui_egui::resize::{HandleRects, ResizeContext, ResizeHandle, ResizeManager, ResizeView, draw_handles, draw_resize_preview};
use crate::ui_egui::theme::CalendarTheme;

use super::filter_events_by_category;

pub struct DayView;

impl DayView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        theme: &CalendarTheme,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        focus_request: &mut Option<AutoFocusRequest>,
        category_filter: Option<&str>,
    ) -> EventInteractionResult {
        let mut result = EventInteractionResult::default();
        let today = Local::now().date_naive();
        let is_today = *current_date == today;
        let day_strip_palette = DayStripPalette::from_theme(theme);
        let time_palette = TimeGridPalette::from_theme(theme);

        // Get events for this day
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_day(&event_service, *current_date);
        let events = filter_events_by_category(events, category_filter);

        // Day header
        let day_name = current_date.format("%A").to_string();
        let date_label = current_date.format("%B %d, %Y").to_string();
        let header_frame = egui::Frame::none()
            .fill(day_strip_palette.header_bg)
            .rounding(egui::Rounding::same(12.0))
            .stroke(Stroke::new(1.0, day_strip_palette.strip_border))
            .inner_margin(Margin::symmetric(16.0, 12.0));

        let header_response = header_frame.show(ui, |strip_ui| {
            strip_ui.horizontal(|row_ui| {
                row_ui.vertical(|text_ui| {
                    let heading_color = if is_today {
                        day_strip_palette.today_text
                    } else {
                        day_strip_palette.header_text
                    };
                    let date_color = if is_today {
                        day_strip_palette.today_date_text
                    } else {
                        day_strip_palette.header_text
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
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let slot_result = Self::render_time_slots(
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
                    &time_palette,
                    focus_request,
                );
                result.merge(slot_result);
            });

        result
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
        palette: &TimeGridPalette,
        focus_request: &mut Option<AutoFocusRequest>,
    ) -> EventInteractionResult {
        let mut result = EventInteractionResult::default();
        // Always render 15-minute intervals (4 slots per hour)
        const SLOT_INTERVAL: i64 = 15;

        // Remove vertical spacing between slots so time calculations are accurate
        ui.spacing_mut().item_spacing.y = 0.0;

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

                let slot_result = Self::render_time_slot(
                    ui,
                    date,
                    time,
                    hour,
                    slot_end,
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
                    palette,
                    focus_request,
                );
                result.merge(slot_result);
            }
        }

        // Draw current time indicator if viewing today
        let now = Local::now();
        let now_date = now.date_naive();
        let now_time = now.time();

        if date == now_date {
            // Calculate Y position based on time
            // Each hour has 4 slots (15 minutes each), each slot is 40 pixels high
            const SLOT_HEIGHT: f32 = 40.0;
            const SLOTS_PER_HOUR: f32 = 4.0;
            
            let hours_since_midnight = now_time.hour() as f32 + (now_time.minute() as f32 / 60.0);
            let relative_y = hours_since_midnight * SLOTS_PER_HOUR * SLOT_HEIGHT;

            // Get the UI's current position to calculate absolute coordinates
            let ui_top = ui.min_rect().top();
            let y_position = ui_top + relative_y;

            // Calculate X position across the full width
            let ui_left = ui.min_rect().left();
            let ui_right = ui.min_rect().right();
            let x_start = ui_left + 50.0; // After time labels
            let x_end = ui_right;

            // Draw the indicator line
            let painter = ui.painter();
            let line_color = Color32::from_rgb(255, 100, 100); // Red indicator
            let circle_center = egui::pos2(ui_left + 46.0, y_position);

            // Draw a small circle at the start
            painter.circle_filled(circle_center, 3.0, line_color);

            // Draw the horizontal line
            painter.line_segment(
                [
                    egui::pos2(x_start, y_position),
                    egui::pos2(x_end, y_position),
                ],
                egui::Stroke::new(2.0, line_color),
            );
        }

        result
    }

    fn render_time_slot(
        ui: &mut egui::Ui,
        date: NaiveDate,
        time: NaiveTime,
        hour: i64,
        slot_end: NaiveTime,
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
        palette: &TimeGridPalette,
        focus_request: &mut Option<AutoFocusRequest>,
    ) -> EventInteractionResult {
        let mut result = EventInteractionResult::default();

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

            maybe_focus_slot(ui, rect, date, time, slot_end, focus_request);

            let hour_bg = palette.hour_bg;
            let regular_bg = palette.regular_bg;
            let hour_line_color = palette.hour_line;
            let slot_line_color = palette.slot_line;
            let hover_overlay = palette.hover_overlay;

            // Check if this slot contains the current time (only for today)
            let today = Local::now().date_naive();
            let now = Local::now().time();
            let is_current_time_slot = date == today && now >= time && now < slot_end;
            
            // Background - highlight current time slot with a subtle tint
            let bg_color = if is_current_time_slot {
                // Blend with a soft highlight color (light blue/cyan tint)
                let highlight = Color32::from_rgba_unmultiplied(100, 180, 255, 25);
                let base = if is_hour_start { hour_bg } else { regular_bg };
                // Simple alpha blend
                Color32::from_rgb(
                    ((base.r() as u16 * 230 + highlight.r() as u16 * 25) / 255) as u8,
                    ((base.g() as u16 * 230 + highlight.g() as u16 * 25) / 255) as u8,
                    ((base.b() as u16 * 230 + highlight.b() as u16 * 25) / 255) as u8,
                )
            } else if is_hour_start {
                hour_bg
            } else {
                regular_bg
            };
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

            // Hover effect with cursor change
            if response.hovered() {
                ui.painter().rect_filled(rect, 0.0, hover_overlay);
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }

            // Track (rect, event, is_starting, is_ending) for handle visibility
            let mut event_hitboxes: Vec<(Rect, Event, bool, bool)> = Vec::new();

            // Draw continuing events first (colored blocks only)
            for event in continuing_events {
                let event_rect = Self::render_event_continuation(ui, rect, event);
                // Check if event ends in this slot
                let event_end = event.end.time();
                let is_ending = event_end > time && event_end <= slot_end;
                // Continuing events never show top handle (they started earlier)
                event_hitboxes.push((event_rect, (*event).clone(), false, is_ending));
            }

            // Draw starting events (full details)
            for event in starting_events {
                let event_rect = Self::render_event_in_slot(ui, rect, event);
                // Check if event ends in this slot
                let event_end = event.end.time();
                let is_ending = event_end > time && event_end <= slot_end;
                // Starting events always show top handle
                event_hitboxes.push((event_rect, (*event).clone(), true, is_ending));
            }

            // Build resize handle info for each event
            let event_handles: Vec<(Rect, Event, HandleRects)> = event_hitboxes
                .iter()
                .map(|(r, e, is_starting, is_ending)| {
                    (*r, e.clone(), HandleRects::for_timed_event_in_slot(*r, *is_starting, *is_ending))
                })
                .collect();

            // Check for pointer position - use hover position to catch right-clicks too
            let pointer_pos = response
                .interact_pointer_pos()
                .or_else(|| ui.input(|i| i.pointer.hover_pos()));
            // Include is_starting and is_ending flags for proper handle visibility
            let pointer_hit = pointer_pos.and_then(|pos| {
                event_hitboxes
                    .iter()
                    .rev()
                    .find(|(hit_rect, _, _, _)| hit_rect.contains(pos))
                    .map(|(hit_rect, event, is_starting, is_ending)| {
                        (*hit_rect, event.clone(), *is_starting, *is_ending)
                    })
            });
            let pointer_event = pointer_hit.as_ref().map(|(_, event, _, _)| event.clone());
            let single_event_fallback = if event_hitboxes.len() == 1 {
                Some(event_hitboxes[0].1.clone())
            } else {
                None
            };

            // Check if pointer is on a resize handle
            let hovered_handle: Option<(ResizeHandle, Rect, Event)> = pointer_pos.and_then(|pos| {
                event_handles
                    .iter()
                    .rev()
                    .find_map(|(event_rect, event, handles)| {
                        // Only allow resize for non-recurring events
                        if event.recurrence_rule.is_some() {
                            return None;
                        }
                        handles.hit_test(pos).map(|h| (h, *event_rect, event.clone()))
                    })
            });

            // Draw resize handles on hovered event (when not dragging/resizing)
            let is_dragging = DragManager::is_active_for_view(ui.ctx(), DragView::Day);
            let is_resizing = ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Day);
            
            // Draw resize preview silhouette when actively resizing
            if is_resizing {
                if let Some(resize_ctx) = ResizeManager::active_for_view(ui.ctx(), ResizeView::Day) {
                    // Find the event color for the preview
                    let event_color = event_hitboxes
                        .iter()
                        .find(|(_, e, _, _)| e.id == Some(resize_ctx.event_id))
                        .map(|(_, e, _, _)| {
                            e.color
                                .as_deref()
                                .and_then(parse_color)
                                .unwrap_or(Color32::from_rgb(100, 150, 200))
                        })
                        .unwrap_or(Color32::from_rgb(100, 150, 200));
                    
                    // Draw the preview for this slot
                    draw_resize_preview(
                        ui,
                        &resize_ctx,
                        rect,
                        date,
                        time,
                        slot_end,
                        event_color,
                        55.0, // Left margin matching event rendering
                    );
                }
            }
            
            if !is_dragging && !is_resizing {
                if let Some((hit_rect, hovered_event, is_starting, is_ending)) = &pointer_hit {
                    // Only show handles for non-recurring events
                    if hovered_event.recurrence_rule.is_none() {
                        // Use slot-aware handles: only show handles that are active in this slot
                        let handles = HandleRects::for_timed_event_in_slot(*hit_rect, *is_starting, *is_ending);
                        let hovered_h = hovered_handle.as_ref().map(|(h, _, _)| *h);
                        let event_color = hovered_event
                            .color
                            .as_deref()
                            .and_then(parse_color)
                            .unwrap_or(Color32::from_rgb(100, 150, 200));
                        draw_handles(ui, &handles, hovered_h, event_color);
                    }
                }
            }

            // Show tooltip when hovering over an event (but not on resize handles)
            if let Some((hit_rect, hovered_event, _, _)) = &pointer_hit {
                if response.hovered() 
                    && hit_rect.contains(pointer_pos.unwrap_or_default())
                    && hovered_handle.is_none()
                {
                    let tooltip_text = super::week_shared::format_event_tooltip(hovered_event);
                    response.clone().on_hover_ui_at_pointer(|ui| {
                        ui.label(tooltip_text);
                    });
                }
            }

            // Set cursor for resize handles
            if let Some((handle, _, _)) = &hovered_handle {
                if !is_dragging && !is_resizing {
                    ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
                }
            }

            let pointer_for_hover = ui
                .ctx()
                .pointer_interact_pos()
                .or_else(|| ui.input(|i| i.pointer.hover_pos()));
            if let Some(pointer) = pointer_for_hover {
                // During active resize, update hover for ANY slot the pointer is over
                // (not just when contained in the specific slot rect)
                let is_resize_active = ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Day);
                
                if rect.contains(pointer) {
                    // Update drag hover when pointer is in rect
                    DragManager::update_hover(ui.ctx(), date, time, rect, pointer);
                    
                    // Update resize hover - works for ANY slot during active resize
                    if is_resize_active {
                        ResizeManager::update_hover(ui.ctx(), date, time, slot_end, pointer);
                    }
                }
                
                // Set cursor icons
                if DragManager::is_active_for_view(ui.ctx(), DragView::Day) {
                    ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                    ui.ctx().request_repaint();
                }
                if let Some(resize_ctx) = ResizeManager::active_for_view(ui.ctx(), ResizeView::Day) {
                    ui.output_mut(|out| out.cursor_icon = resize_ctx.handle.cursor_icon());
                    ui.ctx().request_repaint();
                }
            }
            
            // Check for global mouse release to complete resize operations
            // This handles the case where drag started on one slot but ended on another
            let primary_released = ui.input(|i| i.pointer.primary_released());
            if primary_released && ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Day) {
                if let Some(resize_ctx) = ResizeManager::finish_for_view(ui.ctx(), ResizeView::Day) {
                    log::info!(
                        "Resize finished: handle={:?}, hovered_time={:?}, original_start={}, original_end={}",
                        resize_ctx.handle,
                        resize_ctx.hovered_time,
                        resize_ctx.original_start,
                        resize_ctx.original_end
                    );
                    if let Some((new_start, new_end)) = resize_ctx.hovered_times() {
                        log::info!("New times: start={}, end={}", new_start, new_end);
                        let event_service = EventService::new(database.connection());
                        if let Ok(Some(mut event)) = event_service.get(resize_ctx.event_id) {
                            event.start = new_start;
                            event.end = new_end;
                            if let Err(err) = event_service.update(&event) {
                                log::error!(
                                    "Failed to resize event {}: {}",
                                    resize_ctx.event_id, err
                                );
                            } else {
                                result.moved_events.push(event);
                            }
                        }
                    } else {
                        log::warn!("hovered_times() returned None");
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

                        // Show countdown option prominently for future events
                        if event.start > Local::now() {
                            let timer_exists = event
                                .id
                                .map(|id| active_countdown_events.contains(&id))
                                .unwrap_or(false);
                            if timer_exists {
                                ui.label(
                                    egui::RichText::new("⏱ Countdown active")
                                        .italics()
                                        .color(Color32::from_rgb(100, 200, 100))
                                        .size(11.0),
                                );
                            } else if ui.button("⏱ Create Countdown").clicked() {
                                countdown_requests.push(CountdownRequest::from_event(&event));
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                            ui.separator();
                        }

                        // Delete options - different for recurring events
                        if event.recurrence_rule.is_some() {
                            if ui.button("🗑 Delete This Occurrence").clicked() {
                                if let Some(id) = event.id {
                                    result.delete_confirm_request = Some(DeleteConfirmRequest {
                                        event_id: id,
                                        event_title: event.title.clone(),
                                        occurrence_only: true,
                                        occurrence_date: Some(event.start),
                                    });
                                }
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                            if ui.button("🗑 Delete All Occurrences").clicked() {
                                if let Some(id) = event.id {
                                    result.delete_confirm_request = Some(DeleteConfirmRequest {
                                        event_id: id,
                                        event_title: event.title.clone(),
                                        occurrence_only: false,
                                        occurrence_date: None,
                                    });
                                }
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        } else {
                            if ui.button("🗑 Delete").clicked() {
                                if let Some(id) = event.id {
                                    result.delete_confirm_request = Some(DeleteConfirmRequest {
                                        event_id: id,
                                        event_title: event.title.clone(),
                                        occurrence_only: false,
                                        occurrence_date: None,
                                    });
                                }
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        }

                        if ui.button("📤 Export this event").clicked() {
                            // Export event to ICS file
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(&format!("{}.ics", event.title.replace(' ', "_")))
                                .add_filter("iCalendar", &["ics"])
                                .save_file()
                            {
                                use crate::services::icalendar::export;
                                match export::single(&event) {
                                    Ok(ics_content) => {
                                        if let Err(e) = std::fs::write(&path, ics_content) {
                                            log::error!("Failed to write ICS file: {}", e);
                                        } else {
                                            log::info!("Exported event to {:?}", path);
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Failed to export event: {}", e);
                                    }
                                }
                            }
                            ui.memory_mut(|mem| mem.close_popup());
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
                        
                        // Template submenu
                        let templates: Vec<EventTemplate> = TemplateService::new(database.connection())
                            .list_all()
                            .unwrap_or_default();
                        
                        if !templates.is_empty() {
                            ui.separator();
                            ui.menu_button("📋 From Template", |ui| {
                                for template in &templates {
                                    let label = format!("{}", template.name);
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
                                            result.template_selection = Some((id, date, Some(time)));
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                }
                            });
                        }
                    }
                },
            );

            // Store context menu edit request to result
            if let Some(event) = context_clicked_event {
                result.event_to_edit = Some(event);
            }

            // Check drag_started BEFORE clicked to ensure drag detection works
            if response.drag_started() {
                // Use interact_pointer_pos for the drag start position
                let drag_start_pos = response.interact_pointer_pos();
                
                // Recalculate which handle was clicked using the drag start position
                let drag_handle: Option<(ResizeHandle, Rect, Event)> = drag_start_pos.and_then(|pos| {
                    event_handles
                        .iter()
                        .rev()
                        .find_map(|(event_rect, event, handles)| {
                            if event.recurrence_rule.is_some() {
                                return None;
                            }
                            handles.hit_test(pos).map(|h| (h, *event_rect, event.clone()))
                        })
                });
                
                // First check if we're starting a resize operation
                if let Some((handle, _hit_rect, event)) = drag_handle {
                    // Start resize instead of drag
                    if let Some(resize_context) = ResizeContext::from_event(
                        &event,
                        handle,
                        ResizeView::Day,
                    ) {
                        ResizeManager::begin(ui.ctx(), resize_context);
                        ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
                    }
                } else if let Some((hit_rect, event, _, _)) = pointer_hit.clone() {
                    // Otherwise start a drag operation
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
            } else if result.event_to_edit.is_none() && response.clicked() {
                if let Some(event) = pointer_event.clone() {
                    result.event_to_edit = Some(event);
                }

                if result.event_to_edit.is_none() {
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

            if response.dragged() {
                // Set appropriate cursor for drag or resize
                if ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Day) {
                    if let Some(resize_ctx) = ResizeManager::active_for_view(ui.ctx(), ResizeView::Day) {
                        ui.output_mut(|out| out.cursor_icon = resize_ctx.handle.cursor_icon());
                    }
                } else {
                    ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                }
            }

            if response.drag_stopped() {
                // Note: Resize completion is handled by global primary_released check above
                // This ensures resize works even when drag ends on a different slot
                
                // Handle drag completion
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
                                log::error!(
                                    "Failed to move event {}: {}",
                                    drag_context.event_id, err
                                );
                            } else {
                                // Track moved event for countdown card sync
                                result.moved_events.push(event);
                            }
                        }
                    }
                }
            }
        });

        result
    }

    fn render_event_in_slot(ui: &mut egui::Ui, slot_rect: Rect, event: &Event) -> Rect {
        let now = Local::now();
        let is_past = event.end < now;
        
        let base_color = event
            .color
            .as_deref()
            .and_then(parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));
        
        // Dim past events with stronger dimming for visibility
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
        
        // Dim text for past events
        let text_color = if is_past {
            Color32::from_rgba_unmultiplied(255, 255, 255, 180)
        } else {
            Color32::WHITE
        };

        let layout_job = egui::text::LayoutJob::simple(
            event.title.clone(),
            font_id.clone(),
            text_color,
            available_width,
        );

        let galley = ui.fonts(|f| f.layout_job(layout_job));

        ui.painter().galley(
            Pos2::new(
                text_rect.left(),
                text_rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            text_color,
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
            text_color,
        );

        bg_rect
    }

    fn render_event_continuation(ui: &mut egui::Ui, slot_rect: Rect, event: &Event) -> Rect {
        let now = Local::now();
        let is_past = event.end < now;
        
        let base_color = event
            .color
            .as_deref()
            .and_then(parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));
        
        // Dim past events with stronger dimming for visibility
        let event_color = if is_past {
            Color32::from_rgba_unmultiplied(
                (base_color.r() as f32 * 0.25) as u8,
                (base_color.g() as f32 * 0.25) as u8,
                (base_color.b() as f32 * 0.25) as u8,
                120,
            )
        } else {
            base_color
        };

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
            .into_iter()
            .filter(|e| !e.all_day)
            .collect()
    }
}
