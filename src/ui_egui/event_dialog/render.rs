use chrono::{Duration, Local};
use egui::{Color32, RichText};
use egui_extras::DatePickerButton;

use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;

use super::recurrence::{RecurrenceFrequency, RecurrencePattern, Weekday};
use super::state::EventDialogState;
use super::widgets::{parse_hex_color, render_time_picker};

#[derive(Default)]
pub struct EventDialogResult {
    pub saved_event: Option<Event>,
}

impl EventDialogResult {}

const FORM_LABEL_WIDTH: f32 = 180.0;

pub fn render_event_dialog(
    ctx: &egui::Context,
    state: &mut EventDialogState,
    database: &Database,
    settings: &Settings,
    show_dialog: &mut bool,
) -> EventDialogResult {
    let mut result = EventDialogResult::default();
    let mut dialog_open = *show_dialog;

    egui::Window::new(if state.event_id.is_some() {
        "Edit Event"
    } else {
        "New Event"
    })
    .open(&mut dialog_open)
    .collapsible(false)
    .resizable(true)
    .default_width(600.0)
    .default_height(720.0)
    .min_height(680.0)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            render_error_banner(ui, state);
            render_basic_information_section(ui, state);
            render_date_time_section(ui, state);
            render_appearance_section(ui, state);
            render_recurrence_section(ui, state, settings);
            let action = render_action_buttons(ui, state, database, show_dialog);
            if action.saved_event.is_some() {
                result = action;
            }
        });
    });

    if !dialog_open {
        *show_dialog = false;
    }

    result
}

fn render_error_banner(ui: &mut egui::Ui, state: &EventDialogState) {
    if let Some(ref error) = state.error_message {
        ui.colored_label(Color32::RED, RichText::new(error).strong());
        ui.add_space(8.0);
    }
}

