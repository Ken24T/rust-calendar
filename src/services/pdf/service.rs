//! PDF export service implementation

use anyhow::{Context, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use printpdf::{
    BuiltinFont, IndirectFontRef, Mm, PdfDocument,
    PdfLayerReference, Point, Rgb,
};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::models::event::Event;
use crate::services::event::EventService;

/// Service for exporting calendar data to PDF
pub struct PdfExportService;

/// Export options
pub struct PdfExportOptions {
    /// Title for the PDF
    pub title: String,
    /// Page size (width, height) in mm
    pub page_size: (f32, f32),
    /// Include event descriptions
    pub include_descriptions: bool,
    /// Include event locations
    pub include_locations: bool,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            title: "Calendar".to_string(),
            page_size: (210.0, 297.0), // A4 Portrait
            include_descriptions: true,
            include_locations: true,
        }
    }
}

impl PdfExportService {
    /// Export a month view to PDF
    pub fn export_month(
        event_service: &EventService,
        date: NaiveDate,
        path: &Path,
        options: &PdfExportOptions,
        first_day_of_week: u8,
    ) -> Result<()> {
        let (doc, page1, layer1) = PdfDocument::new(
            &options.title,
            Mm(options.page_size.0),
            Mm(options.page_size.1),
            "Layer 1",
        );

        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .context("Failed to add font")?;
        let font_bold = doc
            .add_builtin_font(BuiltinFont::HelveticaBold)
            .context("Failed to add bold font")?;

        let layer = doc.get_page(page1).get_layer(layer1);

        // Draw month header
        let month_name = date.format("%B %Y").to_string();
        Self::draw_text(&layer, &font_bold, 24.0, 105.0, 280.0, &month_name, true);

        // Draw day headers
        let day_names = Self::get_day_names(first_day_of_week);
        let col_width = 25.0;
        let start_x = 20.0;
        let header_y = 265.0;

        for (i, day_name) in day_names.iter().enumerate() {
            let x = start_x + (i as f32 * col_width) + col_width / 2.0;
            Self::draw_text(&layer, &font_bold, 10.0, x, header_y, day_name, true);
        }

        // Draw calendar grid
        Self::draw_month_grid(
            &layer,
            event_service,
            date,
            start_x,
            header_y - 10.0,
            col_width,
            35.0,
            first_day_of_week,
            &font,
            &font_bold,
        )?;

        // Save to file
        let file = File::create(path).context("Failed to create PDF file")?;
        let mut writer = BufWriter::new(file);
        doc.save(&mut writer).context("Failed to save PDF")?;

        Ok(())
    }

