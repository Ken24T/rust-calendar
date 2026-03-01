use super::CalendarApp;
use tray_icon::menu::{Menu, MenuItem, MenuEvent};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

impl CalendarApp {
    /// Attempt to create a system tray icon. Returns `None` if the tray is
    /// unavailable (e.g. GNOME without AppIndicator extension).
    pub(super) fn create_tray_icon(
    ) -> Option<(TrayIcon, tray_icon::menu::MenuId, tray_icon::menu::MenuId)> {
        // GTK must be initialised before tray-icon creates menus on Linux
        #[cfg(target_os = "linux")]
        {
            if gtk::init().is_err() {
                log::warn!("Failed to initialise GTK for system tray");
                return None;
            }
        }

        let show_item = MenuItem::new("Show Calendar", true, None);
        let exit_item = MenuItem::new("Exit", true, None);
        let show_id = show_item.id().clone();
        let exit_id = exit_item.id().clone();

        let menu = Menu::new();
        if menu.append(&show_item).is_err() || menu.append(&exit_item).is_err() {
            log::warn!("Failed to build tray context menu");
            return None;
        }

        // Load the same icon used for the window
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

        match TrayIconBuilder::new()
            .with_tooltip("Rust Calendar")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
        {
            Ok(tray) => {
                log::info!("System tray icon created successfully");
                Some((tray, show_id, exit_id))
            }
            Err(e) => {
                log::warn!(
                    "Failed to create system tray icon (tray host may not be available): {e}"
                );
                None
            }
        }
    }

    /// Poll tray events and handle show/exit actions.
    /// Call this at the start of `handle_update()`.
    pub(super) fn poll_tray_events(&mut self, ctx: &egui::Context) {
        // Only poll if tray is active
        if self.tray_icon.is_none() {
            return;
        }

        // Process pending GTK events so libappindicator can handle D-Bus
        // registration and menu interactions. Without this, the tray icon
        // never becomes visible because egui/winit doesn't run a GTK loop.
        #[cfg(target_os = "linux")]
        {
            while gtk::events_pending() {
                gtk::main_iteration();
            }
        }

        // Poll menu events (right-click menu)
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if Some(&event.id) == self.tray_show_menu_id.as_ref() {
                self.restore_from_tray(ctx);
            } else if Some(&event.id) == self.tray_exit_menu_id.as_ref() {
                self.exit_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Poll tray icon events (left-click on icon)
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                ..
            } = event
            {
                self.restore_from_tray(ctx);
            }
        }
    }

    /// Intercept the main window close request. If `minimize_to_tray` is enabled
    /// and the user didn't explicitly request exit, hide to tray instead.
    /// Call this at the start of `handle_update()`, after `poll_tray_events()`.
    pub(super) fn handle_close_to_tray(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());

        if close_requested
            && self.settings.minimize_to_tray
            && self.tray_icon.is_some()
            && !self.exit_requested
        {
            // Save the current window position so we can restore it later
            self.tray_saved_outer_position = ctx.input(|i| i.viewport().outer_rect)
                .map(|rect| rect.min);

            // Cancel the real close, hide to tray instead
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.hidden_to_tray = true;
            log::info!("Main window hidden to system tray");
        }
        // If exit_requested is true, let the close proceed normally
        // (eframe will call on_exit / handle_exit)
    }

    /// Show the main window and bring it to focus.
    pub(super) fn restore_from_tray(&mut self, ctx: &egui::Context) {
        if self.hidden_to_tray {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));

            // Restore the saved position so the window reappears where it was
            if let Some(pos) = self.tray_saved_outer_position.take() {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
            }

            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.hidden_to_tray = false;
            log::info!("Main window restored from system tray");
        }
    }

    /// Destroy and recreate the tray icon in response to a settings change.
    /// Call this when the user toggles `minimize_to_tray` in the settings dialog.
    pub(super) fn sync_tray_to_settings(&mut self) {
        if self.settings.minimize_to_tray && self.tray_icon.is_none() {
            // Setting turned ON — create tray
            if let Some((tray, show_id, exit_id)) = Self::create_tray_icon() {
                self.tray_icon = Some(tray);
                self.tray_show_menu_id = Some(show_id);
                self.tray_exit_menu_id = Some(exit_id);
            } else {
                log::warn!(
                    "Tray icon could not be created — disabling minimize_to_tray setting"
                );
                self.settings.minimize_to_tray = false;
                // Persist the reverted setting
                let settings_service =
                    crate::services::settings::SettingsService::new(self.context.database());
                let _ = settings_service.update(&self.settings);
            }
        } else if !self.settings.minimize_to_tray && self.tray_icon.is_some() {
            // Setting turned OFF — destroy tray
            self.tray_icon = None;
            self.tray_show_menu_id = None;
            self.tray_exit_menu_id = None;
            // If currently hidden, restore first
            if self.hidden_to_tray {
                self.hidden_to_tray = false;
                // Window visibility will be restored on next frame
            }
            log::info!("System tray icon removed");
        }
    }
}
