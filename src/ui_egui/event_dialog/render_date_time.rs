//! Date and time section rendering for the event dialog.
//!
//! Handles start/end date pickers, time pickers, all-day toggle,
//! and quick date adjustment buttons.

use chrono::Local;
use egui::{Color32, RichText};

use super::state::{DatePickerTarget, EventDialogState};
use super::widgets::{
    indented_row, labeled_row, render_inline_date_picker, render_time_picker, DatePickerAction,
};

/// Render the Date and Time section of the event dialog.
pub fn render_date_time_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    let today = Local::now().date_naive();

    ui.heading("Date and Time");
    ui.add_space(4.0);

    // Show info banner for past events
    if state.is_past_event {
        ui.horizontal(|ui| {
            ui.label(RichText::new("ℹ").color(Color32::from_rgb(100, 150, 200)));
            ui.label(
                RichText::new("This is a past event. Date and time cannot be modified.")
                    .color(Color32::from_rgb(150, 150, 150))
                    .italics(),
            );
        });
        ui.add_space(8.0);
    }

    // Start date section
    labeled_row(ui, "Start date:", |ui| {
        let btn_text = state.date.format("%B %d, %Y").to_string();

        if state.is_past_event {
            ui.label(format!("📅 {}", btn_text));
        } else {
            let is_start_picker_open =
                state.active_date_picker == Some(DatePickerTarget::StartDate);
            if ui
                .selectable_label(is_start_picker_open, format!("📅 {}", btn_text))
                .on_hover_text("Click to select date")
                .clicked()
            {
                if is_start_picker_open {
                    state.active_date_picker = None;
                } else {
                    state.active_date_picker = Some(DatePickerTarget::StartDate);
                    state.date_picker_viewing = state.date;
                }
            }
        }
    });

    // Show inline calendar for start date (only if not past event)
    if !state.is_past_event && state.active_date_picker == Some(DatePickerTarget::StartDate) {
        indented_row(ui, |ui| {
            let action = render_inline_date_picker(
                ui,
                DatePickerTarget::StartDate,
                state.date,
                &mut state.date_picker_viewing,
                None, // No constraint for start date
                today,
            );

            match action {
                DatePickerAction::Selected(date) => {
                    state.date = date;
                    // Ensure end date is not before start date
                    if state.end_date < state.date {
                        state.end_date = state.date;
                    }
                    state.active_date_picker = None;
                }
                DatePickerAction::Close => {
                    state.active_date_picker = None;
                }
                DatePickerAction::None => {}
            }
        });
    }

    // End date section
    labeled_row(ui, "End date:", |ui| {
        let btn_text = state.end_date.format("%B %d, %Y").to_string();

        if state.is_past_event {
            ui.label(format!("📅 {}", btn_text));
        } else {
            let is_end_picker_open = state.active_date_picker == Some(DatePickerTarget::EndDate);
            if ui
                .selectable_label(is_end_picker_open, format!("📅 {}", btn_text))
                .on_hover_text("Click to select date")
                .clicked()
            {
                if is_end_picker_open {
                    state.active_date_picker = None;
                } else {
                    state.active_date_picker = Some(DatePickerTarget::EndDate);
                    state.date_picker_viewing = state.end_date;
                }
            }
        }
    });

    // Show inline calendar for end date (only if not past event)
    if !state.is_past_event && state.active_date_picker == Some(DatePickerTarget::EndDate) {
        indented_row(ui, |ui| {
            let action = render_inline_date_picker(
                ui,
                DatePickerTarget::EndDate,
                state.end_date,
                &mut state.date_picker_viewing,
                Some(state.date), // Constrain: end date cannot be before start date
                today,
            );

            match action {
                DatePickerAction::Selected(date) => {
                    state.end_date = date;
                    state.active_date_picker = None;
                }
                DatePickerAction::Close => {
                    state.active_date_picker = None;
                }
                DatePickerAction::None => {}
            }
        });
    }

    // Quick date buttons (only if not past event)
    if !state.is_past_event {
        indented_row(ui, |ui| {
            if ui.button("Same day").clicked() {
                state.end_date = state.date;
            }
            if ui.button("+1 day").clicked() {
                state.end_date = state.end_date.succ_opt().unwrap_or(state.end_date);
            }
            if ui.button("+1 week").clicked() {
                state.end_date += chrono::Duration::days(7);
            }
        });
    }

    ui.add_space(4.0);
    if state.is_past_event {
        labeled_row(ui, "All-day:", |ui| {
            ui.label(if state.all_day { "Yes" } else { "No" });
        });
    } else {
        indented_row(ui, |ui| {
            ui.checkbox(&mut state.all_day, "All-day event");
        });
    }
    ui.add_space(4.0);

    if !state.all_day {
        labeled_row(ui, "Start time:", |ui| {
            if state.is_past_event {
                ui.label(state.start_time.format("%H:%M").to_string());
            } else {
                render_time_picker(ui, &mut state.start_time);
            }
        });

        labeled_row(ui, "End time:", |ui| {
            if state.is_past_event {
                ui.label(state.end_time.format("%H:%M").to_string());
            } else {
                render_time_picker(ui, &mut state.end_time);

                // Show warning if end time is before start time on the same day
                if state.date == state.end_date && state.end_time <= state.start_time {
                    ui.label(RichText::new("⚠").color(Color32::from_rgb(200, 150, 0)));
                }
            }
        });

        // Show validation message if times are invalid (only for editable events)
        if !state.is_past_event
            && state.date == state.end_date
            && state.end_time <= state.start_time
        {
            indented_row(ui, |ui| {
                ui.label(
                    RichText::new("Note: End time should be after start time")
                        .color(Color32::from_rgb(200, 150, 0))
                        .small(),
                );
            });
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}
