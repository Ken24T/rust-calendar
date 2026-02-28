use super::CalendarApp;
use chrono::Datelike;
use crate::services::event::EventService;
use crate::services::pdf::{PdfExportService, service::PdfExportOptions};

/// Export-related menu functions (PDF and ICS export).
impl CalendarApp {
    pub(super) fn export_month_to_pdf(&self) {
        let date = self.current_date;
        let month_name = date.format("%B_%Y").to_string();
        
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export Month View to PDF")
            .set_file_name(format!("calendar_{}.pdf", month_name))
            .add_filter("PDF files", &["pdf"])
            .save_file()
        {
            let event_service = EventService::new(self.context.database().connection());
            let options = PdfExportOptions {
                title: format!("Calendar - {}", date.format("%B %Y")),
                ..Default::default()
            };
            
            if let Err(e) = PdfExportService::export_month(
                &event_service,
                date,
                &path,
                &options,
                self.settings.first_day_of_week,
            ) {
                log::error!("Failed to export PDF: {}", e);
            } else {
                log::info!("Successfully exported month view to {:?}", path);
            }
        }
    }

    pub(super) fn export_week_to_pdf(&self) {
        let date = self.current_date;
        let week_num = date.iso_week().week();
        
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export Week View to PDF")
            .set_file_name(format!("calendar_week_{}.pdf", week_num))
            .add_filter("PDF files", &["pdf"])
            .save_file()
        {
            let event_service = EventService::new(self.context.database().connection());
            let options = PdfExportOptions {
                title: format!("Calendar - Week {}", week_num),
                ..Default::default()
            };
            
            if let Err(e) = PdfExportService::export_week(
                &event_service,
                date,
                &path,
                &options,
                self.settings.first_day_of_week,
            ) {
                log::error!("Failed to export PDF: {}", e);
            } else {
                log::info!("Successfully exported week view to {:?}", path);
            }
        }
    }

    pub(super) fn export_events_to_pdf(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export All Events to PDF")
            .set_file_name("calendar_events.pdf")
            .add_filter("PDF files", &["pdf"])
            .save_file()
        {
            let event_service = EventService::new(self.context.database().connection());
            let events = event_service.list_all().unwrap_or_default();
            let options = PdfExportOptions {
                title: "Calendar Events".to_string(),
                ..Default::default()
            };
            
            if let Err(e) = PdfExportService::export_event_list(&events, &path, &options) {
                log::error!("Failed to export PDF: {}", e);
            } else {
                log::info!("Successfully exported {} events to {:?}", events.len(), path);
            }
        }
    }

    /// Export all events to an .ics file
    pub(super) fn export_all_events_ics(&mut self) {
        let event_service = EventService::new(self.context.database().connection());
        let events = match event_service.list_all() {
            Ok(events) => events,
            Err(e) => {
                log::error!("Failed to load events for export: {}", e);
                self.toast_manager.error("Failed to load events");
                return;
            }
        };

        if events.is_empty() {
            self.toast_manager.warning("No events to export");
            return;
        }

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export All Events")
            .set_file_name("calendar_events.ics")
            .add_filter("iCalendar", &["ics"])
            .save_file()
        {
            use crate::services::icalendar::ICalendarService;
            let ics_service = ICalendarService::new();
            
            match ics_service.export_events_to_file(&events, &path) {
                Ok(()) => {
                    log::info!("Exported {} events to {:?}", events.len(), path);
                    self.toast_manager.success(format!("Exported {} events", events.len()));
                }
                Err(e) => {
                    log::error!("Failed to export events: {}", e);
                    self.toast_manager.error("Failed to export events");
                }
            }
        }
    }

    /// Export filtered events (by category) to an .ics file
    pub(super) fn export_filtered_events_ics(&mut self) {
        let category = match &self.active_category_filter {
            Some(cat) => cat.clone(),
            None => {
                // No filter active, fall back to export all
                self.export_all_events_ics();
                return;
            }
        };

        let event_service = EventService::new(self.context.database().connection());
        let all_events = match event_service.list_all() {
            Ok(events) => events,
            Err(e) => {
                log::error!("Failed to load events for export: {}", e);
                self.toast_manager.error("Failed to load events");
                return;
            }
        };

        // Filter events by category
        let events: Vec<_> = all_events
            .into_iter()
            .filter(|e| e.category.as_deref() == Some(&category))
            .collect();

        if events.is_empty() {
            self.toast_manager.warning(format!("No '{}' events to export", category));
            return;
        }

        let safe_category = category.replace(' ', "_").replace('/', "-");
        if let Some(path) = rfd::FileDialog::new()
            .set_title(format!("Export '{}' Events", category))
            .set_file_name(format!("{}_events.ics", safe_category))
            .add_filter("iCalendar", &["ics"])
            .save_file()
        {
            use crate::services::icalendar::ICalendarService;
            let ics_service = ICalendarService::new();
            
            match ics_service.export_events_to_file(&events, &path) {
                Ok(()) => {
                    log::info!("Exported {} '{}' events to {:?}", events.len(), category, path);
                    self.toast_manager.success(format!("Exported {} '{}' events", events.len(), category));
                }
                Err(e) => {
                    log::error!("Failed to export events: {}", e);
                    self.toast_manager.error("Failed to export events");
                }
            }
        }
    }

    /// Export events in a date range to an .ics file
    pub(super) fn export_events_in_range(&mut self, start: chrono::NaiveDate, end: chrono::NaiveDate) {
        use chrono::{Local, NaiveTime, TimeZone};
        
        // Convert NaiveDate to DateTime for the query
        let start_dt = Local.from_local_datetime(
            &start.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        ).unwrap();
        let end_dt = Local.from_local_datetime(
            &end.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap())
        ).unwrap();
        
        let event_service = EventService::new(self.context.database().connection());
        let events = match event_service.find_by_date_range(start_dt, end_dt) {
            Ok(events) => events,
            Err(e) => {
                log::error!("Failed to load events for export: {}", e);
                self.toast_manager.error("Failed to load events");
                return;
            }
        };

        if events.is_empty() {
            self.toast_manager.warning("No events in selected range");
            return;
        }

        let filename = format!("calendar_{}_{}.ics", 
            start.format("%Y%m%d"), 
            end.format("%Y%m%d")
        );

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export Events")
            .set_file_name(&filename)
            .add_filter("iCalendar", &["ics"])
            .save_file()
        {
            use crate::services::icalendar::ICalendarService;
            let ics_service = ICalendarService::new();
            
            match ics_service.export_events_to_file(&events, &path) {
                Ok(()) => {
                    log::info!("Exported {} events to {:?}", events.len(), path);
                    self.toast_manager.success(format!("Exported {} events", events.len()));
                }
                Err(e) => {
                    log::error!("Failed to export events: {}", e);
                    self.toast_manager.error("Failed to export events");
                }
            }
        }
    }
}
