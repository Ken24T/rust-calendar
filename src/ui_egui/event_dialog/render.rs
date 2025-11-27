use chrono::{Duration, Local};
use egui::{Color32, RichText};

use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::countdown::CountdownCardId;
use crate::services::database::Database;

use super::recurrence::{RecurrenceFrequency, RecurrencePattern, Weekday};
use super::state::{DatePickerTarget, EventDialogState};
use super::widgets::{parse_hex_color, render_inline_date_picker, render_time_picker, DatePickerAction};

/// Changes to apply to a linked countdown card
#[derive(Debug, Clone)]
pub struct CountdownCardChanges {
    pub card_id: CountdownCardId,
    pub description: Option<String>,
    pub color: Option<String>,
    pub start_date: chrono::NaiveDate,
    pub start_time: chrono::NaiveTime,
    pub always_on_top: bool,
    pub compact_mode: bool,
    pub title_font_size: f32,
    pub days_font_size: f32,
}

/// Request for delete confirmation from the event dialog
#[derive(Clone)]
pub struct EventDeleteRequest {
    pub event_id: i64,
    pub event_title: String,
}

#[derive(Default)]
pub struct EventDialogResult {
    pub saved_event: Option<Event>,
    pub card_changes: Option<CountdownCardChanges>,
    /// Request to show delete confirmation dialog
    pub delete_request: Option<EventDeleteRequest>,
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

    // Check for warnings (overlap detection, etc.) - this updates state.warning_messages
    state.check_warnings(database);

