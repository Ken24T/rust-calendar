# Testing Guidelines

## Testing Philosophy

> "If it's not tested, it's broken." - Murphy's Law of Software Development

Every line of production code should be covered by tests. Testing is not optional—it's a core part of development.

## Testing Hierarchy

### 1. Unit Tests (Primary Focus)

**Location**: `#[cfg(test)] mod tests` blocks within source files

**Purpose**: Test individual functions, methods, and modules in isolation

**Characteristics**:
- Fast (< 1ms per test)
- No external dependencies (mock everything)
- Test one thing at a time
- Should constitute 70-80% of all tests

**Example**:
```rust
// In src/models/recurrence/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fortnightly_interval() {
        let freq = Frequency::Fortnightly;
        let (_, interval) = freq.to_rrule_params();
        assert_eq!(interval, 2, "Fortnightly should have interval of 2 weeks");
    }

    #[test]
    fn test_quarterly_interval() {
        let freq = Frequency::Quarterly;
        let (_, interval) = freq.to_rrule_params();
        assert_eq!(interval, 3, "Quarterly should have interval of 3 months");
    }
}
```

### 2. Integration Tests

**Location**: `tests/`

**Purpose**: Test interactions between multiple components

**Characteristics**:
- Slower (< 100ms per test)
- May use real database (in-memory or temporary)
- Test workflows and component interactions
- Should constitute 15-20% of all tests

**Example**:
```rust
// tests/integration/event_workflow_tests.rs
use rust_calendar::services::event::EventService;
use rust_calendar::models::event::Event;
use tempfile::TempDir;

#[test]
fn test_create_and_retrieve_event() -> Result<()> {
    // Setup
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let service = EventService::new(&db_path)?;
    
    // Create event
    let event = Event::builder()
        .title("Team Meeting")
        .start(Utc::now())
        .build()?;
    
    let created_event = service.create(event)?;
    let event_id = created_event.id.unwrap();
    
    // Retrieve event
    let retrieved = service.find_by_id(event_id)?;
    
    // Assert
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, "Team Meeting");
    
    Ok(())
}
```

### 3. Property-Based Tests

> **Note**: Property-based tests were used during early development and have
> since been removed. The patterns below remain as guidance if property tests
> are reintroduced in future.

**Location**: `tests/property/` (if reintroduced)

**Purpose**: Test invariants and properties with random inputs

**Characteristics**:
- Discovers edge cases automatically
- Uses `proptest` crate
- Slower but comprehensive
- Should constitute 5-10% of all tests

**Example**:
```rust
// tests/property/recurrence_properties.rs
use proptest::prelude::*;
use chrono::{NaiveDate, Duration};
use rust_calendar::models::recurrence::*;

proptest! {
    #[test]
    fn test_fortnightly_always_14_days_apart(
        year in 2020..2030i32,
        month in 1..=12u32,
        day in 1..=28u32,
    ) {
        let start_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let rule = RecurrenceRule::new(Frequency::Fortnightly);
        
        let occurrences = calculate_occurrences(&rule, start_date, 5);
        
        // Property: Each occurrence should be exactly 14 days after previous
        for window in occurrences.windows(2) {
            let diff = window[1].signed_duration_since(window[0]);
            prop_assert_eq!(diff, Duration::days(14));
        }
    }
    
    #[test]
    fn test_quarterly_always_same_day_of_month(
        year in 2020..2030i32,
        month in 1..=12u32,
        day in 1..=28u32,
    ) {
        let start_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let rule = RecurrenceRule::new(Frequency::Quarterly);
        
        let occurrences = calculate_occurrences(&rule, start_date, 4);
        
        // Property: All occurrences should have same day of month
        let expected_day = start_date.day();
        for occurrence in occurrences {
            prop_assert_eq!(occurrence.day(), expected_day);
        }
    }
}
```

### 4. Benchmark Tests
**Location**: `benches/`

**Purpose**: Measure and track performance

**Characteristics**:
- Uses `criterion` crate
- Run separately from unit tests
- Track performance over time
- Prevent performance regressions

