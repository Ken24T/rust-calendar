//! Rendering logic for individual countdown cards within the container.

use crate::services::countdown::{
    CountdownCardState, CountdownCardVisuals, CountdownCategoryId,
    CountdownNotificationConfig, CountdownWarningState, RgbaColor, MAX_DAYS_FONT_SIZE,
};
use chrono::{DateTime, Local};

// Card rendering constants
const CARD_ROUNDING: f32 = 8.0;
const CARD_MIN_COUNTDOWN_HEIGHT: f32 = 36.0;
const CARD_SPACING: f32 = 4.0;

/// Format the detailed countdown tooltip for a card
pub fn format_card_tooltip(card: &CountdownCardState, now: DateTime<Local>) -> String {
    let mut lines = Vec::new();

    // Event date range if available
    if let (Some(start), Some(end)) = (card.event_start, card.event_end) {
        let start_str = start.format("%d %b %Y %H:%M").to_string();
        let end_str = if start.date_naive() == end.date_naive() {
            // Same day - just show time for end
            end.format("%H:%M").to_string()
        } else {
            end.format("%d %b %Y %H:%M").to_string()
        };
        lines.push(format!("📅 {} - {}", start_str, end_str));
    }

    // Detailed countdown (DD:HH:MM)
    let duration = card.start_at.signed_duration_since(now);
    if duration.num_seconds() > 0 {
        let total_seconds = duration.num_seconds();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;

        if days > 0 {
            lines.push(format!("⏱ {}d {:02}h {:02}m remaining", days, hours, minutes));
        } else if hours > 0 {
            lines.push(format!("⏱ {:02}h {:02}m remaining", hours, minutes));
        } else {
            lines.push(format!("⏱ {:02}m remaining", minutes));
        }
    } else {
        lines.push("⏱ Event has started!".to_string());
    }

    // Target time
    lines.push(format!("🎯 Target: {}", card.start_at.format("%d %b %Y %H:%M")));

    // Comment/description if present
    if let Some(body) = card.comment.as_ref().map(|t| t.trim()).filter(|t| !t.is_empty()) {
        lines.push(String::new()); // blank line
        lines.push(format!("📝 {}", body));
    }

    lines.join("\n")
}

/// Action from rendering a single card within the container
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardUiAction {
    None,
    OpenSettings,
    OpenEventDialog,
    GoToDate,
    Delete,
    Refresh,
    ChangeCategory(CountdownCategoryId),
}

/// Convert an RGBA color to egui Color32
fn rgba_to_color32(color: RgbaColor) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

/// Resolve effective color: use default if flag is set, otherwise use card's value
fn resolve_color(
    card_color: RgbaColor,
    default_color: RgbaColor,
    use_default: bool,
) -> RgbaColor {
    if use_default {
        default_color
    } else {
        card_color
    }
}

