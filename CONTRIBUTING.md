# Contributing to Rust Calendar

Thank you for your interest in contributing to Rust Calendar! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow:

- **Be respectful**: Treat everyone with respect and consideration
- **Be constructive**: Provide helpful feedback and suggestions
- **Be collaborative**: Work together to improve the project
- **Be patient**: Remember that everyone has different skill levels and backgrounds

## Getting Started

### Prerequisites

- Rust 1.75 or later ([install from rustup.rs](https://rustup.rs/))
- Visual Studio Build Tools (Windows C++ development tools)
- Git for version control
- A code editor (VS Code with rust-analyzer recommended)

### Development Setup

1. **Fork and clone the repository**:
   ```powershell
   git clone https://github.com/yourusername/rust-calendar.git
   cd rust-calendar
   ```

2. **Build the project**:
   ```powershell
   cargo build
   ```

3. **Run tests**:
   ```powershell
   cargo test
   ```

4. **Run the application**:
   ```powershell
   cargo run
   ```

## Development Workflow

### Branching Strategy

We use a feature branch workflow:

- `main` - Stable, production-ready code
- `feature/*` - New features (e.g., `feature/event-crud`)
- `fix/*` - Bug fixes (e.g., `fix/reminder-crash`)
- `refactor/*` - Code refactoring (e.g., `refactor/event-service`)
- `docs/*` - Documentation updates (e.g., `docs/api-guide`)

### Making Changes

1. **Create a feature branch**:
   ```powershell
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the project guidelines (see below)

3. **Write tests** for your changes (see TESTING.md)

4. **Run the test suite**:
   ```powershell
   cargo test
   ```

5. **Check code formatting**:
   ```powershell
   cargo fmt --check
   ```

6. **Run clippy for lints**:
   ```powershell
   cargo clippy -- -D warnings
   ```

7. **Commit your changes** with a descriptive message:
   ```powershell
   git commit -m "feat: add event recurrence validation"
   ```

8. **Push to your fork**:
   ```powershell
   git push origin feature/your-feature-name
   ```

9. **Open a Pull Request** with a clear description of your changes

## Code Guidelines

### Modularity

**File Size**: Keep files under 300 lines. See [MODULARITY.md](docs/MODULARITY.md) for details.

**Single Responsibility**: Each module should have one clear purpose.

**Example**:
```rust
// Good: Focused module
// src/services/event/create.rs (150 lines)

// Bad: Monolithic module
// src/services/event.rs (2000 lines)
```

### Code Style

We follow Rust standard style guidelines:

- Use `cargo fmt` for automatic formatting
- Run `cargo clippy` and fix all warnings
- Use descriptive variable names
- Add documentation comments for public APIs
- Keep functions under 50 lines
- Limit nesting to 4 levels

**Example**:
```rust
/// Creates a new event with the given parameters.
///
/// # Arguments
///
/// * `title` - The event title
/// * `start` - The event start time
/// * `end` - The event end time
///
/// # Returns
///
/// Returns `Ok(Event)` on success, or an error if validation fails.
///
/// # Examples
///
/// ```
/// let event = create_event("Meeting", start_time, end_time)?;
/// ```
pub fn create_event(title: &str, start: DateTime<Local>, end: DateTime<Local>) -> Result<Event> {
    validate_times(start, end)?;
    
    Ok(Event {
        title: title.to_string(),
        start,
        end,
        ..Default::default()
    })
}
```

### Testing

**Coverage Requirement**: >90% overall, 100% for critical modules.

See [TESTING.md](docs/TESTING.md) for comprehensive testing guidelines.

**Test Structure**:
```
tests/
├── unit/          # Mirror src/ structure 1:1
├── integration/   # End-to-end tests
├── property/      # Property-based tests
└── fixtures/      # Shared test data
```

**Example Test**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_event_success() {
        // Arrange
        let title = "Team Meeting";
        let start = Local::now();
        let end = start + Duration::hours(1);
        
        // Act
        let result = create_event(title, start, end);
        
        // Assert
        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.title, title);
    }
    
    #[test]
    fn test_create_event_invalid_times() {
        // Arrange
        let start = Local::now();
        let end = start - Duration::hours(1); // End before start
        
        // Act
        let result = create_event("Test", start, end);
        
        // Assert
        assert!(result.is_err());
    }
}
```

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

**Format**: `<type>(<scope>): <description>`

**Types**:
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `refactor` - Code refactoring
- `test` - Adding or updating tests
- `chore` - Build process or tooling changes
- `perf` - Performance improvements

**Examples**:
```
feat(event): add fortnightly recurrence pattern
fix(reminder): resolve notification crash on Windows 11
docs(api): update event creation examples
test(recurrence): add property tests for quarterly patterns
refactor(database): split repository into focused modules
```

### Documentation

- Add doc comments (`///`) for all public items
- Include examples in doc comments when helpful
- Update relevant .md files when changing features
- Keep README.md up to date

## Pull Request Process

### PR Title

Use conventional commit format: `feat: add event drag-and-drop`

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests passing
- [ ] Code coverage maintained/improved

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] No new warnings from clippy
- [ ] cargo fmt applied
- [ ] CHANGELOG.md updated
```

### Review Process

1. **Automated checks** must pass:
   - Build succeeds
   - All tests pass
   - No clippy warnings
   - Code formatted with rustfmt

2. **Code review**:
   - At least one approval required
   - Address all reviewer comments
   - Maintain respectful discussion

3. **Merge**:
   - Squash and merge preferred
   - Keep main branch clean

## Reporting Issues

### Bug Reports

**Title**: Clear, concise description

**Template**:
```markdown
## Description
What happened?

## Steps to Reproduce
1. Step one
2. Step two
3. ...

## Expected Behavior
What should happen?

## Actual Behavior
What actually happened?

## Environment
- OS: Windows 10/11
- Rust version: 1.75
- App version: 0.1.0

## Additional Context
Screenshots, logs, etc.
```

### Feature Requests

**Title**: `feat: [Feature Name]`

**Template**:
```markdown
## Feature Description
Clear description of the feature

## Use Case
Why is this feature needed?

## Proposed Solution
How should it work?

## Alternatives Considered
Other approaches you've thought about

## Additional Context
Mockups, examples, etc.
```

## Development Tips

### Useful Commands

```powershell
# Full build with optimizations
cargo build --release

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Generate documentation
cargo doc --open

# Check for outdated dependencies
cargo outdated

# Run benchmarks
cargo bench

# Code coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Debugging

- Use `dbg!()` macro for quick debugging
- Use `env_logger` for runtime logging
- Use VS Code debugger with CodeLLDB extension
- Check TESTING.md for test-specific debugging tips

## Project Structure

```
rust-calendar/
├── src/              # Source code (max 300 lines per file)
│   ├── ui/           # User interface components
│   ├── models/       # Data models
│   ├── services/     # Business logic
│   └── utils/        # Utility functions
├── tests/            # Test suite
├── benches/          # Performance benchmarks
├── docs/             # Documentation
├── assets/           # Themes, icons, resources
└── Cargo.toml        # Project configuration
```

## Getting Help

- **Documentation**: Check docs/ directory
- **Issues**: Search existing issues first
- **Discussions**: Use GitHub Discussions for questions
- **Contact**: Open an issue for any questions

## Recognition

Contributors will be recognized in:
- CHANGELOG.md (for significant contributions)
- README.md (major features)
- Git commit history (all contributions)

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

---

**Thank you for contributing to Rust Calendar!** 🎉