**Example**:
```rust
// benches/recurrence_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_calendar::models::recurrence::*;
use chrono::NaiveDate;

fn bench_fortnightly_calculation(c: &mut Criterion) {
    let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let rule = RecurrenceRule::new(Frequency::Fortnightly);
    
    c.bench_function("calculate 1000 fortnightly occurrences", |b| {
        b.iter(|| {
            calculate_occurrences(
                black_box(&rule),
                black_box(start),
                black_box(1000)
            )
        })
    });
}

fn bench_quarterly_calculation(c: &mut Criterion) {
    let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let rule = RecurrenceRule::new(Frequency::Quarterly);
    
    c.bench_function("calculate 100 quarterly occurrences", |b| {
        b.iter(|| {
            calculate_occurrences(
                black_box(&rule),
                black_box(start),
                black_box(100)
            )
        })
    });
}

criterion_group!(benches, bench_fortnightly_calculation, bench_quarterly_calculation);
criterion_main!(benches);
```

## Test Organisation

### Inline Unit Tests

Unit tests live as `#[cfg(test)] mod tests` blocks within the source file they
test. This keeps tests close to the code they verify and simplifies navigation.

```rust
// src/models/recurrence/mod.rs
pub fn calculate_interval(freq: &Frequency) -> u32 {
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_interval_fortnightly() {
        assert_eq!(calculate_interval(&Frequency::Fortnightly), 2);
    }
}
```

### Integration Tests

Integration tests live in `tests/` and test interactions between multiple
components, typically using a real in-memory SQLite database.

### Descriptive Test Names

Use the pattern: `test_<what>_<when>_<expected>`

**Good Examples**:
```rust
#[test]
fn test_fortnightly_interval_returns_two()
#[test]
fn test_event_validation_fails_when_end_before_start()
#[test]
fn test_quarterly_occurrences_skip_february_29_on_non_leap_years()
```

**Bad Examples**:
```rust
#[test]
fn test1()
#[test]
fn test_frequency()
#[test]
fn it_works()
```

### Reusable Test Helpers

Create helper functions within `#[cfg(test)]` modules or in test utility files:

```rust
// tests/fixtures/mod.rs
pub mod events {
    use rust_calendar::models::event::Event;
    use chrono::Utc;
    
    pub fn sample_event() -> Event {
        Event::builder()
            .title("Sample Event")
            .start(Utc::now())
            .build()
            .unwrap()
    }
    
    pub fn fortnightly_event() -> Event {
        Event::builder()
            .title("Fortnightly Meeting")
            .start(Utc::now())
            .recurrence(RecurrenceRule::new(Frequency::Fortnightly))
            .build()
            .unwrap()
    }
    
    pub fn quarterly_event() -> Event {
        Event::builder()
            .title("Quarterly Review")
            .start(Utc::now())
            .recurrence(RecurrenceRule::new(Frequency::Quarterly))
            .build()
            .unwrap()
    }
}
```

## Testing Best Practices

### 1. AAA Pattern (Arrange, Act, Assert)

```rust
#[test]
fn test_create_fortnightly_event() {
    // Arrange
    let title = "Bi-weekly Standup";
    let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    
    // Act
    let event = Event::builder()
        .title(title)
        .start(start)
        .recurrence(RecurrenceRule::new(Frequency::Fortnightly))
        .build()
        .unwrap();
    
    // Assert
    assert_eq!(event.title, title);
    assert!(event.recurrence.is_some());
    assert_eq!(event.recurrence.unwrap().frequency, Frequency::Fortnightly);
}
```

### 2. Test Edge Cases

Always test boundary conditions:

```rust
#[test]
fn test_fortnightly_on_leap_year() {
    // Test Feb 29 on leap year
}

#[test]
fn test_quarterly_crosses_year_boundary() {
    // Start in December, ensure next occurrence is in March
}

#[test]
fn test_fortnightly_with_dst_transition() {
    // Test daylight saving time transitions
}

#[test]
fn test_quarterly_on_month_with_31_days() {
    // Jan 31 → April 30 (not May 1)
}
```

### 3. Test Error Conditions

```rust
#[test]
fn test_invalid_event_returns_error() {
    let result = Event::builder()
        .title("")  // Invalid: empty title
        .build();
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ValidationError::EmptyTitle));
}

#[test]
#[should_panic(expected = "End time must be after start time")]
fn test_event_with_end_before_start_panics() {
    Event::builder()
        .start(Utc::now())
        .end(Utc::now() - Duration::hours(1))
        .build()
        .unwrap();
}
```

### 4. Use Parameterized Tests

Use `test-case` crate for similar tests with different inputs:

```rust
use test_case::test_case;

#[test_case(Frequency::Daily, 1)]
#[test_case(Frequency::Weekly, 1)]
#[test_case(Frequency::Fortnightly, 2)]
#[test_case(Frequency::Monthly, 1)]
#[test_case(Frequency::Quarterly, 3)]
#[test_case(Frequency::Yearly, 1)]
fn test_frequency_interval(freq: Frequency, expected: u32) {
    let (_, interval) = freq.to_rrule_params();
    assert_eq!(interval, expected);
}
```

