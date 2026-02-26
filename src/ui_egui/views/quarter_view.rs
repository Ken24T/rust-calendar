use chrono::{Datelike, Local, NaiveDate};
use egui::{Pos2, Rect, Sense, Stroke, Vec2};

use super::palette::CalendarCellPalette;
use crate::models::settings::Settings;
use crate::ui_egui::theme::CalendarTheme;

#[allow(dead_code)]
pub struct QuarterView;

#[allow(dead_code)]
impl QuarterView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
        settings: &Settings,
        theme: &CalendarTheme,
    ) {
        let today = Local::now().date_naive();

        // Determine which quarter we're in
        let quarter_start_month = ((current_date.month() - 1) / 3) * 3 + 1;

        // Collect month dates
        let mut month_dates = Vec::new();
        for month_offset in 0..3 {
            let month = quarter_start_month + month_offset;
            let year = current_date.year();
            let month_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
            month_dates.push(month_date);
        }

        // Render all three months side by side with minimal spacing
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            for month_date in &month_dates {
                ui.vertical(|ui| {
                    ui.style_mut().spacing.item_spacing = Vec2::new(0.0, 5.0);

                    // Month header
                    let month_name = month_date.format("%B %Y").to_string();
                    ui.heading(&month_name);

                    // Day name headers
                    let day_names = ["S", "M", "T", "W", "T", "F", "S"];
                    egui::Grid::new(format!("quarter_day_names_{}", month_date.month()))
                        .spacing([2.0, 2.0])
                        .show(ui, |ui| {
                            for day in &day_names {
                                ui.label(egui::RichText::new(*day).size(12.0).strong());
                            }
                            ui.end_row();
                        });

                    ui.add_space(3.0);

                    // Calendar grid
                    Self::render_mini_calendar(
                        ui,
                        *month_date,
                        today,
                        show_event_dialog,
                        event_dialog_date,
                        event_dialog_recurrence,
                        settings,
                        theme,
                    );
                });
            }
        });
    }

    fn render_mini_calendar(
        ui: &mut egui::Ui,
        month_date: NaiveDate,
        today: NaiveDate,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
        settings: &Settings,
        theme: &CalendarTheme,
    ) {
        // Calculate calendar grid based on first_day_of_week setting
        let first_weekday = (month_date.weekday().num_days_from_sunday() as i32
            - settings.first_day_of_week as i32
            + 7)
            % 7;
        let days_in_month = Self::get_days_in_month(month_date.year(), month_date.month());

        // Build mini calendar grid
        let mut day_counter = 1 - first_weekday;

        let palette = CalendarCellPalette::from_theme(theme);

        ui.vertical(|ui| {
            egui::Grid::new(format!("quarter_grid_{}", month_date.month()))
                .spacing([2.0, 2.0])
                .show(ui, |ui| {
                    for _week in 0..6 {
                        for _day_of_week in 0..7 {
                            if day_counter < 1 || day_counter > days_in_month {
                                // Empty cell
                                let (rect, _response) =
                                    ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, palette.empty_bg);
                            } else {
                                // Day cell
                                let date = NaiveDate::from_ymd_opt(
                                    month_date.year(),
                                    month_date.month(),
                                    day_counter as u32,
                                )
                                .unwrap();

                                let is_today = date == today;

                                // Calculate weekend based on first_day_of_week
                                let day_of_week = (date.weekday().num_days_from_sunday() as i32
                                    - settings.first_day_of_week as i32
                                    + 7)
                                    % 7;
                                let is_weekend = day_of_week == 5 || day_of_week == 6;

                                Self::render_mini_day_cell(
                                    ui,
                                    day_counter,
                                    date,
                                    is_today,
                                    is_weekend,
                                    show_event_dialog,
                                    event_dialog_date,
                                    event_dialog_recurrence,
                                    palette,
                                );
                            }
                            day_counter += 1;
                        }
                        ui.end_row();
                    }
                });
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_mini_day_cell(
        ui: &mut egui::Ui,
        day: i32,
        date: NaiveDate,
        is_today: bool,
        is_weekend: bool,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
        palette: CalendarCellPalette,
    ) {
        let desired_size = Vec2::new(30.0, 30.0);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, Sense::click().union(Sense::hover()));

        // Background color
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

        // Hover effect
        if response.hovered() {
            ui.painter()
                .rect_stroke(rect, 2.0, Stroke::new(2.0, palette.hover_border));
        }

        // Day number
        let text_color = if is_today {
            palette.today_text
        } else {
            palette.text
        };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}", day),
            egui::FontId::proportional(12.0),
            text_color,
        );

        // Context menu trigger area
        let popup_id = response.id.with(format!("quarter_context_menu_{}", date));
        let mut popup_anchor_response = response.clone();
        popup_anchor_response.rect =
            Rect::from_min_size(Pos2::new(rect.left(), rect.top()), Vec2::new(140.0, 30.0));

        if response.secondary_clicked() {
            ui.memory_mut(|mem| mem.open_popup(popup_id));
        }

        egui::popup::popup_above_or_below_widget(
            ui,
            popup_id,
            &popup_anchor_response,
            egui::AboveOrBelow::Below,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.set_width(135.0);
                ui.label(date.format("%b %d").to_string());
                ui.separator();

                if ui.button("📅 New Event").clicked() {
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_recurrence = None;
                    ui.memory_mut(|mem| mem.close_popup());
                }

                if ui.button("🔄 New Quarterly Event").clicked() {
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_recurrence = Some("FREQ=MONTHLY;INTERVAL=3".to_string());
                    ui.memory_mut(|mem| mem.close_popup());
                }
            },
        );

        // Handle click to create event
        if response.clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_recurrence = None;
        }

        // Handle double-click for quarterly recurrence
        if response.double_clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_recurrence = Some("FREQ=MONTHLY;INTERVAL=3".to_string());
        }
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
}
