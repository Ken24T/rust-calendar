use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

/// State for the system tray icon
pub struct SystemTray {
    _tray_icon: TrayIcon,
    restore_requested: Arc<AtomicBool>,
    quit_requested: Arc<AtomicBool>,
}

impl SystemTray {
    /// Create a new system tray icon
    pub fn new(icon_data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        // Decode the icon from PNG
        let decoder = png::Decoder::new(icon_data);
        let mut reader = decoder.read_info()?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf)?;

        let icon = Icon::from_rgba(buf, info.width, info.height)?;

        // Create menu
        let menu = Menu::new();
        let show_item = MenuItem::new("Show Calendar", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        menu.append(&show_item)?;
        menu.append(&quit_item)?;

        let restore_requested = Arc::new(AtomicBool::new(false));
        let quit_requested = Arc::new(AtomicBool::new(false));

        let restore_flag = restore_requested.clone();
        let quit_flag = quit_requested.clone();
        let show_id = show_item.id().clone();
        let quit_id = quit_item.id().clone();

        // Set up menu event handler
        std::thread::spawn(move || {
            loop {
                if let Ok(event) = MenuEvent::receiver().recv() {
                    if event.id == show_id {
                        restore_flag.store(true, Ordering::SeqCst);
                    } else if event.id == quit_id {
                        quit_flag.store(true, Ordering::SeqCst);
                    }
                }
            }
        });

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Rust Calendar")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            restore_requested,
            quit_requested,
        })
    }

    /// Check if restore was requested (clears the flag)
    pub fn take_restore_request(&self) -> bool {
        self.restore_requested.swap(false, Ordering::SeqCst)
    }

    /// Check if quit was requested (clears the flag)
    pub fn take_quit_request(&self) -> bool {
        self.quit_requested.swap(false, Ordering::SeqCst)
    }
}
