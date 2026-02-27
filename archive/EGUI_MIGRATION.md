# egui Migration Guide

## Overview

This document tracks the migration from iced to egui for the Rust Calendar application.

## Motivation

The iced framework has several limitations that were becoming blockers:
- No double-click event support
- Limited styling flexibility (no closure-based styling on buttons)
- Complex custom widget creation
- Restrictive widget composition patterns

egui offers:
- Immediate-mode GUI with simpler interaction handling
- Full double-click support via `response.double_clicked()`
- Flexible styling and customization
- Simpler widget creation and composition
- Better documentation and community support

## Migration Status

### ✅ Completed

1. **Dependency Updates**
   - Replaced `iced` and `iced_aw` with `egui`, `eframe`, and `egui_extras`
   - Updated rust toolchain to stable for edition2024 compatibility
   - All other dependencies (rusqlite, chrono, etc.) remain unchanged

2. **Basic App Structure**
   - Created `src/ui_egui/` directory
   - Implemented `CalendarApp` with eframe::App trait
   - Set up database with 'static lifetime (using Box::leak for compatibility)
   - Basic state management (current_view, current_date, dialog states)

3. **Menu Bar**
   - File menu (Settings, Exit)
   - View menu (Day, Week, WorkWeek, Month, Quarter with view switching)
   - Theme menu (placeholder)
   - Events menu (New Event placeholder)

4. **Navigation**
   - Previous/Today/Next buttons
   - View-specific date navigation (day, week, month, quarter)

5. **Keyboard Shortcuts**
   - ESC key closes dialogs without saving

6. **Branch Management**
   - Committed all iced work to main
   - Created `refactor/egui` branch
   - Old iced UI code commented out for reference

### 🚧 In Progress / Pending

7. **Views** (Priority: High)
   - [ ] Day view with time slots and event rendering
   - [ ] Week view with 7-day grid
   - [ ] WorkWeek view with 5-day grid
   - [ ] Month view with calendar grid
   - [ ] Quarter view with mini-calendars

8. **Event Dialog** (Priority: High)
   - [ ] Form fields (title, description, location, etc.)
   - [ ] Date/time pickers using egui widgets
   - [ ] Recurrence options with BYDAY checkboxes
   - [ ] Validation and error display
   - [ ] Save/Cancel buttons

9. **Settings Dialog** (Priority: Medium)
   - [ ] Theme selection
   - [ ] Work week boundaries
   - [ ] Default event start time
   - [ ] Time slot interval
   - [ ] First day of week

10. **Theme System** (Priority: Medium)
    - [ ] Theme picker dialog
    - [ ] Load themes from TOML files
    - [ ] Apply custom colors to egui::Visuals
    - [ ] Theme preview

11. **Event Interaction** (Priority: High)
    - [ ] Double-click to create event (now possible with egui!)
    - [ ] Click event to view/edit
    - [ ] Drag-and-drop to reschedule
    - [ ] Context menu for quick actions

12. **Testing** (Priority: Medium)
    - [ ] Verify all event CRUD operations
    - [ ] Test recurrence expansion
    - [ ] Validate theme switching
    - [ ] Check settings persistence

## Technical Notes

### Database Lifetime Management

egui requires the App implementation to have 'static lifetime, but our services need references to the Database. Solution:
```rust
let database = Box::leak(Box::new(Database::new("calendar.db")?));
```
This leaks the Database for the program's lifetime, which is acceptable for a desktop application.

### Service Access Pattern

Instead of storing services in the app struct, we create them on-demand:
```rust
let event_service = EventService::new(self.database);
// use event_service...
```

### egui Advantages Over iced

1. **Double-Click Support**
   ```rust
   if response.double_clicked() {
       // Handle double-click
   }
   ```

2. **Flexible Styling**
   - Can style individual widgets with full control
   - No need for predefined theme enum values
   - Direct color/size/padding manipulation

3. **Simpler Interaction**
   ```rust
   if ui.button("Click me").clicked() {
       // Handle click
   }
   ```

4. **Grid Layout**
   ```rust
   egui::Grid::new("my_grid")
       .striped(true)
       .show(ui, |ui| {
           // Grid content
       });
   ```

### Migration Strategy

1. Keep models and services unchanged (already framework-agnostic)
2. Port views one at a time, testing thoroughly
3. Implement dialogs after views are working
4. Add advanced interactions (drag-drop, context menus)
5. Polish and optimize

### References

- egui documentation: https://docs.rs/egui/
- eframe examples: https://github.com/emilk/egui/tree/master/examples
- egui_extras widgets: https://docs.rs/egui_extras/

## Next Steps

The immediate priority is to port the calendar views:
1. Start with Month view (simpler grid layout)
2. Then Day view (time slots with ScrollArea)
3. Week and WorkWeek views (multi-column time grids)
4. Quarter view (3 mini month grids)

After views are functional, implement the Event Dialog with proper egui widgets for all form fields and recurrence options.
