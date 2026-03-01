use super::CalendarApp;
use crate::services::countdown::CountdownCardGeometry;
use crate::utils::monitors::{self, MonitorRect};

const MIN_ROOT_WIDTH: f32 = 320.0;
const MIN_ROOT_HEIGHT: f32 = 220.0;

/// Minimum number of pixels the window must overlap a monitor to count as "visible"
const MIN_VISIBLE_X: f32 = 200.0;
const MIN_VISIBLE_Y: f32 = 100.0;

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
            
            // Get real monitor layout and validate the saved position
            let monitors = get_real_monitors(ctx);
            let visible = monitors::is_visible_on_any_monitor(
                &monitors,
                geometry.x,
                geometry.y,
                geometry.width,
                geometry.height,
                (MIN_VISIBLE_X, MIN_VISIBLE_Y),
            );
            
            let final_geom = if visible {
                log::debug!("Persisted root geometry is visible on a monitor: {:?}", geometry);
                geometry
            } else {
                // Window would be off-screen — centre it on the nearest monitor
                let nearest = monitors::nearest_monitor(&monitors, geometry.x, geometry.y);
                let centred = centre_on_monitor(nearest, geometry.width, geometry.height);
                log::info!(
                    "Persisted root geometry {:?} is not visible on any monitor; \
                     centering on nearest monitor {:?} → {:?}",
                    geometry, nearest, centred
                );
                centred
            };
            
            if final_geom.width > 40.0 && final_geom.height > 40.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    final_geom.x, final_geom.y,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    final_geom.width,
                    final_geom.height,
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
        let monitors = get_real_monitors(ctx);
        let monitor_tuples: Vec<(f32, f32, f32, f32)> = monitors
            .iter()
            .map(|m| (m.x, m.y, m.width, m.height))
            .collect();
        log::info!(
            "Sanitizing countdown geometries for {} monitor(s): {:?}",
            monitor_tuples.len(),
            monitor_tuples
        );
        self.context
            .countdown_service_mut()
            .sanitize_all_geometries(&monitor_tuples);
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
    let inner = info.inner_rect?;
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

/// Get real monitor rectangles, using the Win32 API on Windows and falling
/// back to egui's `monitor_size` on other platforms.
pub(super) fn get_real_monitors(ctx: &egui::Context) -> Vec<MonitorRect> {
    let egui_size = ctx.input(|input| {
        input
            .raw
            .viewports
            .values()
            .filter_map(|info| info.monitor_size)
            .next()
            .map(|s| (s.x, s.y))
    });

    let monitors = monitors::get_available_monitors(egui_size);
    log::debug!("Detected {} monitor(s): {:?}", monitors.len(), monitors);
    monitors
}

/// Centre a window of the given size on the supplied monitor.
fn centre_on_monitor(monitor: &MonitorRect, width: f32, height: f32) -> CountdownCardGeometry {
    // Clamp dimensions so the window fits inside the monitor
    let w = width.min(monitor.width);
    let h = height.min(monitor.height);
    CountdownCardGeometry {
        x: monitor.x + (monitor.width - w) / 2.0,
        y: monitor.y + (monitor.height - h) / 2.0,
        width: w,
        height: h,
    }
}
