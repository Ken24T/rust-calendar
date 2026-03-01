# System Tray (Minimise-to-Tray) Implementation Plan

**Branch:** `infrastructure/egui-exploration` (or a new `feature/system-tray`)
**Status:** Planned — not yet implemented
**Date:** 1 March 2026

---

## Problem Statement

Countdown cards become unresponsive when the main application window is minimised.
This happens because all child viewports (countdown card windows) are rendered via
`ctx.show_viewport_immediate()`, which executes synchronously within the main
window's `update()` loop. When the OS stops sending paint events to a minimised
window, the entire update cycle — including all child viewport rendering — stalls.

## Solution

Add a system tray icon. When the user closes or minimises the main window, hide it
instead of truly minimising. The eframe event loop continues running because
`request_repaint_after()` timers remain active, and child viewport windows (countdown
cards) are independent OS windows that stay visible and responsive.

A tray icon provides restore/exit controls. The feature is gated by a new
`minimize_to_tray` setting (default: `false`).

---

## Technical Design

### Crate Selection

Use **`tray-icon`** (maintained by the Tauri team, cross-platform):

```toml
tray-icon = "0.19"
```

Platform support:
| Platform | Mechanism | Works out of the box? |
|----------|-----------|----------------------|
| Linux X11 | `libappindicator3` / XEmbed | Yes (KDE, XFCE, MATE, Cinnamon, LXQt) |
| Linux Wayland | `libappindicator3` / D-Bus StatusNotifierItem | Yes (KDE, most DEs); GNOME needs AppIndicator extension |
| Windows | `Shell_NotifyIcon` (Win32 API) | Yes |

**Linux build dependency** to document:
```bash
# Ubuntu/Debian
sudo apt install libappindicator3-dev
# or the Ayatana fork (Ubuntu 22.04+)
sudo apt install libayatana-appindicator3-dev
```

### Architecture Overview

```
┌──────────────────────────────────────────────────────────┐
│  main()                                                  │
│  ├── load_icon()      (existing — reuse for tray)        │
│  └── eframe::run_native()                                │
│      └── CalendarApp::new(cc)                            │
│          ├── Load settings (minimize_to_tray flag)       │
│          ├── If enabled: create TrayIcon + context menu  │
│          └── Store tray state in CalendarApp fields      │
│                                                          │
│  CalendarApp::handle_update()                            │
│  ├── Poll tray events (MenuEvent, TrayIconEvent)         │
│  ├── Intercept main window close_requested               │
│  │   ├── If minimize_to_tray: CancelClose + Visible(false)│
│  │   └── If exit_requested flag: allow real close        │
│  ├── On tray restore signal: Visible(true) + Focus       │
│  └── On tray exit signal: set exit flag + Close          │
│                                                          │
│  CalendarApp::handle_exit()                              │
│  └── (existing) persist countdowns                       │
└──────────────────────────────────────────────────────────┘
```

### Why Hiding Works

- `ViewportCommand::Visible(false)` hides the OS window without destroying the
  eframe application or terminating its event loop.
- Countdown card rendering already calls `ctx.request_repaint_after(Duration::from_secs(1))`
  (see `src/ui_egui/app/countdown/render.rs` line 114 and `container.rs`), which
  registers timers with the winit event loop. These continue firing regardless of
  main window visibility.
- Child viewports (`show_viewport_immediate`) are separate OS windows. Hiding the
  root viewport does **not** hide them — they remain visible and updating.

---

## Implementation Steps

### Step 1: Add `tray-icon` dependency

**File:** `Cargo.toml`

Add under `[dependencies]` (after the existing `dark-light` entry, around line 72):

```toml
# System tray integration
tray-icon = "0.19"
```

No platform conditionals needed — the crate is cross-platform internally.

After adding, run `cargo check` to pull the dependency and verify compilation.

### Step 2: Add `minimize_to_tray` setting to the model

