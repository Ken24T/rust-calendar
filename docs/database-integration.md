# Database and Settings Integration

## Overview
The calendar application now has full database integration with persistent settings. User preferences are automatically saved to a SQLite database and restored on application launch.

## Architecture

### Database Layer
- **Location**: `src/services/database/mod.rs`
- **Database File**: `calendar.db` (in current directory)
- **Schema**: Single `settings` table with columns:
  - `id` (PRIMARY KEY)
  - `theme` (TEXT: "light" or "dark")
  - `first_day_of_week` (INTEGER: 0-6)
  - `time_format` (TEXT: "12h" or "24h")
  - `date_format` (TEXT)
  - `show_my_day` (BOOLEAN)
  - `show_ribbon` (BOOLEAN)
  - `current_view` (TEXT: "Day", "WorkWeek", "Week", "Month", "Quarter")
  - `created_at`, `updated_at` (TIMESTAMP)

### Settings Service
- **Location**: `src/services/settings/mod.rs`
- **Operations**:
  - `get()` - Load settings from database
  - `update(&Settings)` - Save settings with validation
  - `reset()` - Restore defaults

### UI Integration
- **Location**: `src/ui/app.rs`
- **Changes**:
  - Added `db: Arc<Mutex<Database>>` field to CalendarApp
  - Changed `Flags` from `()` to `String` (database path)
  - Enhanced `new()` to initialize DB and load settings
  - Added `save_settings()` method to persist UI state
  - Modified `update()` to save on all state changes

## Features

### Automatic Loading
On application startup:
1. Database is opened/created at specified path
2. Schema is initialized if needed
3. Settings are loaded (or defaults if first run)
4. UI state is set from loaded settings:
   - Theme (Light/Dark)
   - My Day panel visibility
   - Ribbon visibility
   - Current view (Day/WorkWeek/Week/Month/Quarter)

### Automatic Saving
Settings are automatically saved when user:
- Toggles theme (☀️/🌙 button)
- Toggles My Day panel (View > My Day)
- Toggles Ribbon (View > Ribbon)
- Switches view (View > Day/Work Week/Week/Month/Quarter)

### Error Handling
- Database initialization failures fall back to in-memory database
- Settings load failures use default values
- Save failures are logged to stderr but don't crash the app

## Testing

### Unit Tests (21 tests)
- Database connection (in-memory and file-based)
- Schema initialization
- Foreign key constraints
- Settings validation (theme, time format, view types)
- CRUD operations (get, update, reset)
- Boolean field persistence
- View type changes

### Integration Tests (3 tests)
- Settings persistence across operations
- App lifecycle simulation (save and reload)
- All view types persistence

### Manual Testing
1. Run the application: `cargo run`
2. Toggle theme - should switch between light and dark
3. Close application
4. Run again - theme should persist
5. Change view to "Week" - should switch
6. Close and reopen - view should still be "Week"
7. Toggle My Day panel
8. Close and reopen - panel state should persist

## Files Modified
- `src/ui/app.rs` - Database integration, settings loading/saving
- `src/main.rs` - Database path configuration
- `src/services/database/mod.rs` - Fixed test imports
- `tests/integration_test.rs` (NEW) - Integration tests

## Database Location
Currently stored in the application directory as `calendar.db`. 
TODO: Move to proper application data directory using `directories` crate.

## Test Results
```
Unit tests:     21 passed
Integration:     3 passed
Total:          24 passed
```

All tests verify:
- Database operations work correctly
- Settings validation prevents invalid data
- CRUD operations persist data
- UI state changes trigger saves
- Settings survive app restarts
