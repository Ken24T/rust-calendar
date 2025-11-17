use super::render::{color32_to_rgba, rgba_to_color32, COUNTDOWN_SETTINGS_MIN_WIDTH};
use crate::services::countdown::{
    CountdownCardId, CountdownCardState, CountdownCardVisuals, RgbaColor,
};
use chrono::{Duration as ChronoDuration, Local, LocalResult, NaiveDate, NaiveTime, TimeZone};
use egui::{self, RichText, ViewportClass};
use egui_extras::DatePickerButton;

const SETTINGS_PANEL_WIDTH: f32 = COUNTDOWN_SETTINGS_MIN_WIDTH;
const SECTION_LABEL_WIDTH: f32 = 140.0;
const FIELD_LABEL_WIDTH: f32 = 170.0;

#[derive(Debug, Clone)]
pub(super) enum CountdownSettingsCommand {
    SetTitleOverride(CountdownCardId, Option<String>),
    SetComment(CountdownCardId, Option<String>),
    SetAlwaysOnTop(CountdownCardId, bool),
    SetCompactMode(CountdownCardId, bool),
    SetDaysFontSize(CountdownCardId, f32),
    SetTitleFontSize(CountdownCardId, f32),
    SetTitleBgColor(CountdownCardId, RgbaColor),
    SetTitleFgColor(CountdownCardId, RgbaColor),
    SetBodyBgColor(CountdownCardId, RgbaColor),
    SetDaysFgColor(CountdownCardId, RgbaColor),
    SetUseDefaultTitleBg(CountdownCardId, bool),
    SetUseDefaultTitleFg(CountdownCardId, bool),
    SetUseDefaultBodyBg(CountdownCardId, bool),
    SetUseDefaultDaysFg(CountdownCardId, bool),
    ApplyVisualDefaults(CountdownCardId),
    DeleteCard(CountdownCardId),
    SetStartAt(CountdownCardId, chrono::DateTime<chrono::Local>),
    SetDefaultTitleBgColor(RgbaColor),
    ResetDefaultTitleBgColor,
    SetDefaultTitleFgColor(RgbaColor),
    ResetDefaultTitleFgColor,
    SetDefaultBodyBgColor(RgbaColor),
    ResetDefaultBodyBgColor,
    SetDefaultDaysFgColor(RgbaColor),
    ResetDefaultDaysFgColor,
    SetDefaultDaysFontSize(f32),
    ResetDefaultDaysFontSize,
    SetDefaultTitleFontSize(f32),
    ResetDefaultTitleFontSize,
}

pub(super) struct CountdownSettingsUiResult {
    pub(super) commands: Vec<CountdownSettingsCommand>,
    pub(super) close_requested: bool,
}

impl CountdownSettingsUiResult {
    pub(super) fn new() -> Self {
        Self {
            commands: Vec::new(),
            close_requested: false,
        }
    }
}