/// Calculate warning colors for a card based on its state
fn calculate_card_colors(
    card: &CountdownCardState,
    visual_defaults: &CountdownCardVisuals,
    warning_state: CountdownWarningState,
    notification_config: &CountdownNotificationConfig,
    ctx: &egui::Context,
) -> (egui::Color32, egui::Color32, egui::Color32, egui::Color32, f32) {
    // Resolve effective colors using defaults when flags are set
    let effective_title_bg = resolve_color(
        card.visuals.title_bg_color,
        visual_defaults.title_bg_color,
        card.visuals.use_default_title_bg,
    );
    let effective_title_fg = resolve_color(
        card.visuals.title_fg_color,
        visual_defaults.title_fg_color,
        card.visuals.use_default_title_fg,
    );
    let effective_body_bg = resolve_color(
        card.visuals.body_bg_color,
        visual_defaults.body_bg_color,
        card.visuals.use_default_body_bg,
    );
    let effective_days_fg = resolve_color(
        card.visuals.days_fg_color,
        visual_defaults.days_fg_color,
        card.visuals.use_default_days_fg,
    );

    let title_bg = rgba_to_color32(effective_title_bg);
    let title_fg = rgba_to_color32(effective_title_fg);

    if !notification_config.enabled || !notification_config.use_visual_warnings {
        let body_bg = rgba_to_color32(effective_body_bg);
        let days_fg = rgba_to_color32(effective_days_fg);
        return (title_bg, title_fg, body_bg, days_fg, 1.0);
    }

    let (body_bg, days_fg, stroke_width) = match warning_state {
        CountdownWarningState::Critical => {
            let pulse_phase = (ctx.input(|i| i.time) * 2.0) % 1.0;
            let pulse_alpha = (pulse_phase * 255.0) as u8;
            let body_bg =
                egui::Color32::from_rgba_unmultiplied(255, 100, 100, 255 - pulse_alpha / 2);
            let days_fg = egui::Color32::from_rgb(139, 0, 0);
            ctx.request_repaint();
            (body_bg, days_fg, 4.0)
        }
        CountdownWarningState::Imminent => {
            let body_bg = egui::Color32::from_rgb(255, 165, 0);
            let days_fg = egui::Color32::from_rgb(139, 69, 0);
            (body_bg, days_fg, 3.0)
        }
        CountdownWarningState::Starting => {
            let pulse_phase = (ctx.input(|i| i.time) * 3.0) % 1.0;
            let pulse_alpha = (pulse_phase * 255.0) as u8;
            let body_bg = egui::Color32::from_rgba_unmultiplied(0, 255, 100, 255 - pulse_alpha / 3);
            let days_fg = egui::Color32::from_rgb(0, 100, 0);
            ctx.request_repaint();
            (body_bg, days_fg, 5.0)
        }
        CountdownWarningState::Approaching => {
            let body_bg = rgba_to_color32(effective_body_bg);
            let days_fg = rgba_to_color32(effective_days_fg);
            (body_bg, days_fg, 2.0)
        }
        CountdownWarningState::Normal => {
            let body_bg = rgba_to_color32(effective_body_bg);
            let days_fg = rgba_to_color32(effective_days_fg);
            (body_bg, days_fg, 1.0)
        }
    };

    (title_bg, title_fg, body_bg, days_fg, stroke_width)
}