**File:** `src/models/settings/mod.rs`

Add a new field to the `Settings` struct (after `sync_startup_delay_minutes` at
line 29):

```rust
pub minimize_to_tray: bool,
```

Add to the `Default` impl (after `sync_startup_delay_minutes: 15,` at line 58):

```rust
minimize_to_tray: false,
```

### Step 3: Add database migration for the new column

**File:** `src/services/database/schema.rs`

At the end of `run_settings_migrations()` (before the `let had_time_slot` block at
approximately line 143), add:

```rust
migrations::ensure_column(
    conn,
    "settings",
    "minimize_to_tray",
    "ALTER TABLE settings ADD COLUMN minimize_to_tray INTEGER NOT NULL DEFAULT 0",
)?;
```

### Step 4: Update the settings mapper

**File:** `src/services/settings/mapper.rs`

Add the new field at the end of the `row_to_settings` function, after
`sync_startup_delay_minutes` (which reads column index 21). The new column will be
index 22:

```rust
minimize_to_tray: row.get::<_, i32>(22).unwrap_or(0) != 0,
```

### Step 5: Update the settings service SELECT query

**File:** `src/services/settings/service.rs`

In the `get()` method (line 22–32), add `minimize_to_tray` to the SELECT column
list. The query should become:

```sql
SELECT id, theme, use_system_theme, first_day_of_week, time_format, date_format,
    show_my_day, my_day_position_right, show_ribbon, show_sidebar, show_week_numbers,
    current_view, default_event_duration, first_day_of_work_week, last_day_of_work_week,
    default_event_start_time, default_card_width, default_card_height,
    auto_create_countdown_on_import, edit_before_import, sidebar_width,
    sync_startup_delay_minutes, minimize_to_tray
FROM settings WHERE id = 1
```

### Step 6: Update the settings service UPDATE query

**File:** `src/services/settings/service.rs`

In the `update()` method (lines 44–96), add `minimize_to_tray` to both the SET
clause and the params list:

In the SET clause, add after `sync_startup_delay_minutes = ?21,`:
```rust
                 minimize_to_tray = ?22, \
```

In the `params!` macro, add after `settings.sync_startup_delay_minutes,`:
```rust
                settings.minimize_to_tray as i32,
```

**Important:** The `updated_at = CURRENT_TIMESTAMP` clause must be renumbered if
it uses a positional parameter. Currently it's a literal string, so no renumbering
is needed — just ensure `minimize_to_tray` is the last numbered parameter before
`WHERE id = 1`.

### Step 7: Add settings UI checkbox

**File:** `src/ui_egui/settings_dialog.rs`

In the "View" section, after the "Show ribbon" checkbox (approximately line 360),
add:

```rust
ui.horizontal(|ui| {
    ui.add_space(label_width);
    ui.checkbox(&mut settings.minimize_to_tray, "Minimise to system tray on close")
        .on_hover_text(
            "When enabled, closing the main window hides it to the system tray \
             instead of exiting. Countdown cards remain visible and responsive."
        );
});
```

### Step 8: Add tray-related fields to `CalendarApp`

**File:** `src/ui_egui/app.rs`

Add these imports at the top of the file:

```rust
use tray_icon::{TrayIcon, TrayIconBuilder};
use tray_icon::menu::{Menu, MenuItem, MenuEvent};
use tray_icon::TrayIconEvent;
```

Add these fields to the `CalendarApp` struct (after `calendar_sync_scheduler`):

```rust
/// System tray icon (None when tray is disabled or unavailable)
tray_icon: Option<TrayIcon>,
/// Menu item IDs for tray context menu
tray_show_menu_id: Option<tray_icon::menu::MenuId>,
tray_exit_menu_id: Option<tray_icon::menu::MenuId>,
/// True when the main window is hidden to the tray
hidden_to_tray: bool,
/// Set to true when user explicitly requests exit (File > Exit or tray > Exit)
exit_requested: bool,
```

