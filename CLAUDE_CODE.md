# 🤖 Claude Code - Specific Instructions

**This file is specifically for Claude Code / Claude with computer use capabilities.**

If you're Claude Code working on this project, read this file AFTER reading `AGENTS.md` (which contains general rules for all AI agents).

---

## 🎯 What Makes You Special

You have **computer use capabilities** that other AI agents don't:
- `bash_tool` - Execute shell commands
- `create_file` - Create files with content
- `str_replace` - Edit existing files
- `view` - Read files and directories

**Use these powers wisely!**

---

## 🚀 Optimal Workflow for Claude Code

### Step 1: Understand the Task

```bash
# 1. View the TODO list to find your task
view /path/to/MASSIVE_TODO_LIST.md

# 2. Search for related code
bash_tool "rg 'Battery' src/ --type rust"

# 3. View the architecture
view docs/ARCHITECTURE.md

# 4. Check existing implementations
bash_tool "fd battery.rs src/"
view src/domain/battery/
```

### Step 2: Plan Your Implementation

**Before writing ANY code:**
1. Identify what files need to be created
2. Identify what files need to be modified
3. Identify what files need to import the new code
4. Plan your tests
5. Plan integration points

**Example plan:**
```
Task: Implement SimulatedBattery

Files to CREATE:
- src/hardware/simulated/battery.rs

Files to MODIFY:
- src/hardware/simulated/mod.rs (add pub mod battery)
- src/hardware/factory.rs (use SimulatedBattery)
- Cargo.toml (ensure tokio dependency)

Files to CREATE for tests:
- tests/integration/simulated_battery_test.rs

Integration points:
- BatteryController should use it
- API should be able to query it
```

### Step 3: Implement Incrementally

**Use create_file for new files:**
```python
create_file(
    path="src/hardware/simulated/battery.rs",
    description="Create SimulatedBattery implementation",
    file_text="""
use crate::domain::battery::{Battery, BatteryState};
use async_trait::async_trait;

pub struct SimulatedBattery {
    // ...
}

#[async_trait]
impl Battery for SimulatedBattery {
    // ...
}

#[cfg(test)]
mod tests {
    // ...
}
"""
)
```

**Use str_replace for modifications:**
```python
str_replace(
    path="src/hardware/simulated/mod.rs",
    old_str="// Add new modules here",
    new_str="""// Add new modules here
pub mod battery;
pub use battery::SimulatedBattery;""",
    description="Export SimulatedBattery module"
)
```

### Step 4: Test Immediately

```bash
# Run cargo fmt
bash_tool "cargo fmt"

# Run clippy
bash_tool "cargo clippy --all-targets --all-features -- -D warnings"

# Run tests
bash_tool "cargo test"

# If tests fail, read the output and fix!
```

### Step 5: Integrate

**Don't just create files - wire them up!**

```bash
# Check where the new code should be used
bash_tool "rg 'dyn Battery' src/"

# View the factory file
view src/hardware/factory.rs

# Update the factory
str_replace(...)

# Test the integration
bash_tool "cargo test --test integration_test"
```

### Step 6: Update TODO List

```python
# View the TODO list
view MASSIVE_TODO_LIST.md

# Find the line to update (e.g., line 450)
str_replace(
    path="MASSIVE_TODO_LIST.md",
    old_str="- [ ] Create `SimulatedBattery` struct",
    new_str="- [x] Create `SimulatedBattery` struct",
    description="Mark SimulatedBattery task as complete"
)
```

### Step 7: Commit

```bash
bash_tool """
git add .
git commit -m "feat: implement SimulatedBattery

- Created src/hardware/simulated/battery.rs
- Implemented Battery trait with realistic simulation
- Added unit tests for state transitions
- Integrated with HardwareFactory
- Updated MASSIVE_TODO_LIST.md Phase 4 items 1-8

Tests:
- cargo test passes
- cargo clippy passes
"""
```

---

## 🛠️ Best Practices for Claude Code

### Use bash_tool Efficiently

**✅ GOOD - Combine commands:**
```bash
bash_tool "cargo fmt && cargo clippy -- -D warnings && cargo test"
```

**❌ BAD - Multiple calls:**
```bash
bash_tool "cargo fmt"
bash_tool "cargo clippy"
bash_tool "cargo test"
```

### Use view Before Modifying

**Always view files before editing them:**
```python
# 1. View the file
view src/api/routes.rs

# 2. Find the exact string to replace
# 3. Use str_replace with precise old_str
str_replace(
    path="src/api/routes.rs",
    old_str="    // Add new routes here",
    new_str="""    // Add new routes here
    .route("/battery", get(handlers::battery::get_state))""",
    description="Add battery route"
)
```

**Why?** Because `str_replace` requires an EXACT match. If you guess wrong, it fails.

### Handle Large Files