/// Render a single card's content within a given rect.
/// This is used by the container to render each card at its computed position.
#[allow(clippy::too_many_arguments)]
pub fn render_card_content(
    ui: &mut egui::Ui,
    card: &CountdownCardState,
    visual_defaults: &CountdownCardVisuals,
    rect: egui::Rect,
    now: DateTime<Local>,
    notification_config: &CountdownNotificationConfig,
    is_being_dragged: bool,
    categories: &[(CountdownCategoryId, String)],
) -> CardUiAction {
    let mut action = CardUiAction::None;

    // Calculate warning state
    let warning_state = if notification_config.enabled && notification_config.use_visual_warnings {
        card.warning_state(now, &notification_config.warning_thresholds)
    } else {
        CountdownWarningState::Normal
    };

    // Get colors (resolving defaults as needed)
    let (title_bg, title_fg, body_bg, days_fg, stroke_width) =
        calculate_card_colors(card, visual_defaults, warning_state, notification_config, ui.ctx());

    let title_font_size = card.visuals.title_font_size.max(12.0);
    let font_size = card.visuals.days_font_size.clamp(32.0, MAX_DAYS_FONT_SIZE);

    // Calculate stroke color based on warning state
    let stroke_color = if is_being_dragged {
        egui::Color32::from_rgb(100, 149, 237) // Cornflower blue for drag indicator
    } else {
        match warning_state {
            CountdownWarningState::Critical => egui::Color32::from_rgb(200, 0, 0),
            CountdownWarningState::Imminent => egui::Color32::from_rgb(255, 140, 0),
            CountdownWarningState::Starting => egui::Color32::from_rgb(0, 200, 0),
            CountdownWarningState::Approaching => egui::Color32::from_rgb(255, 200, 0),
            CountdownWarningState::Normal => egui::Color32::from_gray(40),
        }
    };

    let actual_stroke_width = if is_being_dragged { 3.0 } else { stroke_width };

    // Allocate the rect for this card
    let child_ui = ui.child_ui(rect, egui::Layout::top_down(egui::Align::LEFT), None);
    let mut child_ui = child_ui;

    let rounding = egui::Rounding::from(CARD_ROUNDING);
    let frame = egui::Frame::none()
        .fill(body_bg)
        .rounding(rounding)
        .stroke(egui::Stroke::new(actual_stroke_width, stroke_color));

    let inner = frame.show(&mut child_ui, |ui| {
        let width = rect.width();
        let total_height = rect.height().max(60.0);

        let desired_title_height = (title_font_size * 1.4).clamp(22.0, 48.0);
        let max_title_height = (total_height - CARD_MIN_COUNTDOWN_HEIGHT - CARD_SPACING).max(20.0);
        let title_height = desired_title_height.min(max_title_height);
        let countdown_height = (total_height - title_height - CARD_SPACING).max(CARD_MIN_COUNTDOWN_HEIGHT);

        // Title bar
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

        ui.add_space(CARD_SPACING);

        // Countdown number
        let countdown_size = egui::vec2(width, countdown_height);
        ui.allocate_ui_with_layout(
            countdown_size,
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |countdown_ui| {
                let duration = card.start_at.signed_duration_since(now);
                let total_hours = duration.num_hours();

                // Show HH:MM if less than 24 hours, otherwise show days
                let countdown_text = if (0..24).contains(&total_hours) {
                    let hours = total_hours;
                    let minutes = (duration.num_minutes() % 60).max(0);
                    format!("{:02}:{:02}", hours, minutes)
                } else if total_hours < 0 {
                    // Event has passed
                    "00:00".to_string()
                } else {
                    let days_remaining = (card.start_at.date_naive() - now.date_naive())
                        .num_days()
                        .max(0);
                    days_remaining.to_string()
                };

                // Calculate font size based on available space and number of characters
                let char_count = countdown_text.len();
                let available_width = width * 0.9;
                let estimated_text_width = font_size * 0.6 * char_count as f32;

                let adjusted_font_size = if estimated_text_width > available_width {
                    (available_width / (0.6 * char_count as f32))
                        .max(32.0)
                        .min(font_size)
                } else {
                    font_size
                };

                let countdown_response = countdown_ui.label(
                    egui::RichText::new(countdown_text)
                        .size(adjusted_font_size)
                        .color(days_fg),
                );

                // Enhanced tooltip with event details and countdown
                countdown_response.on_hover_ui_at_pointer(|ui| {
                    ui.label(format_card_tooltip(card, now));
                });
            },
        );
    });

    // Context menu for the card
    inner.response.context_menu(|ui| {
        if card.event_id.is_some()
            && ui.button("📝 Edit event...").clicked() {
                action = CardUiAction::OpenEventDialog;
                ui.close_menu();
            }
        if ui.button("⚙ Card settings...").clicked() {
            action = CardUiAction::OpenSettings;
            ui.close_menu();
        }
        if ui.button("📅 Go to date").clicked() {
            action = CardUiAction::GoToDate;
            ui.close_menu();
        }
        if ui.button("🔄 Refresh countdown").clicked() {
            action = CardUiAction::Refresh;
            ui.close_menu();
        }
        if categories.len() > 1 {
            ui.menu_button("📂 Move to category", |ui| {
                for (cat_id, cat_name) in categories {
                    if *cat_id == card.category_id {
                        ui.label(egui::RichText::new(format!("✓ {cat_name}")).strong());
                    } else if ui.button(cat_name).clicked() {
                        action = CardUiAction::ChangeCategory(*cat_id);
                        ui.close_menu();
                    }
                }
            });
        }
        ui.separator();
        if ui.button("🗑 Delete card").clicked() {
            action = CardUiAction::Delete;
            ui.close_menu();
        }
    });

    action
}
