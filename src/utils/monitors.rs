//! Cross-platform monitor enumeration for window position validation.
//!
//! On Windows, uses the Win32 `EnumDisplayMonitors` / `GetMonitorInfoW` APIs
//! to return the actual working-area rectangles of every connected monitor.
//!
//! On other platforms, falls back to a single-monitor estimate derived from
//! the egui viewport's `monitor_size` field (or a 1920×1080 default).
/// A rectangle representing a monitor's working area (excludes taskbar, etc.).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl MonitorRect {
    /// Returns true if `other` overlaps this monitor by at least
    /// `min_overlap_x` pixels horizontally **and** `min_overlap_y` pixels
    /// vertically.
    pub fn overlaps(&self, other: &MonitorRect, min_overlap_x: f32, min_overlap_y: f32) -> bool {
        let overlap_left = self.x.max(other.x);
        let overlap_right = (self.x + self.width).min(other.x + other.width);
        let overlap_top = self.y.max(other.y);
        let overlap_bottom = (self.y + self.height).min(other.y + other.height);

        let horizontal = overlap_right - overlap_left;
        let vertical = overlap_bottom - overlap_top;

        horizontal >= min_overlap_x && vertical >= min_overlap_y
    }

    /// Centre point of this rectangle.
    pub fn centre(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

// ── Windows implementation ──────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use super::MonitorRect;

    use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };

    /// Enumerate all connected monitors via the Win32 API.
    /// Returns one `MonitorRect` per monitor using the **working area**
    /// (i.e. desktop area excluding the taskbar).
    pub fn enumerate_monitors() -> Vec<MonitorRect> {
        let mut monitors: Vec<MonitorRect> = Vec::new();

        unsafe {
            let monitors_ptr = &mut monitors as *mut Vec<MonitorRect> as isize;
            let _ = EnumDisplayMonitors(
                HDC::default(),
                None,
                Some(monitor_enum_proc),
                LPARAM(monitors_ptr),
            );
        }

        if monitors.is_empty() {
            log::warn!("Win32 EnumDisplayMonitors returned no monitors; using 1920×1080 default");
            monitors.push(MonitorRect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            });
        }

        monitors
    }

    /// Callback invoked once per monitor by `EnumDisplayMonitors`.
    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(lparam.0 as *mut Vec<MonitorRect>);

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if GetMonitorInfoW(hmonitor, &mut info).as_bool() {
            // Use rcWork (excludes taskbar) rather than rcMonitor (full area)
            let work = info.rcWork;
            monitors.push(MonitorRect {
                x: work.left as f32,
                y: work.top as f32,
                width: (work.right - work.left) as f32,
                height: (work.bottom - work.top) as f32,
            });
        }

        BOOL(1) // continue enumeration
    }
}

// ── Non-Windows fallback ────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::MonitorRect;

    /// On non-Windows platforms we don't have a reliable way to enumerate
    /// monitors without pulling in extra dependencies, so we return an empty
    /// Vec and let the caller fall back to egui's `monitor_size`.
    pub fn enumerate_monitors() -> Vec<MonitorRect> {
        Vec::new()
    }
}

pub use platform::enumerate_monitors;

/// Return the list of real monitor rects.
/// If the platform API returned nothing, fall back to a single monitor
/// derived from the provided egui `monitor_size` (or 1920×1080).
pub fn get_available_monitors(egui_monitor_size: Option<(f32, f32)>) -> Vec<MonitorRect> {
    let mut monitors = enumerate_monitors();

    if monitors.is_empty() {
        let (w, h) = egui_monitor_size.unwrap_or((1920.0, 1080.0));
        monitors.push(MonitorRect {
            x: 0.0,
            y: 0.0,
            width: w,
            height: h,
        });
    }

    monitors
}

/// Check whether a rectangle (x, y, w, h) is sufficiently visible on at
/// least one of the supplied monitors.
///
/// "Sufficiently visible" means the overlap with some monitor is at least
/// `min_visible` pixels in both dimensions.
pub fn is_visible_on_any_monitor(
    monitors: &[MonitorRect],
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    min_visible: (f32, f32),
) -> bool {
    let rect = MonitorRect {
        x,
        y,
        width,
        height,
    };
    monitors
        .iter()
        .any(|m| m.overlaps(&rect, min_visible.0, min_visible.1))
}

/// Find the monitor whose centre is closest to the given point.
/// Returns the primary (first) monitor if the list has only one entry.
pub fn nearest_monitor(monitors: &[MonitorRect], x: f32, y: f32) -> &MonitorRect {
    monitors
        .iter()
        .min_by(|a, b| {
            let (ax, ay) = a.centre();
            let (bx, by) = b.centre();
            let da = (ax - x).powi(2) + (ay - y).powi(2);
            let db = (bx - x).powi(2) + (by - y).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("monitors list must not be empty")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(x: f32, y: f32, w: f32, h: f32) -> MonitorRect {
        MonitorRect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn overlap_fully_inside() {
        let m = monitor(0.0, 0.0, 1920.0, 1080.0);
        let w = monitor(100.0, 100.0, 800.0, 600.0);
        assert!(m.overlaps(&w, 200.0, 100.0));
    }

    #[test]
    fn overlap_partial_right() {
        // Window straddles right edge: 200px visible on monitor
        let m = monitor(0.0, 0.0, 1920.0, 1080.0);
        let w = monitor(1720.0, 100.0, 800.0, 600.0);
        assert!(m.overlaps(&w, 200.0, 100.0));
    }

    #[test]
    fn no_overlap_off_right() {
        let m = monitor(0.0, 0.0, 1920.0, 1080.0);
        let w = monitor(2000.0, 100.0, 800.0, 600.0);
        assert!(!m.overlaps(&w, 200.0, 100.0));
    }

    #[test]
    fn no_overlap_off_left() {
        let m = monitor(0.0, 0.0, 1920.0, 1080.0);
        let w = monitor(-900.0, 100.0, 800.0, 600.0);
        assert!(!m.overlaps(&w, 200.0, 100.0));
    }

    #[test]
    fn visible_on_second_monitor() {
        let monitors = vec![
            monitor(0.0, 0.0, 1920.0, 1080.0),
            monitor(1920.0, 0.0, 2560.0, 1440.0),
        ];
        assert!(is_visible_on_any_monitor(
            &monitors, 2000.0, 200.0, 800.0, 600.0, (200.0, 100.0)
        ));
    }

    #[test]
    fn not_visible_between_monitors() {
        // Gap scenario (unlikely but tests the logic)
        let monitors = vec![
            monitor(0.0, 0.0, 1920.0, 1080.0),
            monitor(2000.0, 0.0, 1920.0, 1080.0),
        ];
        // Window placed in the 80px gap
        assert!(!is_visible_on_any_monitor(
            &monitors, 1920.0, 0.0, 80.0, 80.0, (200.0, 100.0)
        ));
    }

    #[test]
    fn nearest_monitor_selects_closest() {
        let monitors = vec![
            monitor(0.0, 0.0, 1920.0, 1080.0),
            monitor(1920.0, 0.0, 2560.0, 1440.0),
        ];
        let m = nearest_monitor(&monitors, 3000.0, 500.0);
        assert_eq!(m.x, 1920.0);
    }

    #[test]
    fn fallback_when_no_platform_monitors() {
        let monitors = get_available_monitors(Some((2560.0, 1440.0)));
        // Should always have at least one
        assert!(!monitors.is_empty());
    }
}
