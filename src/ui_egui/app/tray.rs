use super::CalendarApp;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{TrayIconBuilder, TrayIconEvent};

// ── Tray action flags ─────────────────────────────────────────────────────

const TRAY_ACTION_NONE: u8 = 0;
const TRAY_ACTION_SHOW: u8 = 1;
const TRAY_ACTION_EXIT: u8 = 2;

/// Timer ID for the keep-alive timer (arbitrary non-zero value).
#[cfg(target_os = "windows")]
const TRAY_KEEPALIVE_TIMER_ID: usize = 0xCA1E;

/// Result of creating a tray icon, including the shared action flag.
pub(super) struct TrayIconResult {
    pub tray: tray_icon::TrayIcon,
    pub show_id: MenuId,
    pub exit_id: MenuId,
    pub action_flag: Arc<AtomicU8>,
}

// ── Platform helpers ──────────────────────────────────────────────────────

/// Find the main application window by its title (Windows only).
#[cfg(target_os = "windows")]
pub(super) fn find_main_window_hwnd() -> isize {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    let title: Vec<u16> = "Rust Calendar\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(PCWSTR(std::ptr::null()), PCWSTR(title.as_ptr())) };
    hwnd.0
}

#[cfg(not(target_os = "windows"))]
pub(super) fn find_main_window_hwnd() -> isize {
    0
}

/// Get the window position in screen pixels (for pixel-perfect restore).
#[cfg(target_os = "windows")]
fn get_window_pixel_pos(hwnd_value: isize) -> Option<(i32, i32)> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    if hwnd_value == 0 {
        return None;
    }
    let mut rect = windows::Win32::Foundation::RECT::default();
    match unsafe { GetWindowRect(HWND(hwnd_value), &mut rect) } {
        Ok(()) => Some((rect.left, rect.top)),
        Err(_) => None,
    }
}