### Step 9: Create tray icon helper module

**File:** `src/ui_egui/app/tray.rs` (new file)

Create this new module with the tray icon creation logic:

```rust
use super::CalendarApp;
use tray_icon::menu::{Menu, MenuItem, MenuEvent};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

impl CalendarApp {
    /// Attempt to create a system tray icon. Returns None if the tray is
    /// unavailable (e.g. GNOME without AppIndicator extension).
    pub(super) fn create_tray_icon() -> Option<(TrayIcon, tray_icon::menu::MenuId, tray_icon::menu::MenuId)> {
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
    /// Call this at the start of handle_update().
    pub(super) fn poll_tray_events(&mut self, ctx: &egui::Context) {
        // Only poll if tray is active
        if self.tray_icon.is_none() {
            return;
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
            match event {
                TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    ..
                } => {
                    self.restore_from_tray(ctx);
                }
                _ => {}
            }
        }
    }

    /// Intercept the main window close request. If minimize_to_tray is enabled
    /// and the user didn't explicitly request exit, hide to tray instead.
    /// Call this at the start of handle_update(), after poll_tray_events().
    pub(super) fn handle_close_to_tray(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());

        if close_requested
            && self.settings.minimize_to_tray
            && self.tray_icon.is_some()
            && !self.exit_requested
        {
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
    fn restore_from_tray(&mut self, ctx: &egui::Context) {
        if self.hidden_to_tray {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.hidden_to_tray = false;
            log::info!("Main window restored from system tray");
        }
    }

    /// Destroy and recreate the tray icon in response to a settings change.
    /// Call this when the user toggles minimize_to_tray in the settings dialog.
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
                let settings_service = crate::services::settings::SettingsService::new(
                    self.context.database(),
                );
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
```

### Step 10: Register the tray module

**File:** `src/ui_egui/app.rs`

Add the module declaration alongside the other submodules (after `mod toast;`):

```rust
mod tray;
```

### Step 11: Initialise tray state in `CalendarApp::new()`

**File:** `src/ui_egui/app/lifecycle.rs`

In `CalendarApp::new()`, after building the `CalendarApp` struct (where fields are
listed, around lines 64–92), add the new tray fields:

```rust
tray_icon: None,
tray_show_menu_id: None,
tray_exit_menu_id: None,
hidden_to_tray: false,
exit_requested: false,
```

Then, after the struct is constructed but before the return statement (after
`app.focus_on_current_time_if_visible();` at line 97), add:

```rust
// Create system tray icon if the setting is enabled
if app.settings.minimize_to_tray {
    if let Some((tray, show_id, exit_id)) = Self::create_tray_icon() {
        app.tray_icon = Some(tray);
        app.tray_show_menu_id = Some(show_id);
        app.tray_exit_menu_id = Some(exit_id);
    } else {
        log::warn!("System tray unavailable; minimize_to_tray disabled");
        app.settings.minimize_to_tray = false;
    }
}
```

### Step 12: Wire tray event polling into the update loop

**File:** `src/ui_egui/app/lifecycle.rs`

At the **very beginning** of `handle_update()` (line 168, before
`self.handle_file_drops(ctx);`), add:

```rust
// System tray: poll events and intercept close-to-tray
self.poll_tray_events(ctx);
self.handle_close_to_tray(ctx);
```

### Step 13: Update File > Exit to set exit flag

**File:** `src/ui_egui/app/menu.rs`

Change the Exit button handler (line 48–49) from:

```rust
if ui.button("Exit").clicked() {
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
}
```

to:

```rust
if ui.button("Exit").clicked() {
    self.exit_requested = true;
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
}
```

This ensures File > Exit always truly exits, even when minimize_to_tray is enabled.

### Step 14: Sync tray icon when settings change

**File:** `src/ui_egui/app/lifecycle.rs` (or wherever settings are saved after the
settings dialog closes)

