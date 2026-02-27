# Modularity Guidelines

## Core Principles

### 1. Single Responsibility Principle
Each module, file, and function should have one clear, well-defined purpose.

**Good Example:**
```rust
// src/models/recurrence/frequency.rs
// ONLY handles Frequency enum and conversions
```

**Bad Example:**
```rust
// src/models/recurrence.rs
// Contains Frequency, RecurrenceRule, Calculator, Exceptions all in one file
```

### 2. File Size Limits

**Strict Limits:**
- Maximum 300 lines per file (including tests if embedded)
- Target 150-200 lines for most files
- If a file exceeds 250 lines, consider splitting it

**When to Split:**
- Multiple structs/enums in one file → Split into separate files
- Large impl blocks → Split into trait implementations or submodules
- Many related functions → Group into logical submodules

### 3. Module Organization

#### Pattern: Feature-Based Modules

```
src/models/event/
├── mod.rs          # Public exports only (~20 lines)
├── event.rs        # Event struct definition (~100 lines)
├── builder.rs      # Builder pattern implementation (~150 lines)
├── validator.rs    # Validation logic (~120 lines)
└── serialization.rs # Serde implementations (~80 lines)
```

**mod.rs Template:**
```rust
// src/models/event/mod.rs
mod event;
mod builder;
mod validator;
mod serialization;

pub use event::Event;
pub use builder::EventBuilder;
pub use validator::EventValidator;
// Note: serialization is implementation detail, not re-exported
```

### 4. Avoiding God Objects

**Bad Pattern - Monolithic Service:**
```rust
// ❌ DON'T DO THIS - src/services/event_service.rs (1000+ lines)
impl EventService {
    fn create_event() { /* 50 lines */ }
    fn update_event() { /* 50 lines */ }
    fn delete_event() { /* 40 lines */ }
    fn find_by_id() { /* 30 lines */ }
    fn find_by_date_range() { /* 60 lines */ }
    fn find_conflicting() { /* 80 lines */ }
    fn calculate_recurrences() { /* 200 lines */ }
    fn apply_exceptions() { /* 100 lines */ }
    // ... more methods
}
```

**Good Pattern - Composed Services:**
```rust
// ✅ DO THIS - Split into focused modules

// src/services/event/crud.rs (~120 lines)
pub struct EventCrud { /* ... */ }
impl EventCrud {
    pub fn create(&self, event: Event) -> Result<Event> { /* ... */ }
    pub fn update(&self, event: Event) -> Result<Event> { /* ... */ }
    pub fn delete(&self, id: i64) -> Result<()> { /* ... */ }
}

// src/services/event/query.rs (~150 lines)
pub struct EventQuery { /* ... */ }
impl EventQuery {
    pub fn find_by_id(&self, id: i64) -> Result<Option<Event>> { /* ... */ }
    pub fn find_by_date_range(&self, start: Date, end: Date) -> Result<Vec<Event>> { /* ... */ }
    pub fn find_conflicting(&self, event: &Event) -> Result<Vec<Event>> { /* ... */ }
}

// src/services/event/recurrence_handler.rs (~180 lines)
pub struct RecurrenceHandler { /* ... */ }
impl RecurrenceHandler {
    pub fn calculate_occurrences(&self, event: &Event, range: DateRange) -> Vec<DateTime> { /* ... */ }
    pub fn apply_exceptions(&self, occurrences: Vec<DateTime>, exceptions: &[DateTime]) -> Vec<DateTime> { /* ... */ }
}

// src/services/event/mod.rs (~50 lines)
mod crud;
mod query;
mod recurrence_handler;

pub use crud::EventCrud;
pub use query::EventQuery;
pub use recurrence_handler::RecurrenceHandler;

// Facade pattern for convenience
pub struct EventService {
    crud: EventCrud,
    query: EventQuery,
    recurrence: RecurrenceHandler,
}

impl EventService {
    pub fn new(db: Database) -> Self { /* ... */ }
    
    // Delegate to specialized services
    pub fn create(&self, event: Event) -> Result<Event> {
        self.crud.create(event)
    }
    // ... other delegations
}
```

### 5. Function Complexity

**Guidelines:**
- Maximum 50 lines per function
- Target 10-20 lines for most functions
- Maximum 4 levels of nesting
- If a function does more than one thing, split it

**Example - Refactoring Complex Function:**

**Before (Complex):**
```rust
// ❌ Too complex (~80 lines, multiple responsibilities)
fn process_recurring_event(event: &Event, range: DateRange) -> Result<Vec<DateTime>> {
    // Validate input
    if event.start > event.end {
        return Err(Error::InvalidEvent);
    }
    
    // Parse recurrence rule
    let rule = match &event.recurrence {
        Some(r) => r,
        None => return Ok(vec![event.start]),
    };
    
    // Calculate occurrences
    let mut occurrences = Vec::new();
    let mut current = event.start;
    while current <= range.end {
        if current >= range.start {
            occurrences.push(current);
        }
        // Complex calculation logic...
        // 50+ more lines
    }
    
    // Apply exceptions
    occurrences.retain(|dt| !rule.exceptions.contains(dt));
    
    Ok(occurrences)
}
```

