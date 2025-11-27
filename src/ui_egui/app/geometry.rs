use super::CalendarApp;
use crate::services::countdown::CountdownCardGeometry;

const MIN_ROOT_WIDTH: f32 = 320.0;
const MIN_ROOT_HEIGHT: f32 = 220.0;

/// Default fallback screen dimensions when no monitor info is available
const DEFAULT_SCREEN_WIDTH: f32 = 1920.0;
const DEFAULT_SCREEN_HEIGHT: f32 = 1080.0;

impl CalendarApp {
    pub(super) fn apply_pending_root_geometry(&mut self, ctx: &egui::Context) {
        if let Some(geometry) = self.state.pending_root_geometry.take() {
            if !Self::is_plausible_root_geometry(&geometry) {
                log::warn!(
                    "Ignoring persisted root geometry due to implausible size: {:?}",
                    geometry
                );
                return;
            }
            
            // Sanitize geometry to ensure it's on a visible monitor
            let monitors = get_monitor_bounds(ctx);
            let sanitized = geometry.sanitize_for_monitors(&monitors, (100.0, 100.0));
            
            if sanitized != geometry {
                log::info!(
                    "Sanitized root geometry from {:?} to {:?} (monitor configuration may have changed)",
                    geometry, sanitized
                );
            }
            
            log::debug!("Applying persisted root geometry: {:?}", sanitized);
            if sanitized.width > 40.0 && sanitized.height > 40.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    sanitized.x, sanitized.y,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    sanitized.width,
                    sanitized.height,
                )));
            }
        }
    }

    pub(super) fn capture_root_geometry(&mut self, ctx: &egui::Context) {
        if let Some(info) = viewport_info(ctx, egui::ViewportId::ROOT) {
            if let Some(geometry) = geometry_from_viewport_info(&info) {
                if !Self::is_plausible_root_geometry(&geometry) {
                    log::debug!(
                        "Skipping root geometry capture due to implausible size: {:?}",
                        geometry
                    );
                    return;
                }
                let needs_update = match self.context.countdown_service().app_window_geometry() {
                    Some(current) => geometry_changed(current, geometry),
                    None => true,
                };
                if needs_update {
                    log::debug!("Captured new root geometry: {:?}", geometry);
                    self.context
                        .countdown_service_mut()
                        .update_app_window_geometry(geometry);
                }
            }
        }
    }

    pub(super) fn is_plausible_root_geometry(geometry: &CountdownCardGeometry) -> bool {
        geometry.width >= MIN_ROOT_WIDTH && geometry.height >= MIN_ROOT_HEIGHT
    }
    
    /// Sanitize all countdown card and container geometries to ensure they're visible
    /// on available monitors. Called on first frame when monitor info is available.
    pub(super) fn sanitize_countdown_geometries(&mut self, ctx: &egui::Context) {
        let monitors = get_monitor_bounds(ctx);
        log::info!(
            "Sanitizing countdown geometries for {} monitor(s): {:?}",
            monitors.len(),
            monitors
        );
        self.context
            .countdown_service_mut()
            .sanitize_all_geometries(&monitors);
    }
}

pub(super) fn viewport_info(
    ctx: &egui::Context,
    viewport_id: egui::ViewportId,
) -> Option<egui::ViewportInfo> {
    ctx.input(|input| input.raw.viewports.get(&viewport_id).cloned())
}

pub(super) fn geometry_from_viewport_info(
    info: &egui::ViewportInfo,
) -> Option<CountdownCardGeometry> {
    let inner = match info.inner_rect {
        Some(rect) => rect,
        None => return None,
    };
    let (outer_left, outer_top) = info
        .outer_rect
        .map(|outer| (outer.left(), outer.top()))
        .unwrap_or((inner.left(), inner.top()));

    Some(CountdownCardGeometry {
        x: outer_left,
        y: outer_top,
        width: inner.width(),
        height: inner.height(),
    })
}

pub(super) fn geometry_changed(a: CountdownCardGeometry, b: CountdownCardGeometry) -> bool {
    (a.x - b.x).abs() > 2.0
        || (a.y - b.y).abs() > 2.0
        || (a.width - b.width).abs() > 1.0
        || (a.height - b.height).abs() > 1.0
}

/// Get the bounds of the virtual desktop (all monitors combined).
/// Since egui doesn't provide full multi-monitor info, we estimate based on
/// the primary monitor size and allow for reasonable multi-monitor setups.
/// Returns a list of (x, y, width, height) tuples representing valid screen regions.
pub(super) fn get_monitor_bounds(ctx: &egui::Context) -> Vec<(f32, f32, f32, f32)> {
    ctx.input(|input| {
        // Try to get the primary monitor size from viewport info
        let primary_size = input.raw.viewports
            .values()
            .filter_map(|info| info.monitor_size)
            .next();
        
        let (base_width, base_height) = primary_size
            .map(|s| (s.x, s.y))
            .unwrap_or((DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT));
        
        // Create a virtual desktop that spans a reasonable multi-monitor setup
        // This allows for monitors to the left (negative X), right, above (negative Y), or below
        // We assume up to 3 monitors in any direction
        let virtual_left = -base_width * 2.0;
        let virtual_top = -base_height * 2.0;
        let virtual_width = base_width * 5.0;  // 2 left + primary + 2 right
        let virtual_height = base_height * 5.0; // 2 above + primary + 2 below
        
        vec![(virtual_left, virtual_top, virtual_width, virtual_height)]
    })
}
