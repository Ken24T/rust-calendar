//! Toast notification system for brief feedback messages.
//!
//! Toasts are non-blocking notifications that appear briefly and fade away.
//! They're used for action confirmations like "Event saved", "Backup created", etc.

// Allow unused variants/methods - these are API surface for future use
#![allow(dead_code)]

use egui::{Color32, Context, Pos2, RichText};
use std::time::{Duration, Instant};

/// Types of toast notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    /// Success message (green)
    Success,
    /// Informational message (blue)
    Info,
    /// Warning message (orange)
    Warning,
    /// Error message (red)
    Error,
}

impl ToastLevel {
    /// Get the icon for this toast level
    pub fn icon(&self) -> &'static str {
        match self {
            ToastLevel::Success => "✓",
            ToastLevel::Info => "ℹ",
            ToastLevel::Warning => "⚠",
            ToastLevel::Error => "✗",
        }
    }

    /// Get the background color for this toast level
    pub fn background_color(&self, is_dark_theme: bool) -> Color32 {
        if is_dark_theme {
            match self {
                ToastLevel::Success => Color32::from_rgb(30, 70, 40),
                ToastLevel::Info => Color32::from_rgb(30, 50, 80),
                ToastLevel::Warning => Color32::from_rgb(80, 60, 20),
                ToastLevel::Error => Color32::from_rgb(80, 30, 30),
            }
        } else {
            match self {
                ToastLevel::Success => Color32::from_rgb(220, 255, 220),
                ToastLevel::Info => Color32::from_rgb(220, 235, 255),
                ToastLevel::Warning => Color32::from_rgb(255, 245, 200),
                ToastLevel::Error => Color32::from_rgb(255, 220, 220),
            }
        }
    }

    /// Get the text/icon color for this toast level
    pub fn text_color(&self, is_dark_theme: bool) -> Color32 {
        if is_dark_theme {
            match self {
                ToastLevel::Success => Color32::from_rgb(100, 220, 120),
                ToastLevel::Info => Color32::from_rgb(100, 180, 255),
                ToastLevel::Warning => Color32::from_rgb(255, 200, 80),
                ToastLevel::Error => Color32::from_rgb(255, 120, 120),
            }
        } else {
            match self {
                ToastLevel::Success => Color32::from_rgb(30, 120, 50),
                ToastLevel::Info => Color32::from_rgb(30, 80, 150),
                ToastLevel::Warning => Color32::from_rgb(150, 100, 0),
                ToastLevel::Error => Color32::from_rgb(180, 40, 40),
            }
        }
    }
}

/// A single toast notification
#[derive(Debug, Clone)]
pub struct Toast {
    /// The message to display
    pub message: String,
    /// The severity level
    pub level: ToastLevel,
    /// When this toast was created
    pub created_at: Instant,
    /// How long to show this toast
    pub duration: Duration,
}

impl Toast {
    /// Create a new toast
    pub fn new(message: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            message: message.into(),
            level,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    /// Create a success toast
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Success)
    }

    /// Create an info toast
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Info)
    }

    /// Create a warning toast
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Warning)
    }

    /// Create an error toast
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Error)
    }

    /// Set custom duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Check if this toast has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// Get the opacity based on remaining time (for fade out)
    pub fn opacity(&self) -> f32 {
        let elapsed = self.created_at.elapsed();
        let fade_start = self.duration.saturating_sub(Duration::from_millis(500));
        
        if elapsed >= self.duration {
            0.0
        } else if elapsed >= fade_start {
            let fade_progress = (self.duration - elapsed).as_secs_f32() / 0.5;
            fade_progress.clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
}

/// Manager for toast notifications
#[derive(Debug, Default)]
pub struct ToastManager {
    /// Active toasts
    toasts: Vec<Toast>,
}

impl ToastManager {
    /// Create a new toast manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a toast notification
    pub fn add(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    /// Add a success toast
    pub fn success(&mut self, message: impl Into<String>) {
        self.add(Toast::success(message));
    }

    /// Add an info toast
    pub fn info(&mut self, message: impl Into<String>) {
        self.add(Toast::info(message));
    }

    /// Add a warning toast
    pub fn warning(&mut self, message: impl Into<String>) {
        self.add(Toast::warning(message));
    }

    /// Add an error toast
    pub fn error(&mut self, message: impl Into<String>) {
        self.add(Toast::error(message));
    }

    /// Remove expired toasts
    pub fn cleanup(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Check if there are any active toasts
    pub fn has_toasts(&self) -> bool {
        !self.toasts.is_empty()
    }

    /// Render all active toasts
    pub fn render(&mut self, ctx: &Context, is_dark_theme: bool) {
        self.cleanup();

        if self.toasts.is_empty() {
            return;
        }

        // Request repaint for animation
        ctx.request_repaint();

        // Render toasts from bottom-right, stacking upward
        let screen_rect = ctx.screen_rect();
        let toast_width = 300.0;
        let toast_height = 40.0;
        let margin = 10.0;
        let spacing = 5.0;

        for (i, toast) in self.toasts.iter().enumerate() {
            let opacity = toast.opacity();
            if opacity <= 0.0 {
                continue;
            }

            let y_offset = (i as f32) * (toast_height + spacing);
            let pos = Pos2::new(
                screen_rect.right() - toast_width - margin,
                screen_rect.bottom() - toast_height - margin - y_offset - 30.0, // Above status bar
            );

            egui::Area::new(egui::Id::new(format!("toast_{}", i)))
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let bg_color = toast.level.background_color(is_dark_theme);
                    let text_color = toast.level.text_color(is_dark_theme);

                    // Apply opacity
                    let bg_color = Color32::from_rgba_unmultiplied(
                        bg_color.r(),
                        bg_color.g(),
                        bg_color.b(),
                        (230.0 * opacity) as u8,
                    );
                    let text_color = Color32::from_rgba_unmultiplied(
                        text_color.r(),
                        text_color.g(),
                        text_color.b(),
                        (255.0 * opacity) as u8,
                    );

                    egui::Frame::none()
                        .fill(bg_color)
                        .rounding(6.0)
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .stroke(egui::Stroke::new(1.0, text_color.gamma_multiply(0.3)))
                        .show(ui, |ui| {
                            ui.set_min_width(toast_width - 24.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(toast.level.icon()).color(text_color).strong());
                                ui.label(RichText::new(&toast.message).color(text_color));
                            });
                        });
                });
        }
    }
}