    /// Export a week view to PDF
    pub fn export_week(
        event_service: &EventService,
        date: NaiveDate,
        path: &Path,
        options: &PdfExportOptions,
        first_day_of_week: u8,
    ) -> Result<()> {
        let (doc, page1, layer1) = PdfDocument::new(
            &options.title,
            Mm(options.page_size.1), // Landscape for week view
            Mm(options.page_size.0),
            "Layer 1",
        );

        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .context("Failed to add font")?;
        let font_bold = doc
            .add_builtin_font(BuiltinFont::HelveticaBold)
            .context("Failed to add bold font")?;

        let layer = doc.get_page(page1).get_layer(layer1);

        // Calculate week start
        let week_start = Self::get_week_start(date, first_day_of_week);
        let week_end = week_start + Duration::days(6);

        // Draw week header
        let week_title = format!(
            "Week of {} - {}",
            week_start.format("%B %d"),
            week_end.format("%B %d, %Y")
        );
        Self::draw_text(&layer, &font_bold, 18.0, 148.5, 195.0, &week_title, true);

        // Draw day columns
        let col_width = 38.0;
        let start_x = 15.0;
        let header_y = 180.0;
        let day_names = Self::get_full_day_names(first_day_of_week);

        // Get events for the week
        let start = Local
            .from_local_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end = Local
            .from_local_datetime(&week_end.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();
        let events = event_service
            .expand_recurring_events(start, end)
            .unwrap_or_default();

        for i in 0..7 {
            let day_date = week_start + Duration::days(i);
            let x = start_x + (i as f32 * col_width);

            // Day header
            let header_text = format!("{}\n{}", day_names[i as usize], day_date.format("%d"));
            Self::draw_text(&layer, &font_bold, 10.0, x + col_width / 2.0, header_y, &day_names[i as usize], true);
            Self::draw_text(&layer, &font, 9.0, x + col_width / 2.0, header_y - 6.0, &day_date.format("%d").to_string(), true);

            // Draw border for day column
            Self::draw_rect(&layer, x, header_y - 165.0, col_width - 1.0, 155.0);

            // Events for this day
            let day_events: Vec<&Event> = events
                .iter()
                .filter(|e| e.start.date_naive() == day_date)
                .collect();

            let mut y_offset = header_y - 15.0;
            for event in day_events.iter().take(8) {
                let time_str = if event.all_day {
                    "All day".to_string()
                } else {
                    event.start.format("%H:%M").to_string()
                };

                // Truncate title to fit
                let title = if event.title.len() > 15 {
                    format!("{}...", &event.title[..12])
                } else {
                    event.title.clone()
                };

                Self::draw_text(&layer, &font, 7.0, x + 2.0, y_offset, &time_str, false);
                Self::draw_text(&layer, &font, 7.0, x + 2.0, y_offset - 4.0, &title, false);
                y_offset -= 12.0;
            }

            if day_events.len() > 8 {
                Self::draw_text(
                    &layer,
                    &font,
                    6.0,
                    x + 2.0,
                    y_offset,
                    &format!("+{} more", day_events.len() - 8),
                    false,
                );
            }
        }

        // Save to file
        let file = File::create(path).context("Failed to create PDF file")?;
        let mut writer = BufWriter::new(file);
        doc.save(&mut writer).context("Failed to save PDF")?;

        Ok(())
    }

    /// Export an event list to PDF
    pub fn export_event_list(
        events: &[Event],
        path: &Path,
        options: &PdfExportOptions,
    ) -> Result<()> {
        let (doc, page1, layer1) = PdfDocument::new(
            &options.title,
            Mm(options.page_size.0),
            Mm(options.page_size.1),
            "Layer 1",
        );

        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .context("Failed to add font")?;
        let font_bold = doc
            .add_builtin_font(BuiltinFont::HelveticaBold)
            .context("Failed to add bold font")?;

        let mut _current_page = page1;
        let mut current_layer = doc.get_page(page1).get_layer(layer1);

        // Title
        Self::draw_text(&current_layer, &font_bold, 18.0, 105.0, 280.0, &options.title, true);

        let mut y = 260.0;
        let margin_left = 20.0;
        let page_height = options.page_size.1;

        for event in events {
            // Check if we need a new page
            if y < 30.0 {
                let (new_page, new_layer) =
                    doc.add_page(Mm(options.page_size.0), Mm(options.page_size.1), "Layer 1");
                _current_page = new_page;
                current_layer = doc.get_page(new_page).get_layer(new_layer);
                y = page_height - 20.0;
            }

            // Event title
            Self::draw_text(&current_layer, &font_bold, 11.0, margin_left, y, &event.title, false);
            y -= 5.0;

            // Date/time
            let datetime_str = if event.all_day {
                event.start.format("%B %d, %Y (All day)").to_string()
            } else {
                event
                    .start
                    .format("%B %d, %Y at %I:%M %p")
                    .to_string()
            };
            Self::draw_text(&current_layer, &font, 9.0, margin_left, y, &datetime_str, false);
            y -= 4.0;

            // Location
            if options.include_locations {
                if let Some(ref loc) = event.location {
                    if !loc.is_empty() {
                        Self::draw_text(
                            &current_layer,
                            &font,
                            8.0,
                            margin_left,
                            y,
                            &format!("📍 {}", loc),
                            false,
                        );
                        y -= 4.0;
                    }
                }
            }

            // Description
            if options.include_descriptions {
                if let Some(ref desc) = event.description {
                    if !desc.is_empty() {
                        let truncated = if desc.len() > 100 {
                            format!("{}...", &desc[..97])
                        } else {
                            desc.clone()
                        };
                        Self::draw_text(&current_layer, &font, 8.0, margin_left, y, &truncated, false);
                        y -= 4.0;
                    }
                }
            }

            y -= 6.0; // Space between events
        }

        // Save to file
        let file = File::create(path).context("Failed to create PDF file")?;
        let mut writer = BufWriter::new(file);
        doc.save(&mut writer).context("Failed to save PDF")?;

        Ok(())
    }

    fn draw_text(
        layer: &PdfLayerReference,
        font: &IndirectFontRef,
        size: f32,
        x: f32,
        y: f32,
        text: &str,
        centered: bool,
    ) {
        layer.begin_text_section();
        layer.set_font(font, size);
        layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));

        let position = if centered {
            // Approximate centering based on character count
            let approx_width = text.len() as f32 * size * 0.4;
            (Mm(x - approx_width / 2.0), Mm(y))
        } else {
            (Mm(x), Mm(y))
        };