fn render_basic_information_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    ui.heading("Basic Information");
    ui.add_space(4.0);

    labeled_row(
        ui,
        if state.title.trim().is_empty() {
            RichText::new("Title:")
                .strong()
                .color(Color32::from_rgb(255, 150, 150))
        } else {
            RichText::new("Title:").strong()
        },
        |ui| {
            let title_response = ui.text_edit_singleline(&mut state.title);
            ui.label(RichText::new("*").color(Color32::from_rgb(255, 150, 150)));

            if title_response.changed() && state.error_message.is_some() {
                let _ = state.error_message.take();
            }
        },
    );

    labeled_row(ui, "Location:", |ui| {
        ui.text_edit_singleline(&mut state.location);
    });

    labeled_row(ui, "Category:", |ui| {
        ui.text_edit_singleline(&mut state.category);
    });

    labeled_row(ui, "Description:", |ui| {
        let width = ui.available_width();
        ui.add_sized(
            [width, 100.0],
            egui::TextEdit::multiline(&mut state.description),
        );
    });

    if state.event_id.is_none() {
        ui.add_space(8.0);
        indented_row(ui, |ui| {
            ui.checkbox(
                &mut state.create_countdown,
                "Create countdown card after saving",
            )
            .on_hover_text("Also spawns a countdown using this event's color once you save.");
        });
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_date_time_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    ui.heading("Date and Time");
    ui.add_space(4.0);

    labeled_row(ui, "Start date:", |ui| {
        let mut start_date = state.date;
        if ui
            .add(DatePickerButton::new(&mut start_date).id_source("event_start_date"))
            .changed()
        {
            state.date = start_date;
            if state.end_date < state.date {
                state.end_date = state.date;
            }
        }
    });

    indented_row(ui, |ui| {
        if ui.button("< Previous Day").clicked() {
            state.date = state.date.pred_opt().unwrap_or(state.date);
            if state.end_date < state.date {
                state.end_date = state.date;
            }
        }
        if ui.button("Today").clicked() {
            state.date = Local::now().date_naive();
            if state.end_date < state.date {
                state.end_date = state.date;
            }
        }
        if ui.button("Next Day >").clicked() {
            state.date = state.date.succ_opt().unwrap_or(state.date);
            if state.end_date < state.date {
                state.end_date = state.date;
            }
        }
    });

    labeled_row(ui, "End date:", |ui| {
        let mut end_date = state.end_date;
        if ui
            .add(DatePickerButton::new(&mut end_date).id_source("event_end_date"))
            .changed()
        {
            if end_date < state.date {
                end_date = state.date;
            }
            state.end_date = end_date;
        }
    });

    indented_row(ui, |ui| {
        if ui.button("Same as start").clicked() {
            state.end_date = state.date;
        }
        if ui.button("Add day").clicked() {
            state.end_date = state.end_date.succ_opt().unwrap_or(state.end_date);
        }
    });

    ui.add_space(4.0);
    indented_row(ui, |ui| {
        ui.checkbox(&mut state.all_day, "All-day event");
    });
    ui.add_space(4.0);

    if !state.all_day {
        labeled_row(ui, "Start time:", |ui| {
            render_time_picker(ui, &mut state.start_time);
        });

        labeled_row(ui, "End time:", |ui| {
            render_time_picker(ui, &mut state.end_time);
        });
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_appearance_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    ui.heading("Appearance");
    ui.add_space(4.0);

    labeled_row(ui, "Color:", |ui| {
        ui.add(egui::TextEdit::singleline(&mut state.color).desired_width(80.0));

        if let Some(mut color) = parse_hex_color(&state.color) {
            ui.color_edit_button_srgba(&mut color);
            state.color = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
        }
    });

    labeled_row(ui, "Presets:", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (name, hex) in &[
                ("Blue", "#3B82F6"),
                ("Green", "#10B981"),
                ("Red", "#EF4444"),
                ("Yellow", "#F59E0B"),
                ("Purple", "#8B5CF6"),
                ("Pink", "#EC4899"),
            ] {
                if ui.button(*name).clicked() {
                    state.color = hex.to_string();
                }
            }
        });
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_recurrence_section(ui: &mut egui::Ui, state: &mut EventDialogState, settings: &Settings) {
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
                ui.selectable_value(&mut state.frequency, RecurrenceFrequency::Weekly, "Weekly");
                ui.selectable_value(
                    &mut state.frequency,
                    RecurrenceFrequency::Monthly,
                    "Monthly",
                );
                ui.selectable_value(&mut state.frequency, RecurrenceFrequency::Yearly, "Yearly");
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

fn render_byday_section(ui: &mut egui::Ui, state: &mut EventDialogState, settings: &Settings) {
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

fn render_weekday_shortcuts(ui: &mut egui::Ui, state: &mut EventDialogState, settings: &Settings) {
    let mut shortcuts = Vec::new();
    shortcuts.push(("First Week Day", settings.first_day_of_week));
    shortcuts.push(("Last Week Day", (settings.first_day_of_week + 6) % 7));
    shortcuts.push(("First Work Week Day", settings.first_day_of_work_week % 7));
    shortcuts.push(("Last Work Week Day", settings.last_day_of_work_week % 7));

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

fn render_action_buttons(
    ui: &mut egui::Ui,
    state: &mut EventDialogState,
    database: &Database,
    show_dialog: &mut bool,
) -> EventDialogResult {
    let mut saved_event = None;

    indented_row(ui, |ui| {
        let can_save = !state.title.trim().is_empty();
        let save_button = egui::Button::new("Save").fill(if can_save {
            Color32::from_rgb(70, 120, 200)
        } else {
            Color32::from_gray(60)
        });

        ui.add_enabled_ui(can_save, |ui| {
            if ui.add(save_button).clicked() {
                match state.save(database) {
                    Ok(event) => {
                        saved_event = Some(event);
                        *show_dialog = false;
                    }
                    Err(e) => {
                        state.error_message = Some(e);
                    }
                }
            }
        });

        if !can_save {
            ui.label(
                RichText::new("(Title required)")
                    .small()
                    .color(Color32::from_gray(150)),
            );
        }

        if ui.button("Cancel").clicked() {
            *show_dialog = false;
        }

        if state.event_id.is_some() {
            ui.add_space(20.0);
            if ui
                .button(RichText::new("Delete").color(Color32::RED))
                .clicked()
            {
                if let Some(id) = state.event_id {
                    let service = EventService::new(database.connection());
                    if let Err(e) = service.delete(id) {
                        state.error_message = Some(format!("Failed to delete: {}", e));
                    } else {
                        *show_dialog = false;
                    }
                }
            }
        }
    });

    EventDialogResult { saved_event }
}

fn labeled_row<F>(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, add_contents: F)
where
    F: FnOnce(&mut egui::Ui),
{
    ui.horizontal(|ui| {
        render_form_label(ui, label);
        add_contents(ui);
    });
}

fn render_form_label(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>) {
    let text = label.into();
    ui.allocate_ui_with_layout(
        egui::Vec2::new(FORM_LABEL_WIDTH, 24.0),
        egui::Layout::right_to_left(egui::Align::Center),
        move |ui| {
            ui.label(text);
        },
    );
}

fn indented_row<F>(ui: &mut egui::Ui, add_contents: F)
where
    F: FnOnce(&mut egui::Ui),
{
    ui.horizontal(|ui| {
        ui.add_space(FORM_LABEL_WIDTH);
        add_contents(ui);
    });
}
