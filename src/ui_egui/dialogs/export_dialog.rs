//! Export date range dialog for exporting events to .ics files.

use chrono::{Datelike, Local, NaiveDate};
use egui::Context;

/// Result from the export dialog
pub enum ExportDialogResult {
    /// User hasn't made a choice yet
    None,
    /// User cancelled the dialog
    Cancelled,
    /// User confirmed export with the given date range
    Export { start: NaiveDate, end: NaiveDate },
}

/// Which date picker is currently active
#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveDatePicker {
    Start,
    End,
}

/// State for the export dialog
pub struct ExportDialogState {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    active_picker: Option<ActiveDatePicker>,
    viewing_date: NaiveDate,
}

impl Default for ExportDialogState {
    fn default() -> Self {
        let today = Local::now().date_naive();
        let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
        let end = (start + chrono::Months::new(1)) - chrono::Duration::days(1);
        Self {
            start_date: start,
            end_date: end,
            active_picker: None,
            viewing_date: today,
        }
    }
}

impl ExportDialogState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Renders the export date range dialog
pub fn render_export_range_dialog(
    ctx: &Context,
    state: &mut ExportDialogState,
) -> ExportDialogResult {
    let mut result = ExportDialogResult::None;
    let mut open = true;
    let today = Local::now().date_naive();

    egui::Window::new("Export Events")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(280.0);
            
            ui.add_space(5.0);
            ui.label("Select the date range for events to export:");
            ui.add_space(10.0);
            
            // Start date
            ui.horizontal(|ui| {
                ui.label("Start Date:");
                ui.add_space(10.0);
                
                let is_start_open = state.active_picker == Some(ActiveDatePicker::Start);
                let btn_text = state.start_date.format("%B %d, %Y").to_string();
                
                if ui.selectable_label(is_start_open, format!("📅 {}", btn_text))
                    .on_hover_text("Click to select date")
                    .clicked()
                {
                    if is_start_open {
                        state.active_picker = None;
                    } else {
                        state.active_picker = Some(ActiveDatePicker::Start);
                        state.viewing_date = state.start_date;
                    }
                }
            });
            
            // Show inline calendar for start date
            if state.active_picker == Some(ActiveDatePicker::Start) {
                ui.indent("start_picker_indent", |ui| {
                    let action = render_inline_date_picker(
                        ui,
                        "export_start",
                        state.start_date,
                        &mut state.viewing_date,
                        None, // No constraint for start date
                        today,
                    );
                    
                    match action {
                        DatePickerAction::Selected(date) => {
                            state.start_date = date;
                            // Ensure end date is not before start date
                            if state.end_date < state.start_date {
                                state.end_date = state.start_date;
                            }
                            state.active_picker = None;
                        }
                        DatePickerAction::Close => {
                            state.active_picker = None;
                        }
                        DatePickerAction::None => {}
                    }
                });
            }
            
            ui.add_space(5.0);
            
            // End date
            ui.horizontal(|ui| {
                ui.label("End Date:");
                ui.add_space(16.0);
                
                let is_end_open = state.active_picker == Some(ActiveDatePicker::End);
                let btn_text = state.end_date.format("%B %d, %Y").to_string();
                
                if ui.selectable_label(is_end_open, format!("📅 {}", btn_text))
                    .on_hover_text("Click to select date")
                    .clicked()
                {
                    if is_end_open {
                        state.active_picker = None;
                    } else {
                        state.active_picker = Some(ActiveDatePicker::End);
                        state.viewing_date = state.end_date;
                    }
                }
            });
            
            // Show inline calendar for end date
            if state.active_picker == Some(ActiveDatePicker::End) {
                ui.indent("end_picker_indent", |ui| {
                    let action = render_inline_date_picker(
                        ui,
                        "export_end",
                        state.end_date,
                        &mut state.viewing_date,
                        Some(state.start_date), // Constrain: end date cannot be before start
                        today,
                    );
                    
                    match action {
                        DatePickerAction::Selected(date) => {
                            state.end_date = date;
                            state.active_picker = None;
                        }
                        DatePickerAction::Close => {
                            state.active_picker = None;
                        }
                        DatePickerAction::None => {}
                    }
                });
            }
            
            // Quick range buttons
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Quick select:");
                if ui.small_button("This Month").clicked() {
                    let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                        .unwrap_or(today);
                    let end = (start + chrono::Months::new(1)) - chrono::Duration::days(1);
                    state.start_date = start;
                    state.end_date = end;
                    state.active_picker = None;
                }
                if ui.small_button("This Year").clicked() {
                    let start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
                    let end = NaiveDate::from_ymd_opt(today.year(), 12, 31).unwrap_or(today);
                    state.start_date = start;
                    state.end_date = end;
                    state.active_picker = None;
                }
                if ui.small_button("Last 30 Days").clicked() {
                    let end = today;
                    let start = end - chrono::Duration::days(30);
                    state.start_date = start;
                    state.end_date = end;
                    state.active_picker = None;
                }
            });
            
            // Validation
            let validation_error = if state.end_date < state.start_date {
                Some("End date must be after start date")
            } else {
                None
            };
            
            if let Some(error) = validation_error {
                ui.add_space(5.0);
                ui.colored_label(egui::Color32::RED, error);
            }
            
            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Buttons
            ui.horizontal(|ui| {
                let can_export = validation_error.is_none();
                
                ui.add_enabled_ui(can_export, |ui| {
                    if ui.button("Export...").clicked() {
                        result = ExportDialogResult::Export { 
                            start: state.start_date, 
                            end: state.end_date,
                        };
                    }
                });
                
                ui.add_space(5.0);
                
                if ui.button("Cancel").clicked() {
                    result = ExportDialogResult::Cancelled;
                }
            });
        });

    if !open {
        result = ExportDialogResult::Cancelled;
    }

    result
}

/// Result from the inline date picker
enum DatePickerAction {
    /// No action taken
    None,
    /// User selected a date
    Selected(NaiveDate),
    /// User wants to close the picker without selecting
    Close,
}

/// Render an inline calendar-style date picker (same style as event dialog)
fn render_inline_date_picker(
    ui: &mut egui::Ui,
    id_suffix: &str,
    current_date: NaiveDate,
    viewing_date: &mut NaiveDate,
    constraint_date: Option<NaiveDate>,
    today: NaiveDate,
) -> DatePickerAction {
    let mut action = DatePickerAction::None;

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

            // Fixed-width center label
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
                        let is_disabled = constraint_date.is_some_and(|min| current < min);

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
                if constraint_date.is_none_or(|min| today >= min) {
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

/// Shift a date by the given number of months
fn shift_month(date: NaiveDate, delta: i32) -> NaiveDate {
    let total_months = (date.year() * 12) + (date.month() as i32 - 1) + delta;
    let new_year = total_months.div_euclid(12);
    let new_month = (total_months.rem_euclid(12) + 1) as u32;
    let max_day = days_in_month(new_year, new_month);
    let day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap_or(date)
}

/// Get the number of days in a given month
fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30)
}