    egui::Window::new(if state.event_id.is_some() {
        "Edit Event"
    } else {
        "New Event"
    })
    .open(&mut dialog_open)
    .collapsible(false)
    .resizable(true)
    .default_width(600.0)
    .default_height(750.0)
    .min_height(720.0)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            render_error_banner(ui, state);
            render_warning_banner(ui, state);
            render_basic_information_section(ui, state);
            render_date_time_section(ui, state);
            render_appearance_section(ui, state);
            render_recurrence_section(ui, state, settings);
            render_countdown_card_section(ui, state);
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

fn render_warning_banner(ui: &mut egui::Ui, state: &EventDialogState) {
    if state.warning_messages.is_empty() {
        return;
    }
    
    let warning_color = Color32::from_rgb(200, 140, 0); // Orange/amber
    
    for warning in &state.warning_messages {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚠").color(warning_color));
            ui.colored_label(warning_color, warning);
        });
    }
    ui.add_space(4.0);
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
        let today = Local::now().date_naive();
        let is_future_event = state.date > today;
        
        indented_row(ui, |ui| {
            ui.add_enabled_ui(is_future_event, |ui| {
                ui.checkbox(
                    &mut state.create_countdown,
                    "Create countdown card after saving",
                )
                .on_hover_text(if is_future_event {
                    "Also spawns a countdown using this event's color once you save."
                } else {
                    "Countdown cards can only be created for future events."
                });
            });
            
            if !is_future_event && state.create_countdown {
                // Auto-uncheck if date changed to today or past
                state.create_countdown = false;
            }
        });
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_date_time_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    let today = Local::now().date_naive();
    
    ui.heading("Date and Time");
    ui.add_space(4.0);

    // Start date section
    labeled_row(ui, "Start date:", |ui| {
        let is_start_picker_open = state.active_date_picker == Some(DatePickerTarget::StartDate);
        let btn_text = state.date.format("%B %d, %Y").to_string();
        
        if ui.selectable_label(is_start_picker_open, format!("📅 {}", btn_text))
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
    });

    // Show inline calendar for start date
    if state.active_date_picker == Some(DatePickerTarget::StartDate) {
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
        let is_end_picker_open = state.active_date_picker == Some(DatePickerTarget::EndDate);
        let btn_text = state.end_date.format("%B %d, %Y").to_string();
        
        if ui.selectable_label(is_end_picker_open, format!("📅 {}", btn_text))
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
    });

    // Show inline calendar for end date  
    if state.active_date_picker == Some(DatePickerTarget::EndDate) {
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

    // Quick date buttons
    indented_row(ui, |ui| {
        if ui.button("Same day").clicked() {
            state.end_date = state.date;
        }
        if ui.button("+1 day").clicked() {
            state.end_date = state.end_date.succ_opt().unwrap_or(state.end_date);
        }
        if ui.button("+1 week").clicked() {
            state.end_date = state.end_date + chrono::Duration::days(7);
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
            
            // Show warning if end time is before start time on the same day
            if state.date == state.end_date && state.end_time <= state.start_time {
                ui.label(RichText::new("⚠").color(Color32::from_rgb(200, 150, 0)));
            }
        });
        
        // Show validation message if times are invalid
        if state.date == state.end_date && state.end_time <= state.start_time {
            indented_row(ui, |ui| {
                ui.label(RichText::new("Note: End time should be after start time")
                    .color(Color32::from_rgb(200, 150, 0))
                    .small());
            });
        }
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

fn render_countdown_card_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    // Only show this section if a countdown card is linked to the event
    let Some(ref mut linked_card) = state.linked_card else {
        return;
    };

    ui.heading("Countdown Card");
    ui.add_space(4.0);

    // Collapsible header
    egui::CollapsingHeader::new("Card Display Settings")
        .default_open(state.show_card_settings)
        .show(ui, |ui| {
            ui.add_space(4.0);

            // Layout options
            indented_row(ui, |ui| {
                if ui
                    .checkbox(&mut linked_card.always_on_top, "Always on top")
                    .changed()
                {
                    linked_card.visuals.always_on_top = linked_card.always_on_top;
                }
            });

            indented_row(ui, |ui| {
                if ui
                    .checkbox(&mut linked_card.compact_mode, "Compact mode")
                    .changed()
                {
                    linked_card.visuals.compact_mode = linked_card.compact_mode;
                }
            });

            ui.add_space(8.0);

            // Font sizes
            labeled_row(ui, "Title font size:", |ui| {
                ui.add(egui::Slider::new(
                    &mut linked_card.visuals.title_font_size,
                    12.0..=48.0,
                ));
            });

            labeled_row(ui, "Countdown font size:", |ui| {
                ui.add(egui::Slider::new(
                    &mut linked_card.visuals.days_font_size,
                    32.0..=220.0,
                ));
            });

            ui.add_space(8.0);
            ui.label(
                RichText::new("Note: Color changes are synced with the event color above.")
                    .small()
                    .weak(),
            );
        });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_action_buttons(
    ui: &mut egui::Ui,
    state: &mut EventDialogState,
    database: &Database,
    show_dialog: &mut bool,
) -> EventDialogResult {
    let mut saved_event = None;
    let mut delete_request = None;

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
                    delete_request = Some(EventDeleteRequest {
                        event_id: id,
                        event_title: state.title.clone(),
                    });
                    *show_dialog = false;
                }
            }
        }
    });

    // Add padding below buttons
    ui.add_space(16.0);

    // Build card changes if there's a linked card and we saved successfully
    let card_changes = if saved_event.is_some() {
        state.linked_card.as_ref().map(|card| CountdownCardChanges {
            card_id: card.card_id,
            description: if state.description.is_empty() {
                None
            } else {
                Some(state.description.clone())
            },
            color: if state.color.is_empty() {
                None
            } else {
                Some(state.color.clone())
            },
            start_date: state.date,
            start_time: state.start_time,
            always_on_top: card.always_on_top,
            compact_mode: card.compact_mode,
            title_font_size: card.visuals.title_font_size,
            days_font_size: card.visuals.days_font_size,
        })
    } else {
        None
    };

    EventDialogResult {
        saved_event,
        card_changes,
        delete_request,
    }
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
