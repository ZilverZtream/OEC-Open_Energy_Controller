# Contributing to Open Energy Controller

Thank you for your interest in contributing! This document provides guidelines for contributing code, documentation, and other improvements.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Documentation Requirements](#documentation-requirements)
- [Pull Request Process](#pull-request-process)

---

## Code of Conduct

This project adheres to a code of conduct that ensures a welcoming environment for all contributors. By participating, you agree to uphold professional standards of conduct.

---

## Getting Started

### Prerequisites

- Rust 1.75+ installed via rustup
- PostgreSQL 16
- Docker (optional, for containerized development)

### Initial Setup

```bash
git clone https://github.com/yourusername/open-energy-controller.git
cd open-energy-controller
cp .env.example .env
docker-compose up -d postgres
sqlx migrate run
cargo test
```

---

## Development Workflow

### Branch Strategy

- `main` - Production-ready code
- `develop` - Integration branch
- `feature/*` - Feature branches
- `fix/*` - Bug fix branches

### Making Changes

1. Create a feature branch from `develop`:
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/your-feature-name
   ```

2. Make your changes following the coding standards below

3. Run the pre-commit checklist:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```

4. Update `MASSIVE_TODO_LIST.md` if completing TODO items

5. Commit with a descriptive message:
   ```bash
   git commit -m "feat: add simulated battery implementation"
   ```

6. Push and create a pull request

---

## Coding Standards

### General Principles

1. **Correctness First** - Code must be correct before it's optimized
2. **Safety by Design** - Leverage Rust's type system
3. **No Unsafe Code** - Avoid `unsafe` unless absolutely necessary
4. **No Unwraps in Production** - Use `?` or proper error handling
5. **Integration Required** - New code must be wired into existing systems
6. **Clean Code** - No dead code, commented cruft, or debug prints

### Power Flow Architecture

**Critical:** This system uses holistic power flow orchestration. Never control devices independently:

```rust
// ❌ WRONG - Violates fuse limits
battery.set_power(3000.0).await?;
ev_charger.set_power(11000.0).await?;

// ✅ CORRECT - Coordinated control
let snapshot = power_flow_model.compute_flows(inputs, constraints, objectives).await?;
battery.set_power(snapshot.battery_power_kw).await?;
ev_charger.set_power(snapshot.ev_power_kw).await?;
```

Read [POWER_FLOW_ARCHITECTURE.md](POWER_FLOW_ARCHITECTURE.md) for details.

### File Organization

- Keep files <500 lines - split into modules if larger
- Place domain logic in `src/domain/`
- Place hardware implementations in `src/hardware/`
- Place API handlers in `src/api/handlers/`
- Tests go in `#[cfg(test)] mod tests {}` or `tests/`

### Module Exports

Always export new modules:

```rust
// src/hardware/simulated/mod.rs
pub mod battery;
pub use battery::SimulatedBattery;
```

### Error Handling

Use typed errors with `thiserror`:

```rust
#[derive(Error, Debug)]
pub enum BatteryError {
    #[error("Communication failed: {0}")]
    CommunicationError(String),
    
    #[error("Invalid SoC: {0}")]
    InvalidSoC(f64),
    
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
}
```

### Type Safety

Use domain types, not primitives:

```rust
// ❌ WRONG
fn set_power(watts: f64) { }

// ✅ CORRECT
fn set_power(power: Power) { }

pub struct Power(f64); // Newtype with validation
```

### Async Code

- Never block the async runtime
- Use `tokio::time::sleep`, not `std::thread::sleep`
- Use `tokio::fs`, not `std::fs`
- Use `Arc<RwLock<T>>` for shared mutable state

---

## Testing Requirements

### Coverage

- Overall: >80%
- Domain logic: >90%
- New code: 100%

### Test Types

1. **Unit Tests** - Test individual functions
   ```rust
   #[test]
   fn test_power_balance() {
       let snapshot = PowerSnapshot { /* ... */ };
       assert!(snapshot.verify_power_balance());
   }
   ```

2. **Integration Tests** - Test module interactions
   ```rust
   #[tokio::test]
   async fn test_battery_controller() {
       let battery = SimulatedBattery::new(10.0);
       let controller = BatteryController::new(battery);
       // Test full workflow
   }
   ```

3. **Property Tests** - Test invariants
   ```rust
   #[proptest]
   fn test_fuse_never_exceeded(power: f64) {
       // Property: fuse limit never violated
   }
   ```

### Running Tests

```bash
# All tests
cargo test

# With coverage
cargo tarpaulin --out Html

# Specific test
cargo test test_power_balance

# With logging
RUST_LOG=debug cargo test
```

---

## Documentation Requirements

### Public APIs

Every public item needs documentation:

```rust
/// Represents a battery storage system.
///
/// # Fields
/// - `capacity_kwh`: Total storage capacity in kilowatt-hours
/// - `soc_percent`: Current state of charge (0-100%)
///
/// # Example
/// ```
/// let battery = SimulatedBattery::new(10.0);
/// let state = battery.read_state().await?;
/// assert!(state.soc_percent >= 0.0);
/// ```
pub struct Battery {
    capacity_kwh: f64,
    soc_percent: f64,
}
```

### Code Comments

Use comments for:
- Non-obvious algorithms
- Performance considerations
- Safety invariants
- Limitations

Don't comment:
- Obvious code
- What (code shows this)

### Architecture Decisions

Document significant decisions in `docs/ADR/` using this template:

```markdown
# ADR-001: Use Dynamic Programming for Optimization

## Context
Need to optimize battery charging schedule...

## Decision
Use dynamic programming algorithm...

## Consequences
+ Fast computation (<5ms)
- Limited to discrete states
```

---

## Pull Request Process

### Before Submitting

- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation updated
- [ ] TODO list updated (if applicable)
- [ ] No dead code or debug prints

### PR Description

Include:
- **What**: Brief description of changes
- **Why**: Motivation for the changes
- **How**: Technical approach
- **Testing**: How you tested it

Example:
```markdown
## What
Implement power flow orchestration for EV charging

## Why
Need to coordinate EV charging with fuse limits and solar production

## How
- Added PowerFlowModel with constraint checking
- Implemented urgency-based EV charging
- Added fuse protection logic

## Testing
- Unit tests for power balance
- Integration tests for fuse scenarios
- Property tests for invariants
```

### Review Process

1. Automated checks run (CI)
2. Maintainer reviews code
3. Address feedback
4. Approve and merge

### Commit Message Format

Follow Conventional Commits:

```
<type>(<scope>): <description>

<body>

<footer>
```

Types:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation only
- `test:` - Test additions/changes
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Build/tooling changes

Example:
```
feat(power-flow): implement EV charging coordination

- Added urgency calculation based on departure time
- Integrated with fuse limit protection
- Added comprehensive test scenarios

Updated MASSIVE_TODO_LIST.md Phase 17
```

---

## Code Review Guidelines

### What We Look For

**Architecture:**
- Fits overall design
- Right module location
- Appropriate abstractions

**Quality:**
- Readable and maintainable
- Descriptive naming
- Well-documented
- No commented code

**Testing:**
- Sufficient coverage
- Tests pass
- Edge cases covered

**Safety:**
- Proper error handling
- No unwraps
- Errors logged appropriately

---

## Common Patterns

### Creating New Hardware Implementations

1. Define trait in `src/domain/`
2. Implement in `src/hardware/simulated/` for testing
3. Implement in `src/hardware/modbus/` for real hardware
4. Export from parent module
5. Add to factory pattern
6. Write tests

### Adding API Endpoints

1. Create handler in `src/api/handlers/`
2. Add route in `src/api/routes.rs`
3. Add OpenAPI documentation
4. Write integration tests
5. Test with curl/Postman

### Adding Database Tables

1. Create migration file
2. Create model struct
3. Create repository
4. Add to database tests
5. Run migration

---

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Project Architecture](POWER_FLOW_ARCHITECTURE.md)
- [Development Guide](DEVELOPMENT.md)

---

## Getting Help

- Check existing documentation
- Search closed issues/PRs
- Ask in discussions
- Open an issue with details

---

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