**For files >500 lines, use view with ranges:**
```python
# View specific section
view src/main.rs [1, 50]  # First 50 lines
view src/main.rs [100, 150]  # Lines 100-150
```

### Create Complete Files

**Don't create stub files! Create complete implementations:**

**❌ BAD:**
```rust
// src/domain/battery.rs
pub struct Battery {
    // TODO: implement
}
```

**✅ GOOD:**
```rust
// src/domain/battery.rs
use async_trait::async_trait;

/// Represents a battery system.
pub struct Battery {
    capacity_kwh: f64,
    current_soc: f64,
}

impl Battery {
    /// Creates a new battery with the given capacity.
    pub fn new(capacity_kwh: f64) -> Self {
        Self {
            capacity_kwh,
            current_soc: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_battery() {
        let battery = Battery::new(10.0);
        assert_eq!(battery.capacity_kwh, 10.0);
    }
}
```

---

## 🔍 Debugging with Claude Code

### When Tests Fail

```bash
# 1. Run tests with output
bash_tool "cargo test -- --nocapture"

# 2. View the failing test
view tests/integration/battery_test.rs

# 3. View the implementation
view src/hardware/simulated/battery.rs

# 4. Fix the issue
str_replace(...)

# 5. Re-run tests
bash_tool "cargo test"
```

### When Clippy Complains

```bash
# 1. Run clippy with explanations
bash_tool "cargo clippy --all-targets --all-features -- -D warnings 2>&1"

# 2. Read the warning carefully
# 3. View the offending file
# 4. Fix the issue
# 5. Re-run clippy
```

### When Compilation Fails

```bash
# 1. Run cargo check for faster feedback
bash_tool "cargo check"

# 2. Read the error message
# 3. View the file mentioned in the error
# 4. Fix the issue
# 5. Re-compile
```

---

## 📝 Template: Implementing a New Feature

Here's a complete workflow for implementing a new feature:

```python
# ============================================
# TASK: Implement GET /api/v1/battery/state
# ============================================

# Step 1: View existing similar handlers
view src/api/handlers/

# Step 2: Create the handler file
create_file(
    path="src/api/handlers/battery.rs",
    description="Create battery API handlers",
    file_text="""
use axum::{extract::State, Json};
use crate::api::{AppState, ApiError};
use crate::domain::battery::BatteryState;

/// Get current battery state
#[utoipa::path(
    get,
    path = "/api/v1/battery/state",
    responses(
        (status = 200, description = "Battery state", body = BatteryState)
    )
)]
pub async fn get_battery_state(
    State(state): State<AppState>,
) -> Result<Json<BatteryState>, ApiError> {
    let battery_state = state.battery.read_state().await?;
    Ok(Json(battery_state))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_get_battery_state() {
        // TODO: Add test
    }
}
"""
)

# Step 3: Export the module
view src/api/handlers/mod.rs
str_replace(
    path="src/api/handlers/mod.rs",
    old_str="// Add new handlers here",
    new_str="""// Add new handlers here
pub mod battery;""",
    description="Export battery handlers module"
)

# Step 4: Add the route
view src/api/routes.rs
str_replace(
    path="src/api/routes.rs",
    old_str="    // Add new routes here",
    new_str="""    // Add new routes here
    .route("/api/v1/battery/state", get(handlers::battery::get_battery_state))""",
    description="Add battery state route"
)

# Step 5: Test
bash_tool "cargo test"

# Step 6: Manual test
bash_tool "curl http://localhost:8080/api/v1/battery/state"

# Step 7: Update TODO list
view MASSIVE_TODO_LIST.md [800, 850]  # Find the task
str_replace(
    path="MASSIVE_TODO_LIST.md",
    old_str="- [ ] Implement `GET /api/v1/battery/state` handler",
    new_str="- [x] Implement `GET /api/v1/battery/state` handler",
    description="Mark API handler task as complete"
)

# Step 8: Commit
bash_tool """
git add .
git commit -m 'feat: add GET /api/v1/battery/state endpoint

- Created battery API handlers
- Added OpenAPI documentation
- Integrated with routes
- Added tests
- Updated MASSIVE_TODO_LIST.md Phase 12

Tested with curl, returns current battery state'
"""
```

---

## 🚨 Critical Checks Before Finishing

**Run this checklist before marking any task complete:**

```bash
# 1. Format
bash_tool "cargo fmt"

# 2. Lint
bash_tool "cargo clippy --all-targets --all-features -- -D warnings"

# 3. Test
bash_tool "cargo test"

# 4. Check for dead code
bash_tool "rg 'TODO|FIXME' src/"

# 5. Check for debug prints
bash_tool "rg 'println!|dbg!' src/"

# 6. Check for commented code
bash_tool "rg '^\s*//' src/ | head -20"

# 7. Verify integration
bash_tool "cargo build --release"

# 8. Update TODO list (manually via str_replace)

# 9. Commit
bash_tool "git status"
bash_tool "git add ."
bash_tool "git commit -m '...'"
```

