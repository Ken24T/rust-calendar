use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use egui::Color32;

use super::state::DatePickerTarget;
use crate::ui_egui::views::utils::shift_month;

/// Render a time picker with hour and minute comboboxes
pub fn render_time_picker(ui: &mut egui::Ui, time: &mut NaiveTime) {
    let mut hour = time.hour();
    let mut minute = time.minute();

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_source(format!("hour_{:p}", time))
            .width(60.0)
            .selected_text(format!("{:02}", hour))
            .show_ui(ui, |ui| {
                for h in 0..24 {
                    ui.selectable_value(&mut hour, h, format!("{:02}", h));
                }
            });

        ui.label(":");

        egui::ComboBox::from_id_source(format!("minute_{:p}", time))
            .width(60.0)
            .selected_text(format!("{:02}", minute))
            .show_ui(ui, |ui| {
                for m in (0..60).step_by(15) {
                    ui.selectable_value(&mut minute, m, format!("{:02}", m));
                }
            });
    });

    if let Some(new_time) = NaiveTime::from_hms_opt(hour, minute, 0) {
        *time = new_time;
    }
}

/// Result from the inline date picker
pub enum DatePickerAction {
    /// No action taken
    None,
    /// User selected a date
    Selected(NaiveDate),
    /// User wants to close the picker without selecting
    Close,
}

/// Render an inline calendar-style date picker
/// 
/// # Arguments
/// * `ui` - The egui UI context
/// * `target` - Which date field this picker is for (start/end)
/// * `current_date` - The currently selected date for this field
/// * `viewing_date` - The month/year currently being viewed
/// * `constraint_date` - For end date, this is the start date (minimum); None for start date
/// * `today` - Today's date for highlighting
/// 
/// Returns the action to take (None, Selected date, or Close)
pub fn render_inline_date_picker(
    ui: &mut egui::Ui,
    target: DatePickerTarget,
    current_date: NaiveDate,
    viewing_date: &mut NaiveDate,
    constraint_date: Option<NaiveDate>,
    today: NaiveDate,
) -> DatePickerAction {
    let mut action = DatePickerAction::None;
    
    let id_suffix = match target {
        DatePickerTarget::StartDate => "start",
        DatePickerTarget::EndDate => "end",
    };

    ui.vertical(|ui| {
        // Constrain the width of the entire date picker
        ui.set_max_width(220.0);
        
        // Month/Year header with navigation
        ui.horizontal(|ui| {
            if ui.small_button("◀◀").on_hover_text("Previous year").clicked() {
                if let Some(new_date) = viewing_date.with_year(viewing_date.year() - 1) {
                    *viewing_date = new_date;
                }
            }
            if ui.small_button("◀").on_hover_text("Previous month").clicked() {
                *viewing_date = shift_month(*viewing_date, -1);
            }

            // Fixed-width center label instead of justified layout
            let header = format!("{}", viewing_date.format("%b %Y"));
            ui.add_space(4.0);
            if ui.selectable_label(false, &header).on_hover_text("Go to today").clicked() {
                *viewing_date = today;
            }
            ui.add_space(4.0);

            if ui.small_button("▶").on_hover_text("Next month").clicked() {
                *viewing_date = shift_month(*viewing_date, 1);
            }
            if ui.small_button("▶▶").on_hover_text("Next year").clicked() {
                if let Some(new_date) = viewing_date.with_year(viewing_date.year() + 1) {
                    *viewing_date = new_date;
                }
            }
        });

        ui.separator();

        // Day of week headers and calendar grid
        let day_names = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
        
        egui::Grid::new(format!("date_picker_grid_{}", id_suffix))
            .num_columns(7)
            .spacing([2.0, 2.0])
            .min_col_width(22.0)
            .show(ui, |ui| {
                // Header row
                for name in &day_names {
                    ui.label(egui::RichText::new(*name).small().strong());
                }
                ui.end_row();

                // Calendar grid
                let first_of_month = viewing_date.with_day(1).unwrap();
                let start_weekday = first_of_month.weekday().num_days_from_sunday() as i64;

                // Start from the Sunday before the first of the month
                let grid_start = first_of_month - chrono::Duration::days(start_weekday);

                let mut current = grid_start;
                for _week in 0..6 {
                    for _day in 0..7 {
                        let is_current_month = current.month() == viewing_date.month();
                        let is_today = current == today;
                        let is_selected = current == current_date;
                        
                        // Check if date is constrained (for end date, can't be before start)
                        let is_disabled = constraint_date.map_or(false, |min| current < min);

                        let day_str = format!("{}", current.day());

                        let text = if is_today {
                            egui::RichText::new(&day_str).strong().color(egui::Color32::from_rgb(50, 150, 50))
                        } else if is_disabled {
                            egui::RichText::new(&day_str).weak().strikethrough()
                        } else if !is_current_month {
                            egui::RichText::new(&day_str).weak()
                        } else {
                            egui::RichText::new(&day_str)
                        };

                        let response = ui.add_enabled(!is_disabled, egui::SelectableLabel::new(is_selected, text));
                        
                        if response.clicked() && !is_disabled {
                            action = DatePickerAction::Selected(current);
                        }

                        current += chrono::Duration::days(1);
                    }
                    ui.end_row();

                    // Stop if we've gone past this month
                    if current.month() != viewing_date.month() && current.day() > 7 {
                        break;
                    }
                }
            });

        ui.separator();

        // Quick actions
        ui.horizontal(|ui| {
            if ui.button("Today").clicked() {
                // Only select today if it's not disabled
                if constraint_date.map_or(true, |min| today >= min) {
                    action = DatePickerAction::Selected(today);
                } else {
                    // Jump to view today but don't select
                    *viewing_date = today;
                }
            }
            if ui.button("Close").clicked() {
                action = DatePickerAction::Close;
            }
        });
    });

    action
}

pub fn parse_hex_color(hex: &str) -> Option<Color32> {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}
