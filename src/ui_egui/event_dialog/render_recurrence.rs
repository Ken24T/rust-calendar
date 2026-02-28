//! Recurrence section rendering for the event dialog.
//!
//! Handles frequency selection, interval, pattern configuration,
//! BYDAY toggles, weekday shortcuts, and end condition settings.

use chrono::Duration;

use crate::models::settings::Settings;

use super::recurrence::{RecurrenceFrequency, RecurrencePattern, Weekday};
use super::state::EventDialogState;
use super::widgets::{indented_row, labeled_row};

/// Render the Recurrence section of the event dialog.
pub fn render_recurrence_section(
    ui: &mut egui::Ui,
    state: &mut EventDialogState,
    settings: &Settings,
) {
    ui.heading("Recurrence");
    ui.add_space(4.0);

    indented_row(ui, |ui| {
        ui.checkbox(&mut state.is_recurring, "Repeat this event");
    });

    if !state.is_recurring {
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        return;
    }

    ui.add_space(4.0);
    labeled_row(ui, "Frequency:", |ui| {
        egui::ComboBox::from_id_source("frequency_combo")
            .selected_text(state.frequency.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.frequency, RecurrenceFrequency::Daily, "Daily");
                ui.selectable_value(
                    &mut state.frequency,
                    RecurrenceFrequency::Weekly,
                    "Weekly",
                );
                ui.selectable_value(
                    &mut state.frequency,
                    RecurrenceFrequency::Monthly,
                    "Monthly",
                );
                ui.selectable_value(
                    &mut state.frequency,
                    RecurrenceFrequency::Yearly,
                    "Yearly",
                );
            });
    });

    labeled_row(ui, "Every:", |ui| {
        ui.add(egui::DragValue::new(&mut state.interval).range(1..=999));
        ui.label(match state.frequency {
            RecurrenceFrequency::Daily => "day(s)",
            RecurrenceFrequency::Weekly => "week(s)",
            RecurrenceFrequency::Monthly => "month(s)",
            RecurrenceFrequency::Yearly => "year(s)",
        });
    });

    render_recurrence_pattern(ui, state);
    render_byday_section(ui, state, settings);
    render_recurrence_end_section(ui, state);

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_recurrence_pattern(ui: &mut egui::Ui, state: &mut EventDialogState) {
    if !matches!(
        state.frequency,
        RecurrenceFrequency::Monthly | RecurrenceFrequency::Yearly
    ) {
        return;
    }

    ui.add_space(4.0);
    labeled_row(ui, "Repeat on:", |ui| {
        egui::ComboBox::from_id_source("pattern_combo")
            .selected_text(state.pattern.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.pattern, RecurrencePattern::None, "None");
                ui.selectable_value(
                    &mut state.pattern,
                    RecurrencePattern::FirstDayOfPeriod,
                    "First Day",
                );
                ui.selectable_value(
                    &mut state.pattern,
                    RecurrencePattern::LastDayOfPeriod,
                    "Last Day",
                );
                ui.selectable_value(
                    &mut state.pattern,
                    RecurrencePattern::FirstWeekdayOfPeriod(Weekday::Monday),
                    "First Weekday",
                );
                ui.selectable_value(
                    &mut state.pattern,
                    RecurrencePattern::LastWeekdayOfPeriod(Weekday::Monday),
                    "Last Weekday",
                );
            });
    });

    if let Some(mut weekday) = state.pattern.selected_weekday() {
        labeled_row(ui, "", |ui| {
            ui.label("of:");
            egui::ComboBox::from_id_source("weekday_combo")
                .selected_text(weekday.as_str())
                .show_ui(ui, |ui| {
                    for wd in Weekday::all() {
                        ui.selectable_value(&mut weekday, wd, wd.as_str());
                    }
                });
        });
        state.pattern = state.pattern.with_weekday(weekday);
    }
}

fn render_byday_section(
    ui: &mut egui::Ui,
    state: &mut EventDialogState,
    settings: &Settings,
) {
    if !(state.frequency == RecurrenceFrequency::Weekly
        || (state.frequency == RecurrenceFrequency::Monthly
            && state.pattern == RecurrencePattern::None))
    {
        return;
    }

    ui.add_space(4.0);
    indented_row(ui, |ui| {
        ui.checkbox(&mut state.byday_enabled, "Repeat on specific days");
    });

    if !state.byday_enabled {
        return;
    }

    labeled_row(ui, "Days:", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut state.byday_sunday, "Sun");
            ui.checkbox(&mut state.byday_monday, "Mon");
            ui.checkbox(&mut state.byday_tuesday, "Tue");
            ui.checkbox(&mut state.byday_wednesday, "Wed");
            ui.checkbox(&mut state.byday_thursday, "Thu");
            ui.checkbox(&mut state.byday_friday, "Fri");
            ui.checkbox(&mut state.byday_saturday, "Sat");
        });
    });

    if state.frequency == RecurrenceFrequency::Weekly {
        render_weekday_shortcuts(ui, state, settings);
    }
}

fn render_weekday_shortcuts(
    ui: &mut egui::Ui,
    state: &mut EventDialogState,
    settings: &Settings,
) {
    let shortcuts = vec![
        ("First Week Day", settings.first_day_of_week),
        ("Last Week Day", (settings.first_day_of_week + 6) % 7),
        ("First Work Week Day", settings.first_day_of_work_week % 7),
        ("Last Work Week Day", settings.last_day_of_work_week % 7),
    ];

    ui.add_space(6.0);
    labeled_row(ui, "Quick picks:", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, index) in shortcuts {
                let idx = index % 7;
                if let Some(day) = Weekday::from_index(idx) {
                    let mut selected = state.weekday_flag(idx);
                    let checkbox_label = format!("{} ({})", label, day.short_label());
                    if ui.checkbox(&mut selected, checkbox_label).changed() {
                        state.set_weekday_flag(idx, selected);
                    }
                }
            }
        });
    });
}

fn render_recurrence_end_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    ui.add_space(8.0);
    labeled_row(ui, "End condition:", |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let no_end = state.count.is_none() && state.until_date.is_none();
                if ui.radio(no_end, "Never").clicked() {
                    state.count = None;
                    state.until_date = None;
                }
            });

            ui.horizontal(|ui| {
                let has_count = state.count.is_some();
                if ui.radio(has_count, "After").clicked() {
                    state.count = Some(10);
                    state.until_date = None;
                }

                if let Some(ref mut count) = state.count {
                    ui.add(egui::DragValue::new(count).range(1..=999));
                    ui.label("occurrence(s)");
                }
            });

            ui.horizontal(|ui| {
                let has_until = state.until_date.is_some();
                if ui.radio(has_until, "Until").clicked() {
                    state.until_date = Some(state.date + Duration::days(30));
                    state.count = None;
                }

                if let Some(until) = state.until_date {
                    ui.label(until.format("%Y-%m-%d").to_string());
                }
            });
        });
    });
}
