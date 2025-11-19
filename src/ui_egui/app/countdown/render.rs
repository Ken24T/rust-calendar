use super::super::{geometry_changed, geometry_from_viewport_info};
use crate::services::countdown::{
    CountdownCardGeometry, CountdownCardState, CountdownNotificationConfig, CountdownWarningState,
    RgbaColor, MAX_DAYS_FONT_SIZE,
};
use chrono::{DateTime, Local};
use egui::{self, ViewportClass, ViewportId};
use std::time::Duration as StdDuration;

pub(super) const COUNTDOWN_SETTINGS_HEIGHT: f32 = 870.0;
pub(super) const COUNTDOWN_SETTINGS_MIN_WIDTH: f32 = 640.0;
const CARD_MIN_WIDTH: f32 = 20.0;
const CARD_MIN_HEIGHT: f32 = 20.0;

// Guard against infinite/invalid layout hints that can surface while a viewport is initializing.
fn resolve_dimension(value: f32, fallback: f32, min: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value.max(min)
    } else {
        fallback.max(min)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CountdownCardUiAction {
    None,
    OpenSettings,
    GeometrySettled,
    Delete,
    Refresh,
}

pub(super) fn viewport_builder_for_card(
    card: &CountdownCardState,
    waiting_on_geometry: bool,
) -> egui::ViewportBuilder {
    let mut builder = egui::ViewportBuilder::default()
        .with_title(card.event_title.clone())
        .with_resizable(true)
        .with_transparent(false)
        .with_decorations(true)
        .with_min_inner_size(egui::vec2(CARD_MIN_WIDTH, CARD_MIN_HEIGHT))
        .with_position(egui::pos2(card.geometry.x, card.geometry.y))
        .with_inner_size(egui::vec2(
            card.geometry.width.max(CARD_MIN_WIDTH),
            card.geometry.height.max(CARD_MIN_HEIGHT),
        ))
        // Disable egui's automatic viewport state persistence
        .with_window_level(egui::WindowLevel::Normal);

    if waiting_on_geometry {
        builder = builder.with_visible(false);
    }

    if card.visuals.always_on_top {
        builder = builder.with_always_on_top();
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
            400.0,
        ));

    if let Some(geometry) = geometry {
        builder = builder
            .with_position(egui::pos2(geometry.x, geometry.y))
            .with_inner_size(egui::vec2(
                geometry.width.max(COUNTDOWN_SETTINGS_MIN_WIDTH),
                geometry.height.max(400.0).min(COUNTDOWN_SETTINGS_HEIGHT),
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
    target_geometry: Option<CountdownCardGeometry>,
    notification_config: &CountdownNotificationConfig,
) -> CountdownCardUiAction {
    ctx.request_repaint_after(StdDuration::from_secs(1));
    
    // Enforce size while geometry is still being set up (target_geometry present)
    // Once geometry settles (target_geometry becomes None), allow user to resize freely
    if target_geometry.is_some() {
        ctx.send_viewport_cmd_to(
            viewport_id,
            egui::ViewportCommand::InnerSize(egui::vec2(
                card.geometry.width.max(CARD_MIN_WIDTH),
                card.geometry.height.max(CARD_MIN_HEIGHT),
            )),
        );
    }
    
    ctx.send_viewport_cmd_to(
        viewport_id,
        egui::ViewportCommand::EnableButtons {
            close: true,
            minimized: false,
            maximize: false,
        },
    );

    // Calculate warning state for visual feedback
    let warning_state = if notification_config.enabled && notification_config.use_visual_warnings {
        card.warning_state(now, &notification_config.warning_thresholds)
    } else {
        CountdownWarningState::Normal
    };

    // Base colors from card visuals
    let title_bg = rgba_to_color32(card.visuals.title_bg_color);
    let title_fg = rgba_to_color32(card.visuals.title_fg_color);
    let title_font_size = card.visuals.title_font_size.max(12.0);
    
    // Apply warning state color overrides if enabled
    let (body_bg, days_fg, stroke_color, stroke_width) = if notification_config.enabled
        && notification_config.use_visual_warnings
    {
        apply_warning_colors(warning_state, card, ctx)
    } else {
        (
            rgba_to_color32(card.visuals.body_bg_color),
            rgba_to_color32(card.visuals.days_fg_color),
            egui::Color32::from_gray(40),
            1.0,
        )
    };
    
    let font_size = card
        .visuals
        .days_font_size
        .clamp(32.0, MAX_DAYS_FONT_SIZE);

    let mut geometry_settled = false;
    if let Some(target) = target_geometry {
        let target_position = egui::pos2(target.x, target.y);
        let target_size = egui::vec2(target.width, target.height);
        ctx.send_viewport_cmd_to(
            viewport_id,
            egui::ViewportCommand::OuterPosition(target_position),
        );
        ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::InnerSize(target_size));

        geometry_settled = ctx.input(|input| {
            let info = input.viewport();
            geometry_from_viewport_info(info)
                .map(|current| !geometry_changed(target, current))
                .unwrap_or(false)
        });

        ctx.input(|input| {
            if let Some(current) = geometry_from_viewport_info(input.viewport()) {
                log::debug!(
                    "card {:?} target geometry {:?}, current viewport geometry {:?}",
                    card.id,
                    target,
                    current
                );
            }
        });

        if geometry_settled {
            log::debug!("card {:?} geometry settled at {:?}", card.id, target);
            ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
        } else if waiting_on_geometry {
            log::debug!(
                "card {:?} geometry still settling; keeping viewport hidden",
                card.id
            );
            ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(false));
        } else {
            ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
        }
    } else if !waiting_on_geometry {
        ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
    }

    struct RenderResult {
        action: CountdownCardUiAction,
    }

    let render = |ui: &mut egui::Ui| {
        let mut action = CountdownCardUiAction::None;
        let rounding = egui::Rounding::from(8.0);
        let frame = egui::Frame::none()
            .fill(body_bg)
            .rounding(rounding)
            .stroke(egui::Stroke::new(stroke_width, stroke_color));

        let inner = frame.show(ui, |ui| {
            let available = ui.available_size();
            let width = resolve_dimension(available.x, card.geometry.width, CARD_MIN_WIDTH);
            let total_height = resolve_dimension(available.y, card.geometry.height, CARD_MIN_HEIGHT).max(60.0);
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
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(card.effective_title())
                                            .color(title_fg)
                                            .size(title_font_size)
                                            .strong(),
                                    )
                                    .truncate()
                                    .wrap_mode(egui::TextWrapMode::Truncate),
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
                    
                    let days_text = days_remaining.to_string();
                    
                    // Calculate font size based on available space and number of digits
                    let digit_count = days_text.len();
                    let available_width = width * 0.9; // Leave 10% margin
                    
                    // Estimate width per character (roughly 0.6 of font size for monospace digits)
                    let estimated_text_width = font_size * 0.6 * digit_count as f32;
                    
                    let adjusted_font_size = if estimated_text_width > available_width {
                        // Scale down to fit available width
                        (available_width / (0.6 * digit_count as f32)).max(32.0).min(font_size)
                    } else {
                        font_size
                    };
                    
                    let countdown_response = countdown_ui.label(
                        egui::RichText::new(days_text)
                            .size(adjusted_font_size)
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

        inner.response.context_menu(|ui| {
            if ui.button("Card settings...").clicked() {
                action = CountdownCardUiAction::OpenSettings;
                ui.close_menu();
            }
            if ui.button("Refresh countdown").clicked() {
                action = CountdownCardUiAction::Refresh;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Delete card").clicked() {
                action = CountdownCardUiAction::Delete;
                ui.close_menu();
            }
        });

        RenderResult { action }
    };

    let result = match class {
        ViewportClass::Embedded => {
            let mut action = CountdownCardUiAction::None;
            egui::Window::new(card.effective_title())
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    action = render(ui).action;
                });
            action
        }
        _ => {
            let mut output: Option<RenderResult> = None;
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(body_bg))
                .show(ctx, |ui| {
                    // Force the UI to respect the card's target geometry
                    ui.set_min_size(egui::vec2(card.geometry.width, card.geometry.height));
                    output = Some(render(ui));
                });

            output
                .map(|outcome| outcome.action)
                .unwrap_or(CountdownCardUiAction::None)
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

/// Apply visual warning colors based on countdown state.
/// Returns (body_bg, days_fg, stroke_color, stroke_width)
fn apply_warning_colors(
    warning_state: CountdownWarningState,
    card: &CountdownCardState,
    ctx: &egui::Context,
) -> (egui::Color32, egui::Color32, egui::Color32, f32) {
    match warning_state {
        CountdownWarningState::Critical => {
            // Critical: Red/orange with pulsing effect
            let pulse_phase = (ctx.input(|i| i.time) * 2.0) % 1.0; // 2 Hz pulse
            let pulse_alpha = (pulse_phase * 255.0) as u8;
            let body_bg = egui::Color32::from_rgba_unmultiplied(255, 100, 100, 255 - pulse_alpha / 2);
            let days_fg = egui::Color32::from_rgb(139, 0, 0); // Dark red
            let stroke_color = egui::Color32::from_rgb(200, 0, 0);
            ctx.request_repaint(); // Continuous animation
            (body_bg, days_fg, stroke_color, 4.0)
        }
        CountdownWarningState::Imminent => {
            // Imminent: Orange warning
            let body_bg = egui::Color32::from_rgb(255, 165, 0);
            let days_fg = egui::Color32::from_rgb(139, 69, 0);
            let stroke_color = egui::Color32::from_rgb(255, 140, 0);
            (body_bg, days_fg, stroke_color, 3.0)
        }
        CountdownWarningState::Starting => {
            // Starting: Bright green with pulsing
            let pulse_phase = (ctx.input(|i| i.time) * 3.0) % 1.0; // 3 Hz fast pulse
            let pulse_alpha = (pulse_phase * 255.0) as u8;
            let body_bg = egui::Color32::from_rgba_unmultiplied(0, 255, 100, 255 - pulse_alpha / 3);
            let days_fg = egui::Color32::from_rgb(0, 100, 0);
            let stroke_color = egui::Color32::from_rgb(0, 200, 0);
            ctx.request_repaint(); // Continuous animation
            (body_bg, days_fg, stroke_color, 5.0)
        }
        CountdownWarningState::Approaching => {
            // Approaching: Slight yellow tint
            let body_bg = rgba_to_color32(card.visuals.body_bg_color);
            let days_fg = rgba_to_color32(card.visuals.days_fg_color);
            let stroke_color = egui::Color32::from_rgb(255, 200, 0);
            (body_bg, days_fg, stroke_color, 2.0)
        }
        CountdownWarningState::Normal => {
            // Normal: Use card's configured colors
            let body_bg = rgba_to_color32(card.visuals.body_bg_color);
            let days_fg = rgba_to_color32(card.visuals.days_fg_color);
            let stroke_color = egui::Color32::from_gray(40);
            (body_bg, days_fg, stroke_color, 1.0)
        }
    }
}
