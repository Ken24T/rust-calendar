/// Event-derived colour palette for countdown cards.
///
/// Generates a coordinated set of title/body/text colours from
/// a single base event colour, ensuring readable contrast.

use super::models::{CountdownCardState, RgbaColor};

#[derive(Clone, Copy)]
pub(super) struct EventPalette {
    pub title_bg: RgbaColor,
    pub title_fg: RgbaColor,
    pub body_bg: RgbaColor,
    pub days_fg: RgbaColor,
}

pub(super) fn event_palette_for(card: &CountdownCardState) -> Option<EventPalette> {
    card.event_color.map(EventPalette::from_base)
}

/// Apply event-derived palette colours to any card visual fields that
/// are NOT using the global default (i.e. `use_default_*` is false).
pub(super) fn apply_event_palette_if_needed(card: &mut CountdownCardState) {
    let Some(palette) = event_palette_for(card) else {
        return;
    };

    if !card.visuals.use_default_title_bg {
        card.visuals.title_bg_color = palette.title_bg;
    }
    if !card.visuals.use_default_title_fg {
        card.visuals.title_fg_color = palette.title_fg;
    }
    if !card.visuals.use_default_body_bg {
        card.visuals.body_bg_color = palette.body_bg;
    }
    if !card.visuals.use_default_days_fg {
        card.visuals.days_fg_color = palette.days_fg;
    }
}

impl EventPalette {
    fn from_base(base: RgbaColor) -> Self {
        let title_bg = darken_color(base, 0.18);
        let body_bg = lighten_color(base, 0.12);
        let title_fg = readable_text_color(title_bg);
        let days_fg = readable_text_color(body_bg);
        Self {
            title_bg,
            title_fg,
            body_bg,
            days_fg,
        }
    }
}

// ── Colour arithmetic ──────────────────────────────────────────────

fn readable_text_color(bg: RgbaColor) -> RgbaColor {
    const LIGHT: RgbaColor = RgbaColor::new(255, 255, 255, 255);
    const DARK: RgbaColor = RgbaColor::new(20, 28, 45, 255);
    if relative_luminance(bg) > 0.5 {
        DARK
    } else {
        LIGHT
    }
}

fn lighten_color(color: RgbaColor, factor: f32) -> RgbaColor {
    mix_colors(color, RgbaColor::new(255, 255, 255, color.a), factor)
}

fn darken_color(color: RgbaColor, factor: f32) -> RgbaColor {
    mix_colors(color, RgbaColor::new(0, 0, 0, color.a), factor)
}

fn mix_colors(base: RgbaColor, target: RgbaColor, factor: f32) -> RgbaColor {
    let weight = factor.clamp(0.0, 1.0);
    let mix = |start: u8, end: u8| -> u8 {
        let start_f = start as f32;
        let end_f = end as f32;
        ((start_f + (end_f - start_f) * weight).round()).clamp(0.0, 255.0) as u8
    };
    RgbaColor::new(
        mix(base.r, target.r),
        mix(base.g, target.g),
        mix(base.b, target.b),
        base.a,
    )
}

fn relative_luminance(color: RgbaColor) -> f32 {
    fn srgb_component(value: u8) -> f32 {
        let channel = value as f32 / 255.0;
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    let r = srgb_component(color.r);
    let g = srgb_component(color.g);
    let b = srgb_component(color.b);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_has_high_luminance() {
        let white = RgbaColor::new(255, 255, 255, 255);
        assert!(relative_luminance(white) > 0.9);
    }

    #[test]
    fn black_has_low_luminance() {
        let black = RgbaColor::new(0, 0, 0, 255);
        assert!(relative_luminance(black) < 0.01);
    }

    #[test]
    fn readable_text_on_dark_bg_is_light() {
        let dark = RgbaColor::new(30, 30, 30, 255);
        let text = readable_text_color(dark);
        // Expect light text on dark background
        assert!(text.r > 200);
    }

    #[test]
    fn readable_text_on_light_bg_is_dark() {
        let light = RgbaColor::new(230, 230, 230, 255);
        let text = readable_text_color(light);
        // Expect dark text on light background
        assert!(text.r < 50);
    }

    #[test]
    fn palette_from_base_produces_coordinated_colours() {
        let base = RgbaColor::new(100, 150, 200, 255);
        let palette = EventPalette::from_base(base);
        // Title bg should be darker than base
        assert!(palette.title_bg.r <= base.r);
        // Body bg should be lighter than base
        assert!(palette.body_bg.r >= base.r);
    }

    #[test]
    fn mix_at_zero_returns_base() {
        let base = RgbaColor::new(100, 150, 200, 255);
        let target = RgbaColor::new(0, 0, 0, 255);
        let result = mix_colors(base, target, 0.0);
        assert_eq!(result.r, base.r);
        assert_eq!(result.g, base.g);
        assert_eq!(result.b, base.b);
    }
}