Search for the code that applies settings after the dialog closes. Look in
`src/ui_egui/app/dialogs.rs` or `handle_dialogs()`. After settings are persisted
(after `settings_service.update(&self.settings)` is called), add:

```rust
self.sync_tray_to_settings();
```

This handles the case where the user enables/disables minimize_to_tray in settings
at runtime — the tray icon is created or destroyed immediately without requiring an
app restart.

To locate the exact insertion point, search for:
```
settings_service.update
```
in the settings dialog save path. You will likely find it in
`src/ui_egui/app/dialogs.rs` or `src/ui_egui/settings_dialog.rs` in the save/apply
handler.

### Step 15: Update Linux build dependencies documentation

**File:** `.github/copilot-instructions.md`

In the "Environment and Dependencies" section, add `libappindicator3-dev` (or
`libayatana-appindicator3-dev`) to the Linux build dependencies list:

```
- **Linux build dependencies**: `build-essential libgtk-3-dev libxcb-render0-dev
  libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
  libayatana-appindicator3-dev`
```

**File:** `README.md`

Add the same dependency to any Linux build instructions section.

### Step 16: Update user documentation

**File:** `docs/USER_GUIDE.md`

Add a new "System Tray" subsection (after the existing "Backup" section, before
"Themes"). Example text:

```markdown
### System Tray

- **Enable**: Go to **Edit > Settings > View** and tick "Minimise to system tray
  on close"
- **Behaviour**: When enabled, closing the main window hides it to the system
  tray instead of exiting. Countdown cards remain visible and responsive.
- **Restore**: Click the tray icon or right-click and select "Show Calendar"
- **Exit**: Right-click the tray icon and select "Exit", or use **File > Exit**
  (always exits regardless of the tray setting)
- **Linux note**: Requires a desktop environment with system tray support (KDE,
  XFCE, MATE, Cinnamon, LXQt). On GNOME, install the
  [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/).
  If no tray host is detected, the setting is automatically disabled.
```

**File:** `docs/FEATURES.md`

Add to the features list: "System tray integration — keep countdown cards alive
while the main window is hidden"

---

## File Change Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `Cargo.toml` | Edit | Add `tray-icon = "0.19"` dependency |
| `src/models/settings/mod.rs` | Edit | Add `minimize_to_tray: bool` field + default |
| `src/services/database/schema.rs` | Edit | Add migration for `minimize_to_tray` column |
| `src/services/settings/mapper.rs` | Edit | Map column index 22 to `minimize_to_tray` |
| `src/services/settings/service.rs` | Edit | Add column to SELECT and UPDATE queries |
| `src/ui_egui/settings_dialog.rs` | Edit | Add checkbox in View section |
| `src/ui_egui/app.rs` | Edit | Add tray fields to struct + import + mod declaration |
| `src/ui_egui/app/tray.rs` | **New** | Tray icon creation, event polling, close intercept |
| `src/ui_egui/app/lifecycle.rs` | Edit | Init tray in `new()`, poll in `handle_update()` |
| `src/ui_egui/app/menu.rs` | Edit | Set `exit_requested = true` in File > Exit |
| `src/ui_egui/app/dialogs.rs` | Edit | Call `sync_tray_to_settings()` after save |
| `.github/copilot-instructions.md` | Edit | Add `libayatana-appindicator3-dev` to deps |
| `docs/USER_GUIDE.md` | Edit | Add System Tray section |
| `docs/FEATURES.md` | Edit | Add tray feature to list |

## Current File State Reference

These are the key structural details of files as they exist at v2.2.1 (the base
for this implementation). Use these to locate insertion points:

### `Cargo.toml`
- Version: `2.2.1`
- Last dependency entry before `[dev-dependencies]`: `dark-light = "1.1"` (line 72)
- Windows-specific deps at lines 100–106

