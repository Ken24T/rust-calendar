#![allow(dead_code)]

//! RFC 5545 (.ics) import/export service.

mod export;
mod import;
mod service;
mod utils;

#[allow(unused_imports)]
pub use service::ICalendarService;
