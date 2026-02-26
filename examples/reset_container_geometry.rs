//! View or reset container geometry to primary monitor position
//! Run with:
//!   cargo run --example reset_container_geometry        # Just show current geometry
//!   cargo run --example reset_container_geometry reset  # Reset to (100, 100)

use rusqlite::Connection;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let should_reset = args.len() > 1 && args[1] == "reset";
    
    // In debug mode, database is in current directory
    // In release mode, it's in APPDATA
    let debug_path = PathBuf::from("calendar.db");
    let release_path = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("rust-calendar")
        .join("calendar.db");
    
    let db_path = if debug_path.exists() {
        debug_path
    } else {
        release_path
    };

    println!("Opening database at: {:?}", db_path);

    let conn = Connection::open(&db_path).expect("Failed to open database");

    // Check current values
    let result = conn.query_row(
        "SELECT container_geometry_x, container_geometry_y, container_geometry_width, container_geometry_height FROM countdown_settings WHERE id = 1",
        [],
        |row| {
            let x: Option<f64> = row.get(0)?;
            let y: Option<f64> = row.get(1)?;
            let w: Option<f64> = row.get(2)?;
            let h: Option<f64> = row.get(3)?;
            Ok((x, y, w, h))
        },
    );

    match result {
        Ok((x, y, w, h)) => {
            println!("Current container geometry:");
            println!("  x: {:?}", x);
            println!("  y: {:?}", y);
            println!("  width: {:?}", w);
            println!("  height: {:?}", h);
        }
        Err(e) => {
            println!("Error reading geometry: {}", e);
        }
    }

    if should_reset {
        // Reset to primary monitor position
        println!("\nResetting container geometry to (100, 100, 200, 400)...");
        
        conn.execute(
            "UPDATE countdown_settings SET container_geometry_x = 100, container_geometry_y = 100, container_geometry_width = 200, container_geometry_height = 400 WHERE id = 1",
            [],
        ).expect("Failed to update geometry");

        println!("Done! Container will appear at (100, 100) on next app start.");
    } else {
        println!("\nTo reset geometry, run: cargo run --example reset_container_geometry reset");
    }
}