**After (Modular):**
```rust
// ✅ Split into focused functions (~10-15 lines each)

fn process_recurring_event(event: &Event, range: DateRange) -> Result<Vec<DateTime>> {
    validate_event(event)?;
    let rule = get_recurrence_rule(event)?;
    let occurrences = calculate_occurrences(event, &rule, range)?;
    let filtered = apply_exceptions(occurrences, &rule.exceptions);
    Ok(filtered)
}

fn validate_event(event: &Event) -> Result<()> {
    if event.start > event.end {
        return Err(Error::InvalidEvent);
    }
    Ok(())
}

fn get_recurrence_rule(event: &Event) -> Result<RecurrenceRule> {
    event.recurrence
        .clone()
        .ok_or(Error::NoRecurrenceRule)
}

fn calculate_occurrences(
    event: &Event,
    rule: &RecurrenceRule,
    range: DateRange
) -> Result<Vec<DateTime>> {
    // Focused calculation logic (~30 lines)
}

fn apply_exceptions(
    occurrences: Vec<DateTime>,
    exceptions: &[DateTime]
) -> Vec<DateTime> {
    occurrences.into_iter()
        .filter(|dt| !exceptions.contains(dt))
        .collect()
}
```

### 6. Trait-Based Composition

Use traits to keep implementations focused:

```rust
// src/services/database/traits.rs
pub trait EventRepository {
    fn create(&self, event: &Event) -> Result<i64>;
    fn read(&self, id: i64) -> Result<Option<Event>>;
    fn update(&self, event: &Event) -> Result<()>;
    fn delete(&self, id: i64) -> Result<()>;
}

pub trait EventQuery {
    fn find_by_date_range(&self, start: Date, end: Date) -> Result<Vec<Event>>;
    fn find_by_category(&self, category: &str) -> Result<Vec<Event>>;
}

// src/services/database/events_repo.rs (~150 lines)
pub struct SqliteEventRepository {
    conn: Connection,
}

impl EventRepository for SqliteEventRepository {
    // Only CRUD implementation
}

impl EventQuery for SqliteEventRepository {
    // Only query implementation
}
```

### 7. Configuration Module Structure

Keep configuration separate and modular:

```
src/config/
├── mod.rs              # Config exports
├── app_config.rs       # Application settings (~80 lines)
├── database_config.rs  # Database settings (~60 lines)
├── ui_config.rs        # UI settings (~70 lines)
└── defaults.rs         # Default values (~50 lines)
```

### 8. Error Handling Modularity

Separate error types by domain:

```rust
// src/error/mod.rs
mod event_error;
mod database_error;
mod recurrence_error;
mod ui_error;

pub use event_error::EventError;
pub use database_error::DatabaseError;
pub use recurrence_error::RecurrenceError;
pub use ui_error::UiError;

// Main application error that wraps all others
#[derive(thiserror::Error, Debug)]
pub enum CalendarError {
    #[error(transparent)]
    Event(#[from] EventError),
    
    #[error(transparent)]
    Database(#[from] DatabaseError),
    
    #[error(transparent)]
    Recurrence(#[from] RecurrenceError),
    
    #[error(transparent)]
    Ui(#[from] UiError),
}
```

### 9. Testing Modularity

Each module should have its own test module:

```rust
// src/models/event/validator.rs
pub struct EventValidator;

impl EventValidator {
    pub fn validate(event: &Event) -> Result<(), ValidationError> {
        // Implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_event() {
        // Test code
    }
    
    #[test]
    fn test_invalid_dates() {
        // Test code
    }
}
```

### 10. Documentation Requirements

Every public module, struct, function, and trait must have documentation:

```rust
/// Represents the frequency of a recurring event.
///
/// Provides convenient wrappers for common recurrence patterns like
/// fortnightly (bi-weekly) and quarterly (every 3 months).
///
/// # Examples
///
/// ```
/// use rust_calendar::models::recurrence::Frequency;
///
/// let freq = Frequency::Fortnightly;
/// let (rrule_freq, interval) = freq.to_rrule_params();
/// assert_eq!(interval, 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    /// Event occurs every day
    Daily,
    /// Event occurs every week
    Weekly,
    /// Event occurs every two weeks (bi-weekly)
    Fortnightly,
    /// Event occurs every month
    Monthly,
    /// Event occurs every three months (quarterly)
    Quarterly,
    /// Event occurs every year
    Yearly,
    /// Custom recurrence pattern
    Custom,
}
```

## Code Review Checklist

Before committing any code, verify:

- [ ] No file exceeds 300 lines
- [ ] Each file has a single, clear purpose
- [ ] No function exceeds 50 lines
- [ ] Maximum nesting depth is 4 levels
- [ ] All public items have documentation
- [ ] Each module has corresponding tests
- [ ] No "God objects" or monolithic services
- [ ] Trait boundaries are well-defined
- [ ] Error types are appropriately scoped
- [ ] Module exports are minimal and intentional

## Refactoring Triggers

Refactor immediately when:

1. A file reaches 250+ lines
2. A function reaches 40+ lines
3. A struct has more than 10 methods
4. Cyclomatic complexity exceeds 10
5. You're scrolling to find code
6. You need a comment saying "This does X and Y and Z"
7. You're adding the 3rd similar function (extract pattern)

## Tools to Enforce Modularity

```toml
# .cargo/config.toml
[build]
rustflags = ["-D", "warnings"]

# Use clippy lints
[lints.clippy]
too_many_lines = "warn"          # Warn on files >200 lines
cognitive_complexity = "warn"     # Warn on complex functions
module_inception = "warn"         # Warn on deep nesting
```

---

**Remember**: Small, focused modules are easier to test, understand, maintain, and reuse. When in doubt, split it out!
