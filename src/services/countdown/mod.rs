mod models;
mod persistence;
mod service;

pub use models::{
    CountdownAutoDismissConfig, CountdownCardGeometry, CountdownCardId, CountdownCardState,
    CountdownCardVisuals, CountdownNotificationConfig, CountdownWarningState, WarningThresholds,
    MAX_DAYS_FONT_SIZE, RgbaColor,
};
pub use service::CountdownService;
