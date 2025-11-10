use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use egui::{Color32, RichText};

/// State for the event editing dialog
pub struct EventDialogState {
    // Event being edited (None for new event)
    pub event_id: Option<i64>,
    
    // Basic fields
    pub title: String,
    pub description: String,
    pub location: String,
    
    // Date/time
    pub date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub all_day: bool,
    
    // Visual
    pub color: String,
    pub category: String,
    
    // Recurrence
    pub is_recurring: bool,
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until_date: Option<NaiveDate>,
    
    // BYDAY for weekly/monthly recurrence
    pub byday_enabled: bool,
    pub byday_monday: bool,
    pub byday_tuesday: bool,
    pub byday_wednesday: bool,
    pub byday_thursday: bool,
    pub byday_friday: bool,
    pub byday_saturday: bool,
    pub byday_sunday: bool,
    
    // UI state
    pub error_message: Option<String>,
    pub show_advanced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceFrequency {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
            Self::Yearly => "Yearly",
        }
    }
    
    fn to_rrule_freq(&self) -> &'static str {
        match self {
            Self::Daily => "DAILY",
            Self::Weekly => "WEEKLY",
            Self::Monthly => "MONTHLY",
            Self::Yearly => "YEARLY",
        }
    }
}

impl EventDialogState {
    /// Create a new event dialog state for creating a new event
    pub fn new_event(date: NaiveDate, settings: &Settings) -> Self {
        // Parse default start time from settings (format: "HH:MM")
        let (start_hour, start_minute) = settings.default_event_start_time
            .split_once(':')
            .and_then(|(h, m)| {
                let hour = h.parse::<u32>().ok()?;
                let minute = m.parse::<u32>().ok()?;
                Some((hour, minute))
            })
            .unwrap_or((9, 0));
        
        let start_time = NaiveTime::from_hms_opt(start_hour, start_minute, 0)
            .unwrap_or(NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        
        // Calculate end time based on default event duration
        let duration_minutes = settings.default_event_duration as i64;
        let end_datetime = NaiveDateTime::new(date, start_time) + chrono::Duration::minutes(duration_minutes);
        let end_time = end_datetime.time();
        
        Self {
            event_id: None,
            title: String::new(),
            description: String::new(),
            location: String::new(),
            date,
            start_time,
            end_time,
            all_day: false,
            color: "#3B82F6".to_string(), // Default blue
            category: String::new(),
            is_recurring: false,
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            count: None,
            until_date: None,
            byday_enabled: false,
            byday_monday: false,
            byday_tuesday: false,
            byday_wednesday: false,
            byday_thursday: false,
            byday_friday: false,
            byday_saturday: false,
            byday_sunday: false,
            error_message: None,
            show_advanced: false,
        }
    }
    
    /// Create a new event dialog state for editing an existing event
    pub fn from_event(event: &Event, _settings: &Settings) -> Self {
        let date = event.start.date_naive();
        let start_time = event.start.time();
        let end_time = event.end.time();
        
        // Parse recurrence rule if present
        let (is_recurring, frequency, interval, count, until_date, byday_flags) = 
            if let Some(ref rrule) = event.recurrence_rule {
                Self::parse_rrule(rrule)
            } else {
                (false, RecurrenceFrequency::Daily, 1, None, None, [false; 7])
            };
        
        Self {
            event_id: event.id,
            title: event.title.clone(),
            description: event.description.clone().unwrap_or_default(),
            location: event.location.clone().unwrap_or_default(),
            date,
            start_time,
            end_time,
            all_day: event.all_day,
            color: event.color.clone().unwrap_or_else(|| "#3B82F6".to_string()),
            category: event.category.clone().unwrap_or_default(),
            is_recurring,
            frequency,
            interval,
            count,
            until_date,
            byday_enabled: byday_flags.iter().any(|&b| b),
            byday_sunday: byday_flags[0],
            byday_monday: byday_flags[1],
            byday_tuesday: byday_flags[2],
            byday_wednesday: byday_flags[3],
            byday_thursday: byday_flags[4],
            byday_friday: byday_flags[5],
            byday_saturday: byday_flags[6],
            error_message: None,
            show_advanced: false,
        }
    }
    
    /// Parse RRULE string to extract recurrence information
    fn parse_rrule(rrule: &str) -> (bool, RecurrenceFrequency, u32, Option<u32>, Option<NaiveDate>, [bool; 7]) {
        let mut frequency = RecurrenceFrequency::Daily;
        let mut interval = 1u32;
        let mut count = None;
        let mut until_date = None;
        let mut byday_flags = [false; 7]; // SU, MO, TU, WE, TH, FR, SA
        
        for part in rrule.split(';') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "FREQ" => {
                        frequency = match value {
                            "DAILY" => RecurrenceFrequency::Daily,
                            "WEEKLY" => RecurrenceFrequency::Weekly,
                            "MONTHLY" => RecurrenceFrequency::Monthly,
                            "YEARLY" => RecurrenceFrequency::Yearly,
                            _ => RecurrenceFrequency::Daily,
                        };
                    }
                    "INTERVAL" => {
                        if let Ok(val) = value.parse::<u32>() {
                            interval = val;
                        }
                    }
                    "COUNT" => {
                        if let Ok(val) = value.parse::<u32>() {
                            count = Some(val);
                        }
                    }
                    "UNTIL" => {
                        // Parse UNTIL date (format: YYYYMMDD)
                        if value.len() >= 8 {
                            if let (Ok(year), Ok(month), Ok(day)) = (
                                value[0..4].parse::<i32>(),
                                value[4..6].parse::<u32>(),
                                value[6..8].parse::<u32>(),
                            ) {
                                until_date = NaiveDate::from_ymd_opt(year, month, day);
                            }
                        }
                    }
                    "BYDAY" => {
                        for day in value.split(',') {
                            match day {
                                "SU" => byday_flags[0] = true,
                                "MO" => byday_flags[1] = true,
                                "TU" => byday_flags[2] = true,
                                "WE" => byday_flags[3] = true,
                                "TH" => byday_flags[4] = true,
                                "FR" => byday_flags[5] = true,
                                "SA" => byday_flags[6] = true,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        (true, frequency, interval, count, until_date, byday_flags)
    }
    
    /// Build RRULE string from current state
    fn build_rrule(&self) -> Option<String> {
        if !self.is_recurring {
            return None;
        }
        
        let mut parts = vec![format!("FREQ={}", self.frequency.to_rrule_freq())];
        
        if self.interval > 1 {
            parts.push(format!("INTERVAL={}", self.interval));
        }
        
        // Add BYDAY if enabled and applicable
        if self.byday_enabled && (self.frequency == RecurrenceFrequency::Weekly || 
                                  self.frequency == RecurrenceFrequency::Monthly) {
            let mut days = Vec::new();
            if self.byday_sunday { days.push("SU"); }
            if self.byday_monday { days.push("MO"); }
            if self.byday_tuesday { days.push("TU"); }
            if self.byday_wednesday { days.push("WE"); }
            if self.byday_thursday { days.push("TH"); }
            if self.byday_friday { days.push("FR"); }
            if self.byday_saturday { days.push("SA"); }
            
            if !days.is_empty() {
                parts.push(format!("BYDAY={}", days.join(",")));
            }
        }
        
        // Add COUNT or UNTIL (mutually exclusive)
        if let Some(count) = self.count {
            parts.push(format!("COUNT={}", count));
        } else if let Some(until) = self.until_date {
            parts.push(format!("UNTIL={}", until.format("%Y%m%d")));
        }
        
        Some(parts.join(";"))
    }
    
    /// Validate the event data
    fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Event title is required".to_string());
        }
        
        if self.end_time <= self.start_time {
            return Err("End time must be after start time".to_string());
        }
        
        if self.is_recurring {
            if self.interval < 1 {
                return Err("Interval must be at least 1".to_string());
            }
            
            // If BYDAY is enabled for weekly/monthly, at least one day must be selected
            if self.byday_enabled && (self.frequency == RecurrenceFrequency::Weekly || 
                                     self.frequency == RecurrenceFrequency::Monthly) {
                let any_day_selected = self.byday_monday || self.byday_tuesday || 
                                      self.byday_wednesday || self.byday_thursday ||
                                      self.byday_friday || self.byday_saturday || 
                                      self.byday_sunday;
                
                if !any_day_selected {
                    return Err("Select at least one day for weekly/monthly recurrence".to_string());
                }
            }
        }
        
        Ok(())
    }
    
    /// Convert dialog state to Event
    fn to_event(&self) -> Result<Event, String> {
        self.validate()?;
        
        let start_datetime = self.date.and_time(self.start_time).and_local_timezone(Local).unwrap();
        let end_datetime = self.date.and_time(self.end_time).and_local_timezone(Local).unwrap();
        
        let mut event = Event::builder()
            .title(&self.title)
            .start(start_datetime)
            .end(end_datetime)
            .all_day(self.all_day);
        
        if !self.description.is_empty() {
            event = event.description(&self.description);
        }
        
        if !self.location.is_empty() {
            event = event.location(&self.location);
        }
        
        if !self.color.is_empty() {
            event = event.color(&self.color);
        }
        
        if !self.category.is_empty() {
            event = event.category(&self.category);
        }
        
        if let Some(rrule) = self.build_rrule() {
            event = event.recurrence_rule(rrule);
        }
        
        event.build()
    }
    
    /// Save the event (create or update)
    pub fn save(&self, database: &Database) -> Result<Event, String> {
        let mut event = self.to_event()?;
        
        let service = EventService::new(database.connection());
        
        if let Some(id) = self.event_id {
            // Update existing event
            event.id = Some(id);
            service.update(&event)
                .map_err(|e| format!("Failed to update event: {}", e))?;
            Ok(event)
        } else {
            // Create new event
            service.create(event)
                .map_err(|e| format!("Failed to create event: {}", e))
        }
    }
}

/// Render the event dialog
pub fn render_event_dialog(
    ctx: &egui::Context,
    state: &mut EventDialogState,
    database: &Database,
    _settings: &Settings,
    show_dialog: &mut bool,
) -> bool {
    let mut saved = false;
    
    egui::Window::new(if state.event_id.is_some() { "Edit Event" } else { "New Event" })
        .collapsible(false)
        .resizable(true)
        .default_width(600.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Error message display
                if let Some(ref error) = state.error_message {
                    ui.colored_label(Color32::RED, RichText::new(error).strong());
                    ui.add_space(8.0);
                }
                
                // Basic Information Section
                ui.heading("Basic Information");
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(&mut state.title);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Location:");
                    ui.text_edit_singleline(&mut state.location);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Category:");
                    ui.text_edit_singleline(&mut state.category);
                });
                
                ui.label("Description:");
                ui.text_edit_multiline(&mut state.description);
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Date and Time Section
                ui.heading("Date and Time");
                ui.add_space(4.0);
                
                // Date picker
                ui.horizontal(|ui| {
                    ui.label("Date:");
                    let date_string = state.date.format("%B %d, %Y").to_string();
                    ui.label(RichText::new(date_string).strong());
                });
                
                // Quick date adjustment
                ui.horizontal(|ui| {
                    if ui.button("< Previous Day").clicked() {
                        state.date = state.date.pred_opt().unwrap_or(state.date);
                    }
                    if ui.button("Today").clicked() {
                        state.date = Local::now().date_naive();
                    }
                    if ui.button("Next Day >").clicked() {
                        state.date = state.date.succ_opt().unwrap_or(state.date);
                    }
                });
                
                ui.add_space(4.0);
                
                // All-day checkbox
                ui.checkbox(&mut state.all_day, "All-day event");
                
                ui.add_space(4.0);
                
                // Time pickers (only if not all-day)
                if !state.all_day {
                    ui.horizontal(|ui| {
                        ui.label("Start time:");
                        render_time_picker(ui, &mut state.start_time);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("End time:");
                        render_time_picker(ui, &mut state.end_time);
                    });
                }
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Appearance Section
                ui.heading("Appearance");
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.add(egui::TextEdit::singleline(&mut state.color).desired_width(80.0));
                    
                    // Color preview
                    if let Some(mut color) = parse_hex_color(&state.color) {
                        ui.color_edit_button_srgba(&mut color);
                        // Update the hex string if user changed the color
                        state.color = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
                    }
                });
                
                // Preset colors
                ui.horizontal(|ui| {
                    ui.label("Presets:");
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
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Recurrence Section
                ui.heading("Recurrence");
                ui.add_space(4.0);
                
                ui.checkbox(&mut state.is_recurring, "Repeat this event");
                
                if state.is_recurring {
                    ui.add_space(4.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("Frequency:");
                        egui::ComboBox::from_id_source("frequency_combo")
                            .selected_text(state.frequency.as_str())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut state.frequency, RecurrenceFrequency::Daily, "Daily");
                                ui.selectable_value(&mut state.frequency, RecurrenceFrequency::Weekly, "Weekly");
                                ui.selectable_value(&mut state.frequency, RecurrenceFrequency::Monthly, "Monthly");
                                ui.selectable_value(&mut state.frequency, RecurrenceFrequency::Yearly, "Yearly");
                            });
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Every");
                        ui.add(egui::DragValue::new(&mut state.interval).range(1..=999));
                        ui.label(match state.frequency {
                            RecurrenceFrequency::Daily => "day(s)",
                            RecurrenceFrequency::Weekly => "week(s)",
                            RecurrenceFrequency::Monthly => "month(s)",
                            RecurrenceFrequency::Yearly => "year(s)",
                        });
                    });
                    
                    // BYDAY options for Weekly and Monthly
                    if state.frequency == RecurrenceFrequency::Weekly || 
                       state.frequency == RecurrenceFrequency::Monthly {
                        ui.add_space(4.0);
                        ui.checkbox(&mut state.byday_enabled, "Repeat on specific days");
                        
                        if state.byday_enabled {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut state.byday_sunday, "Sun");
                                ui.checkbox(&mut state.byday_monday, "Mon");
                                ui.checkbox(&mut state.byday_tuesday, "Tue");
                                ui.checkbox(&mut state.byday_wednesday, "Wed");
                                ui.checkbox(&mut state.byday_thursday, "Thu");
                                ui.checkbox(&mut state.byday_friday, "Fri");
                                ui.checkbox(&mut state.byday_saturday, "Sat");
                            });
                        }
                    }
                    
                    ui.add_space(8.0);
                    ui.label("End condition:");
                    
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
                            state.until_date = Some(state.date + chrono::Duration::days(30));
                            state.count = None;
                        }
                        
                        if let Some(until) = state.until_date {
                            ui.label(until.format("%Y-%m-%d").to_string());
                        }
                    });
                }
                
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        match state.save(database) {
                            Ok(_) => {
                                *show_dialog = false;
                                saved = true;
                            }
                            Err(e) => {
                                state.error_message = Some(e);
                            }
                        }
                    }
                    
                    if ui.button("Cancel").clicked() {
                        *show_dialog = false;
                    }
                    
                    if state.event_id.is_some() {
                        ui.add_space(20.0);
                        if ui.button(RichText::new("Delete").color(Color32::RED)).clicked() {
                            if let Some(id) = state.event_id {
                                let service = EventService::new(database.connection());
                                if let Err(e) = service.delete(id) {
                                    state.error_message = Some(format!("Failed to delete: {}", e));
                                } else {
                                    *show_dialog = false;
                                    saved = true; // Trigger refresh
                                }
                            }
                        }
                    }
                });
            });
        });
    
    saved
}

/// Simple time picker using hour and minute dropdowns
fn render_time_picker(ui: &mut egui::Ui, time: &mut NaiveTime) {
    let mut hour = time.hour();
    let mut minute = time.minute();
    
    ui.horizontal(|ui| {
        // Hour picker (0-23)
        egui::ComboBox::from_id_source(format!("hour_{:p}", time))
            .width(60.0)
            .selected_text(format!("{:02}", hour))
            .show_ui(ui, |ui| {
                for h in 0..24 {
                    ui.selectable_value(&mut hour, h, format!("{:02}", h));
                }
            });
        
        ui.label(":");
        
        // Minute picker (0-59)
        egui::ComboBox::from_id_source(format!("minute_{:p}", time))
            .width(60.0)
            .selected_text(format!("{:02}", minute))
            .show_ui(ui, |ui| {
                for m in (0..60).step_by(15) {
                    ui.selectable_value(&mut minute, m, format!("{:02}", m));
                }
            });
    });
    
    // Update time if changed
    if let Some(new_time) = NaiveTime::from_hms_opt(hour, minute, 0) {
        *time = new_time;
    }
}

/// Parse hex color string to egui Color32
fn parse_hex_color(hex: &str) -> Option<Color32> {
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
