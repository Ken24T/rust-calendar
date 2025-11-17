use super::super::{geometry_changed, geometry_from_viewport_info};
use crate::services::countdown::{CountdownCardGeometry, CountdownCardState, RgbaColor};
use chrono::{DateTime, Local};
use egui::{self, ViewportClass, ViewportId};
use std::time::Duration as StdDuration;

pub(super) const COUNTDOWN_SETTINGS_HEIGHT: f32 = 1000.0;
pub(super) const COUNTDOWN_SETTINGS_MIN_WIDTH: f32 = 640.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CountdownCardUiAction {
    None,
    Close,
    OpenSettings,
    GeometrySettled,
}

pub(super) fn viewport_builder_for_card(
    card: &CountdownCardState,
    start_hidden: bool,
) -> egui::ViewportBuilder {
    let mut builder = egui::ViewportBuilder::default()
        .with_title(card.event_title.clone())
        .with_position(egui::pos2(card.geometry.x, card.geometry.y))
        .with_inner_size(egui::vec2(
            card.geometry.width.max(110.0),
            card.geometry.height.max(90.0),
        ))
        .with_resizable(true)
        .with_transparent(false);

    if card.visuals.always_on_top {
        builder = builder.with_always_on_top();
    }

    if start_hidden {
        builder = builder.with_visible(false);
    }

    builder
}

pub(super) fn viewport_builder_for_settings(
    geometry: Option<CountdownCardGeometry>,
    card: &CountdownCardState,
) -> egui::ViewportBuilder {
    let mut builder = egui::ViewportBuilder::default()
        .with_title(format!("Settings: {}", card.effective_title()))
        .with_resizable(true)
        .with_min_inner_size(egui::vec2(
            COUNTDOWN_SETTINGS_MIN_WIDTH,
            COUNTDOWN_SETTINGS_HEIGHT,
        ));

    if let Some(geometry) = geometry {
        builder = builder
            .with_position(egui::pos2(geometry.x, geometry.y))
            .with_inner_size(egui::vec2(
                geometry.width.max(COUNTDOWN_SETTINGS_MIN_WIDTH),
                COUNTDOWN_SETTINGS_HEIGHT,
            ));
    } else {
        builder = builder.with_inner_size(egui::vec2(
            COUNTDOWN_SETTINGS_MIN_WIDTH,
            COUNTDOWN_SETTINGS_HEIGHT,
        ));
    }

    builder
}