---

## 🎯 Common Patterns

### Pattern 1: Adding a New Domain Type

```python
# 1. Create the type file
create_file("src/domain/types/power.rs", "Create Power newtype", """
/// Power in watts
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Power(f64);

impl Power {
    pub fn from_watts(w: f64) -> Result<Self> {
        if w.is_finite() {
            Ok(Power(w))
        } else {
            Err(Error::InvalidPower)
        }
    }
    
    pub fn watts(&self) -> f64 { self.0 }
}
""")

# 2. Export it
str_replace("src/domain/types/mod.rs", 
    old_str="// Add new types here",
    new_str="pub mod power;\npub use power::Power;\n// Add new types here",
    description="Export Power type"
)

# 3. Test it
bash_tool "cargo test domain::types::power"
```

### Pattern 2: Adding a Database Migration

```bash
# 1. Create migration file
bash_tool "sqlx migrate add create_batteries"

# 2. Edit the migration
# (view will show the new file path)
bash_tool "ls -la migrations/"

# 3. Write the SQL
create_file("migrations/XXXXXX_create_batteries.sql", "Create batteries table", """
CREATE TABLE batteries (
    id UUID PRIMARY KEY,
    household_id UUID NOT NULL,
    capacity_kwh DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_batteries_household ON batteries(household_id);
""")

# 4. Run migration
bash_tool "sqlx migrate run"

# 5. Verify
bash_tool "psql $DATABASE_URL -c '\\d batteries'"
```

### Pattern 3: Adding an Integration Test

```python
create_file(
    "tests/integration/battery_integration_test.rs",
    "Create battery integration test",
    """
use open_energy_controller::*;

#[tokio::test]
async fn test_battery_full_workflow() {
    // Setup
    let config = Config::test();
    let app = create_app(config).await.unwrap();
    
    // Create battery
    let response = app
        .post("/api/v1/devices")
        .json(&json!({"type": "battery", "capacity": 10.0}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    
    // Read state
    let response = app
        .get("/api/v1/battery/state")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    
    // Verify state
    let state: BatteryState = response.json().await.unwrap();
    assert!(state.soc_percent >= 0.0);
}
"""
)

bash_tool "cargo test --test battery_integration_test"
```

---

## 🐛 Troubleshooting

### "File not found" errors

```bash
# Check if file exists
bash_tool "ls -la src/domain/battery.rs"

# Check the full path
bash_tool "pwd"
bash_tool "fd battery.rs"
```

### "str_replace failed - string not found"

```bash
# View the file first
view src/api/routes.rs

# Find the exact string
bash_tool "rg 'Add new routes' src/api/routes.rs"

# Use the EXACT string from the file
```

### "Compilation errors"

```bash
# Get detailed errors
bash_tool "cargo check 2>&1 | head -50"

# View the problematic file
view src/domain/battery.rs

# Fix and retry
```

### "Tests failing"

```bash
# Run single test with output
bash_tool "cargo test test_battery -- --nocapture"

# View test file
view tests/integration/battery_test.rs

# Debug
bash_tool "RUST_LOG=debug cargo test test_battery"
```

---

## ✅ Final Checklist

Before you finish ANY task, verify:

```bash
# [ ] Code compiles
bash_tool "cargo build"

# [ ] No warnings
bash_tool "cargo clippy -- -D warnings"

# [ ] Tests pass
bash_tool "cargo test"

# [ ] Formatted
bash_tool "cargo fmt -- --check"

# [ ] TODO updated
view MASSIVE_TODO_LIST.md
# Then: str_replace to mark items complete

# [ ] No debug code
bash_tool "rg 'println!|dbg!|TODO|FIXME' src/"

# [ ] Committed
bash_tool "git status"
bash_tool "git add ."
bash_tool "git commit -m '...'"
```

---

## 🎓 Learning Resources

**Rust:**
- https://doc.rust-lang.org/book/
- https://doc.rust-lang.org/rust-by-example/

**Async Rust:**
- https://rust-lang.github.io/async-book/
- https://tokio.rs/tokio/tutorial

**This Project:**
- Read `docs/ARCHITECTURE.md` for system design
- Read `docs/ADR/*.md` for design decisions
- Search code: `rg "pattern" src/`
- View examples: `view tests/integration/`

---

## 💪 You've Got This!

**Remember:**
1. ✅ Plan before coding
2. ✅ Create complete implementations
3. ✅ Test immediately
4. ✅ Integrate everything
5. ✅ Update TODO list
6. ✅ Clean up after yourself
7. ✅ Commit with good messages

**Your computer use capabilities make you incredibly powerful. Use them to build something amazing!** 🚀

---

**Now go implement some features!** 🔋⚡
