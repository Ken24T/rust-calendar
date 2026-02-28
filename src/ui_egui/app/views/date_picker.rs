//! Mini-calendar date picker popup.
//!
//! Provides the "Go to Date" floating window used by the toolbar 📅 button.

use super::super::CalendarApp;
use chrono::{Datelike, Local, NaiveDate};

impl CalendarApp {
    /// Render the mini calendar date picker popup as a floating window
    pub(in crate::ui_egui) fn render_date_picker_popup(&mut self, ctx: &egui::Context) {
        if !self.state.date_picker_state.is_open {
            return;
        }

        let viewing_date = self
            .state
            .date_picker_state
            .viewing_date
            .unwrap_or(self.current_date);
        let today = Local::now().date_naive();

        let mut is_open = true;
        egui::Window::new("📅 Go to Date")
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.set_max_width(220.0);
                self.render_date_picker_header(ui, viewing_date, today);
                ui.separator();
                self.render_date_picker_grid(ui, viewing_date, today);
                ui.separator();

                // Quick actions
                ui.horizontal(|ui| {
                    if ui.button("Today").clicked() {
                        self.current_date = today;
                        self.state.date_picker_state.close();
                        self.focus_on_current_time_if_visible();
                    }
                });
            });

        if !is_open {
            self.state.date_picker_state.close();
        }
    }

    /// Month/Year header with year and month navigation arrows.
    fn render_date_picker_header(
        &mut self,
        ui: &mut egui::Ui,
        viewing_date: NaiveDate,
        today: NaiveDate,
    ) {
        ui.horizontal(|ui| {
            if ui
                .small_button("◀◀")
                .on_hover_text("Previous year")
                .clicked()
            {
                if let Some(new_date) = viewing_date.with_year(viewing_date.year() - 1) {
                    self.state.date_picker_state.viewing_date = Some(new_date);
                }
            }
            if ui
                .small_button("◀")
                .on_hover_text("Previous month")
                .clicked()
            {
                self.state.date_picker_state.viewing_date = Some(shift_month(viewing_date, -1));
            }

            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    let header = format!("{}", viewing_date.format("%B %Y"));
                    if ui
                        .selectable_label(false, &header)
                        .on_hover_text("Click to go to today")
                        .clicked()
                    {
                        self.state.date_picker_state.viewing_date = Some(today);
                    }
                },
            );

            if ui
                .small_button("▶")
                .on_hover_text("Next month")
                .clicked()
            {
                self.state.date_picker_state.viewing_date = Some(shift_month(viewing_date, 1));
            }
            if ui
                .small_button("▶▶")
                .on_hover_text("Next year")
                .clicked()
            {
                if let Some(new_date) = viewing_date.with_year(viewing_date.year() + 1) {
                    self.state.date_picker_state.viewing_date = Some(new_date);
                }
            }
        });
    }

    /// Day-of-week headers and 6-row calendar grid.
    fn render_date_picker_grid(
        &mut self,
        ui: &mut egui::Ui,
        viewing_date: NaiveDate,
        today: NaiveDate,
    ) {
        let day_names = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

        egui::Grid::new("date_picker_grid")
            .num_columns(7)
            .spacing([4.0, 2.0])
            .min_col_width(24.0)
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
                        let is_selected = current == self.current_date;

                        let day_str = format!("{}", current.day());

                        let text = if is_today {
                            egui::RichText::new(&day_str)
                                .strong()
                                .color(egui::Color32::from_rgb(50, 150, 50))
                        } else if !is_current_month {
                            egui::RichText::new(&day_str).weak()
                        } else {
                            egui::RichText::new(&day_str)
                        };

                        if ui.selectable_label(is_selected, text).clicked() {
                            self.current_date = current;
                            self.state.date_picker_state.close();
                            self.focus_on_current_time_if_visible();
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
    }
}

/// Shift a date by the given number of months.
fn shift_month(date: NaiveDate, delta: i32) -> NaiveDate {
    let total_months = (date.year() * 12) + (date.month() as i32 - 1) + delta;
    let new_year = total_months.div_euclid(12);
    let new_month = (total_months.rem_euclid(12) + 1) as u32;
    let max_day = days_in_month(new_year, new_month);
    let day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap_or(date)
}

/// Get the number of days in a given month.
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