pub(super) fn render_countdown_card_ui(
    ctx: &egui::Context,
    class: ViewportClass,
    viewport_id: ViewportId,
    card: &CountdownCardState,
    now: DateTime<Local>,
    waiting_on_geometry: bool,
) -> CountdownCardUiAction {
    ctx.request_repaint_after(StdDuration::from_secs(1));

    if !waiting_on_geometry {
        ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
    }

    let title_bg = rgba_to_color32(card.visuals.title_bg_color);
    let title_fg = rgba_to_color32(card.visuals.title_fg_color);
    let title_font_size = card.visuals.title_font_size.max(12.0);
    let body_bg = rgba_to_color32(card.visuals.body_bg_color);
    let days_fg = rgba_to_color32(card.visuals.days_fg_color);
    let font_size = card.visuals.days_font_size.max(32.0);

    let mut geometry_settled = false;
    if waiting_on_geometry {
        let target_position = egui::pos2(card.geometry.x, card.geometry.y);
        let target_size = egui::vec2(card.geometry.width, card.geometry.height);
        ctx.send_viewport_cmd_to(
            viewport_id,
            egui::ViewportCommand::OuterPosition(target_position),
        );
        ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::InnerSize(target_size));
        log::debug!(
            "card {:?} forcing position {:?} and size {:?}",
            card.id,
            target_position,
            target_size
        );

        geometry_settled = ctx.input(|input| {
            let info = input.viewport();
            geometry_from_viewport_info(info)
                .map(|current| !geometry_changed(card.geometry, current))
                .unwrap_or(false)
        });

        ctx.input(|input| {
            if let Some(current) = geometry_from_viewport_info(input.viewport()) {
                log::debug!(
                    "card {:?} current viewport geometry: {:?}",
                    card.id,
                    current
                );
            }
        });

        if geometry_settled {
            log::debug!("card {:?} geometry settled", card.id);
            ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
        } else {
            ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(false));
        }
    }

    let render = |ui: &mut egui::Ui| {
        let mut action = CountdownCardUiAction::None;
        let rounding = egui::Rounding::from(8.0);
        let frame = egui::Frame::none()
            .fill(body_bg)
            .rounding(rounding)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(40)));

        let inner = frame.show(ui, |ui| {
            let available = ui.available_size();
            ui.set_min_size(available);
            let total_height = available.y.max(60.0);
            let width = available.x;
            let spacing = 4.0;
            let min_countdown_height = 36.0;
            let desired_title_height = (title_font_size * 1.4).clamp(22.0, 48.0);
            let max_title_height = (total_height - min_countdown_height - spacing).max(20.0);
            let title_height = desired_title_height.min(max_title_height);
            let countdown_height =
                (total_height - title_height - spacing).max(min_countdown_height);

            let title_size = egui::vec2(width, title_height);
            ui.allocate_ui_with_layout(
                title_size,
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |title_ui| {
                    egui::Frame::none()
                        .fill(title_bg)
                        .rounding(egui::Rounding {
                            nw: rounding.nw,
                            ne: rounding.ne,
                            sw: 0.0,
                            se: 0.0,
                        })
                        .show(title_ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    egui::RichText::new(card.effective_title())
                                        .color(title_fg)
                                        .size(title_font_size)
                                        .strong(),
                                );
                            });
                        });
                },
            );

            ui.add_space(spacing);

            let countdown_size = egui::vec2(width, countdown_height);
            ui.allocate_ui_with_layout(
                countdown_size,
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |countdown_ui| {
                    let days_remaining = (card.start_at.date_naive() - now.date_naive())
                        .num_days()
                        .max(0);
                    let countdown_response = countdown_ui.label(
                        egui::RichText::new(days_remaining.to_string())
                            .size(font_size)
                            .color(days_fg),
                    );

                    if let Some(body) = card
                        .comment
                        .as_ref()
                        .map(|text| text.trim())
                        .filter(|text| !text.is_empty())
                    {
                        countdown_response.on_hover_ui_at_pointer(|ui| {
                            ui.label(body);
                        });
                    }
                },
            );
        });

        let response = ui.interact(
            inner.response.rect,
            ui.make_persistent_id(("countdown_card_surface", card.id.0)),
            egui::Sense::click(),
        );
        response.context_menu(|ui| {
            if ui.button("Card settings...").clicked() {
                action = CountdownCardUiAction::OpenSettings;
                ui.close_menu();
            }
            if ui.button("Close countdown").clicked() {
                action = CountdownCardUiAction::Close;
                ui.close_menu();
            }
        });

        action
    };

    let result = match class {
        ViewportClass::Embedded => {
            let mut action = CountdownCardUiAction::None;
            egui::Window::new(card.effective_title())
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    action = render(ui);
                });
            action
        }
        _ => {
            let mut action = CountdownCardUiAction::None;
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(body_bg))
                .show(ctx, |ui| {
                    action = render(ui);
                });
            action
        }
    };

    if geometry_settled {
        CountdownCardUiAction::GeometrySettled
    } else {
        result
    }
}

pub(super) fn viewport_title_matches(info: &egui::ViewportInfo, expected: &str) -> bool {
    match info.title.as_deref() {
        Some(title) => title == expected,
        None => true,
    }
}

pub(super) fn rgba_to_color32(color: RgbaColor) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

pub(super) fn color32_to_rgba(color: egui::Color32) -> RgbaColor {
    RgbaColor {
        r: color.r(),
        g: color.g(),
        b: color.b(),
        a: color.a(),
    }
}
