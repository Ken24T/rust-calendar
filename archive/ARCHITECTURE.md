# Rust Calendar Architecture

## Overview

This document describes the architectural design of the Rust Calendar application, including component relationships, data flow, and design decisions.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        User Interface                        │
│                         (iced GUI)                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Calendar View│  │  Event View  │  │Settings View │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            │ Messages/Commands
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Application State                       │
│                    (Reactive State Model)                    │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            │ Service Calls
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Service Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │Event Service │  │Reminder Svc  │  │ Theme Svc    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            │ Data Operations
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Data Layer                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  SQLite DB   │  │  iCal I/O    │  │Windows Notif │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. User Interface Layer

**Framework**: iced (Reactive UI)

**Responsibilities**:
- Render calendar views and UI components
- Handle user input and interactions
- Dispatch messages to update application state
- Apply themes and styling

**Key Components**:
- `CalendarView`: Month/week/day calendar display
- `EventView`: Event creation and editing forms
- `SettingsView`: Application settings and preferences
- `ReminderView`: Reminder configuration

### 2. Application State

**Pattern**: Elm Architecture (Model-Update-View)

**Responsibilities**:
- Maintain current application state
- Process messages and update state
- Coordinate between UI and services
- Handle asynchronous operations

**State Structure**:
```rust
struct AppState {
    current_date: NaiveDate,
    view_mode: ViewMode,  // Month, Week, Day
    events: Vec<Event>,
    selected_event: Option<Event>,
    theme: Theme,
    settings: Settings,
}
```

### 3. Service Layer

#### Event Service
- CRUD operations for events
- Recurrence rule processing
- Event querying and filtering
- Conflict detection

#### Reminder Service
- Background reminder checking
- Notification scheduling
- Reminder persistence
- Snooze handling

#### Database Service
- SQLite connection management
- Transaction handling
- Migration management
- Query optimization

#### Notification Service
- Windows notification integration
- Notification display and interaction
- System tray management

#### Theme Service
- Theme loading and parsing
- Theme application
- Custom theme management

### 4. Data Models

#### Event Model
```rust
struct Event {
    id: Option<i64>,
    title: String,
    description: Option<String>,
    location: Option<String>,
    start: DateTime<Local>,
    end: DateTime<Local>,
    is_all_day: bool,
    category: Option<String>,
    color: Option<String>,
    recurrence: Option<RecurrenceRule>,
    reminders: Vec<Reminder>,
}
```

#### Recurrence Rule
```rust
struct RecurrenceRule {
    frequency: Frequency,  // Daily, Weekly, Monthly, Yearly
    interval: u32,
    count: Option<u32>,
    until: Option<DateTime<Local>>,
    by_day: Option<Vec<Weekday>>,
    by_month_day: Option<Vec<i32>>,
    exceptions: Vec<DateTime<Local>>,
}
```

#### Reminder Model
```rust
struct Reminder {
    id: Option<i64>,
    event_id: i64,
    minutes_before: i32,
    custom_time: Option<DateTime<Local>>,
    is_enabled: bool,
    last_triggered: Option<DateTime<Local>>,
}
```

## Data Flow

### Event Creation Flow
```
User Input → UI Event → App State Update → Event Service → Database → Confirmation
```

### Reminder Trigger Flow
```
Background Thread → Check Due Reminders → Reminder Service → Windows Notification → User Interaction
```

### Theme Switch Flow
```
User Selection → Theme Service → Load Theme Config → Apply to UI → Persist Setting
```

## Concurrency Model

### Main Thread
- UI rendering and interaction
- State management
- Non-blocking operations

### Background Thread
- Reminder checking (every 60 seconds)
- Database operations (when needed)
- File I/O operations

### Async Operations
- iCalendar import/export
- Large database queries
- System notifications

## Error Handling Strategy

### Error Types
```rust
#[derive(thiserror::Error, Debug)]
enum CalendarError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Invalid recurrence rule: {0}")]
    InvalidRecurrence(String),
    
    #[error("Event not found: {0}")]
    EventNotFound(i64),
    
    #[error("Notification error: {0}")]
    Notification(String),
}
```

### Error Handling Approach
- Use `Result<T, CalendarError>` for fallible operations
- Display user-friendly error messages in UI
- Log detailed errors for debugging
- Graceful degradation when possible

## Performance Considerations

### Database Optimization
- Index on event start/end times
- Prepared statements for common queries
- Connection pooling for concurrent access
- Lazy loading of event details

### UI Optimization
- Virtual scrolling for large event lists
- Efficient re-rendering with iced's reactive model
- Cached theme resources
- Debounced search input

### Memory Management
- Limit loaded events to visible date range
- Periodic cleanup of old notifications
- Efficient recurrence calculation caching

## Security Considerations

### Data Protection
- Local database with file system permissions
- Input validation and sanitization
- SQL injection prevention (parameterized queries)
- Secure handling of file paths

### Privacy
- No telemetry or data collection
- No network operations (local-only app)
- User control over all data

## Testing Strategy

### Unit Tests
- Model validation logic
- Recurrence calculation
- Date/time utilities
- Service layer functions

### Integration Tests
- Database operations
- Event CRUD workflows
- Reminder scheduling
- iCalendar import/export

### UI Tests
- Component rendering
- User interaction flows
- Theme switching
- State management

## Future Architecture Considerations

### Extensibility
- Plugin system for custom event types
- Theme marketplace
- Calendar protocol support (CalDAV)

### Scalability
- Support for multiple calendars
- Shared calendars (if sync added)
- Performance with 100k+ events

### Cross-Platform
- Abstract OS-specific code
- Platform-agnostic notification system
- Universal theme system

---

**Document Version**: 1.0  
**Last Updated**: November 6, 2025
