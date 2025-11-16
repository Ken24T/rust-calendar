use chrono::{NaiveTime, Timelike};
use egui::Color32;

pub fn render_time_picker(ui: &mut egui::Ui, time: &mut NaiveTime) {
    let mut hour = time.hour();
    let mut minute = time.minute();

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_source(format!("hour_{:p}", time))
            .width(60.0)
            .selected_text(format!("{:02}", hour))
            .show_ui(ui, |ui| {
                for h in 0..24 {
                    ui.selectable_value(&mut hour, h, format!("{:02}", h));
                }
            });

        ui.label(":");

        egui::ComboBox::from_id_source(format!("minute_{:p}", time))
            .width(60.0)
            .selected_text(format!("{:02}", minute))
            .show_ui(ui, |ui| {
                for m in (0..60).step_by(15) {
                    ui.selectable_value(&mut minute, m, format!("{:02}", m));
                }
            });
    });

    if let Some(new_time) = NaiveTime::from_hms_opt(hour, minute, 0) {
        *time = new_time;
    }
}

pub fn parse_hex_color(hex: &str) -> Option<Color32> {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}
