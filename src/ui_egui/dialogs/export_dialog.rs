//! Export date range dialog for exporting events to .ics files.

use chrono::{Datelike, NaiveDate};
use egui::{Context, Ui};

/// Result from the export dialog
pub enum ExportDialogResult {
    /// User hasn't made a choice yet
    None,
    /// User cancelled the dialog
    Cancelled,
    /// User confirmed export with the given date range
    Export { start: NaiveDate, end: NaiveDate },
}

/// Renders the export date range dialog
pub fn render_export_range_dialog(
    ctx: &Context,
    start_date: &mut Option<NaiveDate>,
    end_date: &mut Option<NaiveDate>,
    current_date: NaiveDate,
) -> ExportDialogResult {
    let mut result = ExportDialogResult::None;
    let mut open = true;
    
    // Initialize dates if not set
    if start_date.is_none() {
        let start = NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 1)
            .unwrap_or(current_date);
        *start_date = Some(start);
    }
    if end_date.is_none() {
        if let Some(start) = *start_date {
            let end = (start + chrono::Months::new(1)) - chrono::Duration::days(1);
            *end_date = Some(end);
        }
    }

    egui::Window::new("Export Events")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(320.0);
            
            ui.add_space(5.0);
            ui.label("Select the date range for events to export:");
            ui.add_space(10.0);
            
            egui::Grid::new("export_date_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Start Date:");
                    render_date_picker(ui, "export_start", start_date);
                    ui.end_row();
                    
                    ui.label("End Date:");
                    render_date_picker(ui, "export_end", end_date);
                    ui.end_row();
                });
            
            // Quick range buttons
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Quick select:");
                if ui.small_button("This Month").clicked() {
                    let start = NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 1)
                        .unwrap_or(current_date);
                    let end = (start + chrono::Months::new(1)) - chrono::Duration::days(1);
                    *start_date = Some(start);
                    *end_date = Some(end);
                }
                if ui.small_button("This Year").clicked() {
                    let start = NaiveDate::from_ymd_opt(current_date.year(), 1, 1)
                        .unwrap_or(current_date);
                    let end = NaiveDate::from_ymd_opt(current_date.year(), 12, 31)
                        .unwrap_or(current_date);
                    *start_date = Some(start);
                    *end_date = Some(end);
                }
                if ui.small_button("Last 30 Days").clicked() {
                    let end = current_date;
                    let start = end - chrono::Duration::days(30);
                    *start_date = Some(start);
                    *end_date = Some(end);
                }
            });
            
            // Validation
            let mut validation_error: Option<&str> = None;
            if let (Some(start), Some(end)) = (*start_date, *end_date) {
                if end < start {
                    validation_error = Some("End date must be after start date");
                }
            }
            
            if let Some(error) = validation_error {
                ui.add_space(5.0);
                ui.colored_label(egui::Color32::RED, error);
            }
            
            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Buttons
            ui.horizontal(|ui| {
                let can_export = validation_error.is_none() 
                    && start_date.is_some() 
                    && end_date.is_some();
                
                ui.add_enabled_ui(can_export, |ui| {
                    if ui.button("Export...").clicked() {
                        if let (Some(start), Some(end)) = (*start_date, *end_date) {
                            result = ExportDialogResult::Export { start, end };
                        }
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

/// Render a simple date picker (day/month/year dropdowns)
fn render_date_picker(ui: &mut Ui, id_prefix: &str, date: &mut Option<NaiveDate>) {
    let current = date.unwrap_or_else(|| chrono::Local::now().date_naive());
    
    let mut year = current.year();
    let mut month = current.month() as i32;
    let mut day = current.day() as i32;
    
    ui.horizontal(|ui| {
        // Year dropdown
        egui::ComboBox::from_id_source(format!("{}_year", id_prefix))
            .width(70.0)
            .selected_text(format!("{}", year))
            .show_ui(ui, |ui| {
                let current_year = chrono::Local::now().year();
                for y in (current_year - 5)..=(current_year + 10) {
                    ui.selectable_value(&mut year, y, format!("{}", y));
                }
            });
        
        // Month dropdown
        egui::ComboBox::from_id_source(format!("{}_month", id_prefix))
            .width(50.0)
            .selected_text(format!("{:02}", month))
            .show_ui(ui, |ui| {
                for m in 1..=12 {
                    ui.selectable_value(&mut month, m, format!("{:02}", m));
                }
            });
        
        // Day dropdown (adjust for month/year)
        let days_in_month = days_in_month(year, month as u32);
        if day > days_in_month as i32 {
            day = days_in_month as i32;
        }
        
        egui::ComboBox::from_id_source(format!("{}_day", id_prefix))
            .width(50.0)
            .selected_text(format!("{:02}", day))
            .show_ui(ui, |ui| {
                for d in 1..=days_in_month {
                    ui.selectable_value(&mut day, d as i32, format!("{:02}", d));
                }
            });
    });
    
    // Update the date if changed
    if let Some(new_date) = NaiveDate::from_ymd_opt(year, month as u32, day as u32) {
        *date = Some(new_date);
    }
}

/// Get the number of days in a given month
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Check if a year is a leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