        layer.set_text_cursor(position.0, position.1);
        layer.write_text(text, font);
        layer.end_text_section();
    }

    fn draw_rect(layer: &PdfLayerReference, x: f32, y: f32, width: f32, height: f32) {
        let points = vec![
            (Point::new(Mm(x), Mm(y)), false),
            (Point::new(Mm(x + width), Mm(y)), false),
            (Point::new(Mm(x + width), Mm(y + height)), false),
            (Point::new(Mm(x), Mm(y + height)), false),
        ];
        layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.7, 0.7, 0.7, None)));
        layer.set_outline_thickness(0.5);
        layer.add_polygon(printpdf::Polygon {
            rings: vec![points],
            mode: printpdf::path::PaintMode::Stroke,
            winding_order: printpdf::path::WindingOrder::NonZero,
        });
    }

    fn draw_month_grid(
        layer: &PdfLayerReference,
        event_service: &EventService,
        date: NaiveDate,
        start_x: f32,
        start_y: f32,
        col_width: f32,
        row_height: f32,
        first_day_of_week: u8,
        font: &IndirectFontRef,
        font_bold: &IndirectFontRef,
    ) -> Result<()> {
        let first_of_month = date.with_day(1).unwrap();
        let first_weekday = (first_of_month.weekday().num_days_from_sunday() as i32
            - first_day_of_week as i32
            + 7)
            % 7;
        let days_in_month = Self::get_days_in_month(date.year(), date.month());

        // Get events for the month
        let start = Local
            .from_local_datetime(&first_of_month.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end_of_month = date.with_day(days_in_month as u32).unwrap();
        let end = Local
            .from_local_datetime(&end_of_month.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();
        let events = event_service
            .expand_recurring_events(start, end)
            .unwrap_or_default();

        let mut day_counter = 1 - first_weekday;
        let today = Local::now().date_naive();

        for week in 0..6 {
            let y = start_y - (week as f32 * row_height);

            for day_of_week in 0..7 {
                let x = start_x + (day_of_week as f32 * col_width);

                // Draw cell border
                Self::draw_rect(layer, x, y - row_height, col_width - 1.0, row_height - 1.0);

                if day_counter >= 1 && day_counter <= days_in_month {
                    let cell_date = NaiveDate::from_ymd_opt(
                        date.year(),
                        date.month(),
                        day_counter as u32,
                    )
                    .unwrap();

                    // Draw day number
                    let day_str = format!("{}", day_counter);
                    let is_today = cell_date == today;
                    let day_font = if is_today { font_bold } else { font };
                    Self::draw_text(layer, day_font, 9.0, x + 2.0, y - 5.0, &day_str, false);

                    // Draw events (max 2)
                    let day_events: Vec<&Event> = events
                        .iter()
                        .filter(|e| {
                            if e.all_day {
                                let start_date = e.start.date_naive();
                                let end_date = e.end.date_naive();
                                cell_date >= start_date && cell_date <= end_date
                            } else {
                                e.start.date_naive() == cell_date
                            }
                        })
                        .collect();

                    let mut event_y = y - 12.0;
                    for event in day_events.iter().take(2) {
                        let title = if event.title.len() > 10 {
                            format!("{}...", &event.title[..7])
                        } else {
                            event.title.clone()
                        };
                        Self::draw_text(layer, font, 6.0, x + 2.0, event_y, &title, false);
                        event_y -= 5.0;
                    }

                    if day_events.len() > 2 {
                        Self::draw_text(
                            layer,
                            font,
                            5.0,
                            x + 2.0,
                            event_y,
                            &format!("+{}", day_events.len() - 2),
                            false,
                        );
                    }
                }

                day_counter += 1;
            }
        }

        Ok(())
    }

    fn get_week_start(date: NaiveDate, first_day_of_week: u8) -> NaiveDate {
        let weekday = date.weekday().num_days_from_sunday();
        let days_to_subtract = (weekday as i64 - first_day_of_week as i64 + 7) % 7;
        date - Duration::days(days_to_subtract)
    }

    fn get_days_in_month(year: i32, month: u32) -> i32 {
        NaiveDate::from_ymd_opt(
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 },
            1,
        )
        .unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as i32
    }

    fn get_day_names(first_day_of_week: u8) -> Vec<&'static str> {
        let all_days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let start = first_day_of_week as usize;
        (0..7).map(|i| all_days[(start + i) % 7]).collect()
    }

    fn get_full_day_names(first_day_of_week: u8) -> Vec<&'static str> {
        let all_days = [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ];
        let start = first_day_of_week as usize;
        (0..7).map(|i| all_days[(start + i) % 7]).collect()
    }
}