### 5. Mock External Dependencies

Use `mockall` for mocking:

```rust
#[cfg(test)]
mod tests {
    use mockall::predicate::*;
    use mockall::mock;
    
    mock! {
        pub EventRepository {}
        
        impl EventRepository for EventRepository {
            fn create(&self, event: &Event) -> Result<i64>;
            fn read(&self, id: i64) -> Result<Option<Event>>;
        }
    }
    
    #[test]
    fn test_event_service_creates_event() {
        let mut mock_repo = MockEventRepository::new();
        mock_repo
            .expect_create()
            .with(predicate::always())
            .times(1)
            .returning(|_| Ok(1));
        
        let service = EventService::new(mock_repo);
        let result = service.create(sample_event());
        
        assert!(result.is_ok());
    }
}
```

### 6. Use Temporary Files for Database Tests

```rust
use tempfile::TempDir;

#[test]
fn test_database_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    
    let db = Database::new(&db_path)?;
    // Test operations...
    
    // temp_dir automatically cleaned up when dropped
    Ok(())
}
```

## Test Coverage

All new non-trivial logic should include tests. Critical modules (recurrence
expansion, database operations, date arithmetic) should have thorough coverage.

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir ./coverage

# View coverage report
# Open coverage/index.html in browser
```

### Critical Modules

These modules should have thorough test coverage:
- `src/models/recurrence/` - All recurrence logic
- `src/services/database/` - All database operations
- `src/models/event/validator.rs` - Event validation
- `src/utils/date/` - Date calculations

## Test Execution

### Run All Tests
```bash
cargo test --all
```

### Run Unit Tests Only
```bash
cargo test --lib
```

### Run Integration Tests Only
```bash
cargo test --test '*'
```

### Run Specific Test
```bash
cargo test test_fortnightly_interval
```

### Run Tests with Output
```bash
cargo test -- --nocapture
```

### Run Tests in Parallel
```bash
cargo test -- --test-threads=4
```

### Run Tests Sequentially (for database tests)
```bash
cargo test -- --test-threads=1
```

Or use `serial_test` crate:
```rust
use serial_test::serial;

#[test]
#[serial]
fn test_database_operation_1() {
    // Runs sequentially
}

#[test]
#[serial]
fn test_database_operation_2() {
    // Runs sequentially
}
```

## Continuous Integration

> **Note**: CI is not yet configured. The workflow below is a template for
> future setup.

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cargo test --all --verbose
      - name: Check coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml
      - name: Upload coverage
        uses: codecov/codecov-action@v1
```

## Test Documentation

Every test should be self-documenting:

```rust
/// Tests that fortnightly recurrence correctly calculates occurrences
/// every 14 days, handling month boundaries correctly.
///
/// This test verifies:
/// - First occurrence matches start date
/// - Subsequent occurrences are exactly 14 days apart
/// - Month boundaries are handled correctly
/// - February is handled correctly (both leap and non-leap years)
#[test]
fn test_fortnightly_recurrence_handles_month_boundaries() {
    // Test implementation
}
```

## Common Testing Pitfalls to Avoid

1. **Testing Implementation Details**: Test behavior, not internal structure
2. **Interdependent Tests**: Each test should be independent
3. **Slow Tests**: Keep unit tests fast (< 1ms)
4. **Flaky Tests**: Tests should be deterministic (avoid `Utc::now()` directly)
5. **Unclear Failures**: Use descriptive assertion messages
6. **Missing Edge Cases**: Always test boundaries
7. **No Negative Tests**: Test error conditions
8. **Mocking Too Much**: Don't mock what you own

## Test Quality Checklist

For each test, verify:

- [ ] Test name clearly describes what is being tested
- [ ] Test follows AAA pattern (Arrange, Act, Assert)
- [ ] Test is independent (no shared state)
- [ ] Test is deterministic (no random failures)
- [ ] Test is fast (< 1ms for unit tests)
- [ ] Assertions have descriptive messages
- [ ] Error cases are tested
- [ ] Edge cases are covered
- [ ] Test uses fixtures/helpers for setup
- [ ] Test cleans up resources (if any)

---

**Remember**: Good tests are the safety net that allows confident refactoring and rapid development. Invest in your tests!