/// Move the window off-screen and hide its taskbar button, then install
/// a keep-alive timer so that winit's message pump stays active.
#[cfg(target_os = "windows")]
fn hide_window_offscreen(hwnd_value: isize) -> i32 {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::*;

    if hwnd_value == 0 {
        log::warn!("Cannot hide window: HWND is null");
        return 0;
    }
    let hwnd = HWND(hwnd_value);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            -32000,
            -32000,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let orig = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let hidden = ((orig as u32) | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
        SetWindowLongW(hwnd, GWL_EXSTYLE, hidden as i32);

        // Install a keep-alive timer.  WM_TIMER messages always wake
        // WaitMessage/GetMessage, ensuring the winit event loop keeps
        // pumping even though the window has no visible area.
        let _ = SetTimer(hwnd, TRAY_KEEPALIVE_TIMER_ID, 200, None);

        orig
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_window_offscreen(_hwnd_value: isize) -> i32 {
    0
}

/// Move the window back on-screen, restore its taskbar button, and kill
/// the keep-alive timer.
#[cfg(target_os = "windows")]
fn show_window_onscreen(hwnd_value: isize, orig_exstyle: i32, x: i32, y: i32) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::*;

    if hwnd_value == 0 {
        log::warn!("Cannot show window: HWND is null");
        return;
    }
    let hwnd = HWND(hwnd_value);
    unsafe {
        let _ = KillTimer(hwnd, TRAY_KEEPALIVE_TIMER_ID);
        let _ = SetWindowPos(
            hwnd,
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        SetWindowLongW(hwnd, GWL_EXSTYLE, orig_exstyle);
        let _ = SetForegroundWindow(hwnd);
    }
}

#[cfg(not(target_os = "windows"))]
fn show_window_onscreen(_hwnd_value: isize, _orig_exstyle: i32, _x: i32, _y: i32) {}

impl CalendarApp {
    /// Attempt to create a system tray icon with a context menu.
    pub(super) fn create_tray_icon(ctx: &egui::Context) -> Option<TrayIconResult> {
        #[cfg(target_os = "linux")]
        {
            if gtk::init().is_err() {
                log::warn!("Failed to initialise GTK for system tray");
                return None;
            }
        }

        // Fixed IDs so they stay consistent across create/destroy cycles
        let show_id = MenuId::new("rc_tray_show");
        let exit_id = MenuId::new("rc_tray_exit");
        let show_item = MenuItem::with_id(show_id.clone(), "Show Calendar", true, None);
        let exit_item = MenuItem::with_id(exit_id.clone(), "Exit", true, None);

        let menu = Menu::new();
        if menu.append(&show_item).is_err() || menu.append(&exit_item).is_err() {
            log::warn!("Failed to build tray context menu");
            return None;
        }

        let icon_bytes = include_bytes!("../../../assets/icons/663353.png");
        let decoder = png::Decoder::new(&icon_bytes[..]);
        let mut reader = match decoder.read_info() {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Failed to decode tray icon PNG: {e}");
                return None;
            }
        };
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = match reader.next_frame(&mut buf) {
            Ok(i) => i,
            Err(e) => {
                log::warn!("Failed to read tray icon frame: {e}");
                return None;
            }
        };
        let icon = match tray_icon::Icon::from_rgba(buf, info.width, info.height) {
            Ok(i) => i,
            Err(e) => {
                log::warn!("Failed to create tray icon from RGBA data: {e}");
                return None;
            }
        };

        // Shared atomic flag — event handlers write, poll_tray_events reads.
        let action_flag = Arc::new(AtomicU8::new(TRAY_ACTION_NONE));

        // ── Menu event handler ────────────────────────────────────────────
        let show_id_h = show_id.clone();
        let exit_id_h = exit_id.clone();
        let af = action_flag.clone();
        let ctx_m = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            log::info!("[event handler] Menu event: {:?}", event.id);
            if event.id == show_id_h {
                af.store(TRAY_ACTION_SHOW, Ordering::SeqCst);
            } else if event.id == exit_id_h {
                af.store(TRAY_ACTION_EXIT, Ordering::SeqCst);
            }
            ctx_m.request_repaint();
        }));

        // ── Tray icon click handler ───────────────────────────────────────
        let af2 = action_flag.clone();
        let ctx_t = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                ..
            } = event
            {
                log::info!("[event handler] Left-click on tray icon");
                af2.store(TRAY_ACTION_SHOW, Ordering::SeqCst);
                ctx_t.request_repaint();
            }
        }));

        match TrayIconBuilder::new()
            .with_tooltip("Rust Calendar")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
        {
            Ok(tray) => {
                log::info!("System tray icon created successfully");
                Some(TrayIconResult {
                    tray,
                    show_id,
                    exit_id,
                    action_flag,
                })
            }
            Err(e) => {
                log::warn!("Failed to create system tray icon: {e}");
                None
            }
        }
    }

    /// Poll tray events via the shared action flag.
    ///
    /// The window is moved off-screen (not hidden), so the winit event loop
    /// stays active and `update()` keeps being called.  Event handlers just
    /// set the atomic flag and `request_repaint`; no `ShowWindow` hacks needed.
    pub(super) fn poll_tray_events(&mut self, ctx: &egui::Context) {
        if self.tray_icon.is_none() {
            return;
        }

        #[cfg(target_os = "linux")]
        {
            while gtk::events_pending() {
                gtk::main_iteration();
            }
        }

        if let Some(flag) = &self.tray_action_flag {
            let action = flag.swap(TRAY_ACTION_NONE, Ordering::SeqCst);
            match action {
                TRAY_ACTION_SHOW => {
                    log::info!("Tray action: Show Calendar");
                    self.restore_from_tray(ctx);
                }
                TRAY_ACTION_EXIT => {
                    log::info!("Tray action: Exit");
                    self.exit_requested = true;
                    self.restore_from_tray(ctx);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                _ => {}
            }
        }
    }

    /// Intercept the main window close request.  If `minimize_to_tray` is
    /// enabled and the user didn't explicitly request exit, hide to tray.
    pub(super) fn handle_close_to_tray(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());

        if close_requested && self.settings.minimize_to_tray && !self.exit_requested {
            // Lazily create the tray icon (kept alive across cycles)
            if self.tray_icon.is_none() {
                if self.tray_hwnd == 0 {
                    self.tray_hwnd = find_main_window_hwnd();
                    log::info!("Cached main window HWND: {}", self.tray_hwnd);
                }

                if let Some(result) = Self::create_tray_icon(ctx) {
                    self.tray_icon = Some(result.tray);
                    self.tray_show_menu_id = Some(result.show_id);
                    self.tray_exit_menu_id = Some(result.exit_id);
                    self.tray_action_flag = Some(result.action_flag);
                    log::info!("Tray icon created (first hide)");
                } else {
                    log::warn!("System tray unavailable; cannot minimise to tray");
                    return;
                }
            } else {
                log::info!("Reusing existing tray icon for hide");
            }

            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);

            // Save pixel position for restore, then move off-screen
            #[cfg(target_os = "windows")]
            {
                self.tray_saved_pixel_pos = get_window_pixel_pos(self.tray_hwnd);
                self.tray_original_exstyle = hide_window_offscreen(self.tray_hwnd);
                log::info!(
                    "Window moved off-screen (saved pos: {:?}, exstyle: {})",
                    self.tray_saved_pixel_pos,
                    self.tray_original_exstyle
                );
            }

            // On non-Windows, fall back to eframe's visibility
            #[cfg(not(target_os = "windows"))]
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));

            self.hidden_to_tray = true;
            log::info!("Main window hidden to system tray");
        }
    }

    /// Show the main window, keeping the tray icon alive for reuse.
    ///
    /// The tray icon and its event handlers are kept so that the same
    /// handlers (with valid `ctx` clones) are reused on subsequent
    /// hide/show cycles.  Only `sync_tray_to_settings` or an explicit
    /// exit destroys the tray.
    pub(super) fn restore_from_tray(&mut self, ctx: &egui::Context) {
        if self.hidden_to_tray {
            // Move window back on-screen and restore taskbar button
            #[cfg(target_os = "windows")]
            {
                let (x, y) = self.tray_saved_pixel_pos.unwrap_or((100, 100));
                show_window_onscreen(self.tray_hwnd, self.tray_original_exstyle, x, y);
            }

            #[cfg(not(target_os = "windows"))]
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));

            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.hidden_to_tray = false;

            // NOTE: tray icon is intentionally kept alive.  Destroying and
            // re-creating it between cycles causes the replacement event
            // handlers' ctx.request_repaint() to fail to wake winit on the
            // second cycle.

            log::info!("Main window restored from system tray (tray icon kept alive)");
        }
    }

    /// Handle a settings change for `minimize_to_tray`.
    pub(super) fn sync_tray_to_settings(&mut self) {
        if !self.settings.minimize_to_tray && self.tray_icon.is_some() {
            self.tray_icon = None;
            self.tray_show_menu_id = None;
            self.tray_exit_menu_id = None;
            self.tray_action_flag = None;
            MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
            TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
            if self.hidden_to_tray {
                self.hidden_to_tray = false;
                #[cfg(target_os = "windows")]
                {
                    let (x, y) = self.tray_saved_pixel_pos.unwrap_or((100, 100));
                    show_window_onscreen(self.tray_hwnd, self.tray_original_exstyle, x, y);
                }
            }
            log::info!("System tray icon removed (setting disabled)");
        }
    }
}