pub(super) fn render_countdown_settings_ui(
    ctx: &egui::Context,
    _class: ViewportClass,
    card: &CountdownCardState,
    defaults: &CountdownCardVisuals,
) -> CountdownSettingsUiResult {
    let mut result = CountdownSettingsUiResult::new();

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.set_width(SETTINGS_PANEL_WIDTH);
                ui.heading(card.effective_title());
                ui.add_space(12.0);

                section_with_label(ui, "Title", |ui| {
                    let mut title_text = card
                        .title_override
                        .clone()
                        .unwrap_or_else(|| card.event_title.clone());
                    field_row(ui, "Countdown title:", |ui| {
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut title_text)
                                    .desired_width(260.0)
                                    .hint_text("Countdown title"),
                            )
                            .changed()
                        {
                            let trimmed = title_text.trim();
                            let payload = if trimmed.is_empty() || trimmed == card.event_title {
                                None
                            } else {
                                Some(trimmed.to_owned())
                            };
                            result
                                .commands
                                .push(CountdownSettingsCommand::SetTitleOverride(card.id, payload));
                        }
                    });
                    field_row(ui, "", |ui| {
                        if ui.button("Reset to event name").clicked() {
                            result
                                .commands
                                .push(CountdownSettingsCommand::SetTitleOverride(card.id, None));
                        }
                    });
                });
                section_divider(ui);

                section_with_label(ui, "Date", |ui| {
                    let mut target_date = card.start_at.date_naive();
                    let date_picker_id = format!("countdown_date_{}", card.id.0);
                    field_row(ui, "Target date:", |ui| {
                        if ui
                            .add(
                                DatePickerButton::new(&mut target_date)
                                    .id_source(date_picker_id.as_str()),
                            )
                            .changed()
                        {
                            let new_dt = combine_date_with_time(target_date, card.start_at.time());
                            result
                                .commands
                                .push(CountdownSettingsCommand::SetStartAt(card.id, new_dt));
                        }
                    });
                });
                section_divider(ui);

                section_with_label(ui, "Layout", |ui| {
                    field_row(ui, "", |ui| {
                        let mut always_on_top = card.visuals.always_on_top;
                        if ui.checkbox(&mut always_on_top, "Always on top").changed() {
                            result
                                .commands
                                .push(CountdownSettingsCommand::SetAlwaysOnTop(
                                    card.id,
                                    always_on_top,
                                ));
                        }
                    });
                    field_row(ui, "", |ui| {
                        let mut compact_mode = card.visuals.compact_mode;
                        if ui.checkbox(&mut compact_mode, "Compact mode").changed() {
                            result
                                .commands
                                .push(CountdownSettingsCommand::SetCompactMode(
                                    card.id,
                                    compact_mode,
                                ));
                        }
                    });
                });
                section_divider(ui);

                section_with_label(ui, "Card Title", |ui| {
                    let mut title_font_size = card.visuals.title_font_size;
                    field_row(ui, "Text size:", |ui| {
                        if ui
                            .add(egui::Slider::new(&mut title_font_size, 12.0..=48.0))
                            .changed()
                        {
                            result
                                .commands
                                .push(CountdownSettingsCommand::SetTitleFontSize(
                                    card.id,
                                    title_font_size,
                                ));
                        }
                    });

                    let mut title_font_default =
                        (title_font_size - defaults.title_font_size).abs() < 0.5;
                    field_row(ui, "", |ui| {
                        if ui
                            .checkbox(&mut title_font_default, "Default card title font size")
                            .changed()
                        {
                            if title_font_default {
                                result.commands.push(
                                    CountdownSettingsCommand::SetDefaultTitleFontSize(
                                        title_font_size,
                                    ),
                                );
                            } else {
                                result
                                    .commands
                                    .push(CountdownSettingsCommand::ResetDefaultTitleFontSize);
                            }
                        }
                    });

                    render_color_setting(
                        ui,
                        card.id,
                        "Card Title Background",
                        card.visuals.title_bg_color,
                        card.visuals.use_default_title_bg,
                        card.event_color,
                        |color| CountdownSettingsCommand::SetTitleBgColor(card.id, color),
                        |flag| CountdownSettingsCommand::SetUseDefaultTitleBg(card.id, flag),
                        |color| CountdownSettingsCommand::SetDefaultTitleBgColor(color),
                        CountdownSettingsCommand::ResetDefaultTitleBgColor,
                        &mut result,
                    );
                    render_color_setting(
                        ui,
                        card.id,
                        "Card Title Text",
                        card.visuals.title_fg_color,
                        card.visuals.use_default_title_fg,
                        card.event_color,
                        |color| CountdownSettingsCommand::SetTitleFgColor(card.id, color),
                        |flag| CountdownSettingsCommand::SetUseDefaultTitleFg(card.id, flag),
                        |color| CountdownSettingsCommand::SetDefaultTitleFgColor(color),
                        CountdownSettingsCommand::ResetDefaultTitleFgColor,
                        &mut result,
                    );
                });
                section_divider(ui);

                section_with_label(ui, "Countdown Display", |ui| {
                    let mut font_size = card.visuals.days_font_size;
                    field_row(ui, "Number size:", |ui| {
                        if ui
                            .add(egui::Slider::new(&mut font_size, 32.0..=220.0))
                            .changed()
                        {
                            result
                                .commands
                                .push(CountdownSettingsCommand::SetDaysFontSize(
                                    card.id, font_size,
                                ));
                        }
                    });

                    let mut font_default = (font_size - defaults.days_font_size).abs() < 0.5;
                    field_row(ui, "", |ui| {
                        if ui
                            .checkbox(&mut font_default, "Default countdown font size")
                            .changed()
                        {
                            if font_default {
                                result.commands.push(
                                    CountdownSettingsCommand::SetDefaultDaysFontSize(font_size),
                                );
                            } else {
                                result
                                    .commands
                                    .push(CountdownSettingsCommand::ResetDefaultDaysFontSize);
                            }
                        }
                    });

                    render_color_setting(
                        ui,
                        card.id,
                        "Countdown Background",
                        card.visuals.body_bg_color,
                        card.visuals.use_default_body_bg,
                        card.event_color,
                        |color| CountdownSettingsCommand::SetBodyBgColor(card.id, color),
                        |flag| CountdownSettingsCommand::SetUseDefaultBodyBg(card.id, flag),
                        |color| CountdownSettingsCommand::SetDefaultBodyBgColor(color),
                        CountdownSettingsCommand::ResetDefaultBodyBgColor,
                        &mut result,
                    );
                    render_color_setting(
                        ui,
                        card.id,
                        "Countdown Text",
                        card.visuals.days_fg_color,
                        card.visuals.use_default_days_fg,
                        card.event_color,
                        |color| CountdownSettingsCommand::SetDaysFgColor(card.id, color),
                        |flag| CountdownSettingsCommand::SetUseDefaultDaysFg(card.id, flag),
                        |color| CountdownSettingsCommand::SetDefaultDaysFgColor(color),
                        CountdownSettingsCommand::ResetDefaultDaysFgColor,
                        &mut result,
                    );
                });
                section_divider(ui);

                section_with_label(ui, "Event Body", |ui| {
                    field_row(ui, "", |ui| {
                        ui.label("Edits here update the event's description.");
                    });
                    let mut comment_text = card.comment.clone().unwrap_or_default();
                    field_row(ui, "", |ui| {
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut comment_text)
                                    .desired_rows(4)
                                    .hint_text("Add notes for this countdown"),
                            )
                            .changed()
                        {
                            let payload = if comment_text.trim().is_empty() {
                                None
                            } else {
                                Some(comment_text.clone())
                            };
                            result
                                .commands
                                .push(CountdownSettingsCommand::SetComment(card.id, payload));
                        }
                    });
                });
                section_divider(ui);

                section_with_label(ui, "Actions", |ui| {
                    field_row(ui, "", |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Reset").clicked() {
                                result
                                    .commands
                                    .push(CountdownSettingsCommand::ApplyVisualDefaults(card.id));
                            }
                            if ui.button("Save").clicked() {
                                result.close_requested = true;
                            }
                            let delete_clicked = ui
                                .add(
                                    egui::Button::new("Delete")
                                        .fill(egui::Color32::from_rgb(185, 28, 28)),
                                )
                                .clicked();
                            if delete_clicked {
                                result
                                    .commands
                                    .push(CountdownSettingsCommand::DeleteCard(card.id));
                                result.close_requested = true;
                            }
                            if ui.button("Cancel").clicked() {
                                result.close_requested = true;
                            }
                        });
                    });
                });
            });
    });

    result
}