### `src/models/settings/mod.rs`
- `Settings` struct lines 7–30 (27 fields)
- Last field: `sync_startup_delay_minutes: i64` (line 29)
- `Default` impl lines 32–59
- Last default: `sync_startup_delay_minutes: 15` (line 58)

### `src/services/database/schema.rs`
- `run_settings_migrations()` starts at line 53
- Last migration before the `had_time_slot` block: `sync_startup_delay_minutes`
  at approximately line 139
- Insert new migration before the `let had_time_slot` line (~line 143)

### `src/services/settings/mapper.rs`
- `row_to_settings()` reads 22 columns (indices 0–21)
- Last mapped field: `sync_startup_delay_minutes: row.get::<_, i64>(21).unwrap_or(15)` (line 30)

### `src/services/settings/service.rs`
- `get()` SELECT query at lines 22–31 (22 columns currently)
- `update()` UPDATE query at lines 48–96 (21 params: ?1 to ?21)

### `src/ui_egui/app.rs`
- `CalendarApp` struct at lines 37–91 (22 fields)
- Last field: `calendar_sync_scheduler: Arc<Mutex<CalendarSyncScheduler>>`
- Module declarations at lines 1–20
- `impl eframe::App` at line 93

### `src/ui_egui/app/lifecycle.rs`
- `CalendarApp::new()` at lines 24–99
- Struct construction at lines 64–92
- `handle_update()` at lines 167–271
- First line of update: `self.handle_file_drops(ctx);` (line 168)
- `handle_exit()` at lines 273–275

### `src/ui_egui/app/menu.rs`
- File > Exit at lines 48–49: sends `ViewportCommand::Close`

### `src/ui_egui/settings_dialog.rs`
- View section checkboxes at lines 329–365
- "Show ribbon" checkbox at approximately line 356
- "Card" heading section starts around line 370

---

## Verification Checklist

After implementation, verify:

1. **`cargo test`** — all existing tests pass (316+)
2. **`cargo clippy`** — zero warnings
3. **Manual: Setting off (default)**
   - Close main window → app exits normally
   - No tray icon visible
4. **Manual: Setting on, tray available**
   - Tray icon appears in system tray
   - Close main window → window hides, tray icon remains
   - Countdown cards remain visible and ticking
   - Left-click tray icon → main window restores + gains focus
   - Right-click tray > "Show Calendar" → same restore behaviour
   - Right-click tray > "Exit" → app fully exits
   - File > Exit → app fully exits (bypasses hide-to-tray)
5. **Manual: Setting toggled at runtime**
   - Enable in settings → tray icon appears immediately
   - Disable in settings → tray icon disappears immediately
   - If hidden when disabled → main window restores
6. **Manual: Tray unavailable (GNOME without extension)**
   - Enabling setting fails gracefully → setting reverts to off
   - Log message indicates tray host not available
   - App continues working normally
7. **Cross-compilation check**
   - `cargo check` passes (verify no Linux-only API leakage)
   - Windows-specific code paths not broken

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| GNOME users can't use the feature | Medium | Default off; tooltip explains requirement; graceful fallback |
| `tray-icon` API changes | Low | Pin version in `Cargo.toml`; crate is mature |
| LTO build time increase | Low | Negligible — one small additional crate |
| Frame-timing with hidden root | Very Low | Already tested: `request_repaint_after` timers work when hidden |
| Child viewports freeze despite hiding | Very Low | `Visible(false)` ≠ minimise; event loop keeps running |

---

## Future Considerations

- **Tooltip enhancement:** Show the nearest countdown card's remaining time in
  the tray tooltip text (update on each repaint).
- **Deferred viewports:** A separate branch could later migrate
  `show_viewport_immediate()` to `show_viewport_deferred()` for truly independent
  card rendering threads. This is complementary, not required.
- **Start minimised:** Add a "Start minimised to tray" option for users who want
  the app to start hidden with only countdown cards visible.