fn render_color_setting<F, G, H>(
    ui: &mut egui::Ui,
    card_id: CountdownCardId,
    label: &str,
    color_value: RgbaColor,
    use_default_value: bool,
    event_color: Option<RgbaColor>,
    mut on_color_change: F,
    mut on_use_default_change: G,
    mut on_set_default: H,
    reset_default_command: CountdownSettingsCommand,
    result: &mut CountdownSettingsUiResult,
) where
    F: FnMut(RgbaColor) -> CountdownSettingsCommand,
    G: FnMut(bool) -> CountdownSettingsCommand,
    H: FnMut(RgbaColor) -> CountdownSettingsCommand,
{
    ui.push_id((card_id.0, label), |ui| {
        field_row(ui, label, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let mut default_toggle = use_default_value;
                    if ui
                        .checkbox(&mut default_toggle, "Use default color")
                        .changed()
                    {
                        result.commands.push(on_use_default_change(default_toggle));
                    }
                });

                let mut edited_color = rgba_to_color32(color_value);
                let mut current_value = color_value;
                let picker = ui.add_enabled_ui(!use_default_value, |ui| {
                    let mut color = edited_color;
                    let changed = egui::color_picker::color_edit_button_srgba(
                        ui,
                        &mut color,
                        egui::color_picker::Alpha::Opaque,
                    )
                    .changed();
                    (color, changed)
                });
                let (new_color, changed) = picker.inner;
                if changed {
                    edited_color = new_color;
                    let rgba = color32_to_rgba(edited_color);
                    current_value = rgba;
                    result.commands.push(on_color_change(rgba));
                }

                if !use_default_value && event_color.is_none() {
                    ui.label(
                        RichText::new("No event color available; adjust manually.")
                            .italics()
                            .weak(),
                    );
                }

                ui.horizontal(|ui| {
                    if ui.button("Save as default").clicked() {
                        result.commands.push(on_set_default(current_value));
                    }
                    if ui.button("Reset default").clicked() {
                        result.commands.push(reset_default_command.clone());
                    }
                });
            });
        });
    });
}

fn section_with_label<F>(ui: &mut egui::Ui, title: &str, add_contents: F)
where
    F: FnOnce(&mut egui::Ui),
{
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::Vec2::new(SECTION_LABEL_WIDTH, 24.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.label(RichText::new(title).heading());
            },
        );

        ui.vertical(|ui| {
            add_contents(ui);
        });
    });
}

fn section_divider(ui: &mut egui::Ui) {
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
}

fn field_row<F>(ui: &mut egui::Ui, label: &str, add_contents: F)
where
    F: FnOnce(&mut egui::Ui),
{
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::Vec2::new(FIELD_LABEL_WIDTH, 24.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                if label.is_empty() {
                    ui.label(" ");
                } else {
                    ui.label(label);
                }
            },
        );
        ui.add_space(8.0);
        ui.vertical(|ui| {
            add_contents(ui);
        });
    });
}

fn combine_date_with_time(date: NaiveDate, time: NaiveTime) -> chrono::DateTime<chrono::Local> {
    let mut naive = date.and_time(time);
    for _ in 0..3 {
        match Local.from_local_datetime(&naive) {
            LocalResult::Single(dt) => return dt,
            LocalResult::Ambiguous(dt, _) => return dt,
            LocalResult::None => naive += ChronoDuration::minutes(30),
        }
    }
    Local::now()
}
