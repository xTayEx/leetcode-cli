# AGENTS.md — leetcode-cli

> Rust CLI tool for interacting with LeetCode from the command line.
> Binary: `leetcode`, crate: `leetcode-cli`, edition 2024, stable toolchain.

## Build / Lint / Test Commands

```bash
# Build
cargo build                          # Debug build
cargo build --release                # Release build
cargo build --all-features           # Build with optional `pym` (PyO3) feature

# Lint
cargo fmt --check                    # Format check (CI enforced)
cargo fmt                            # Auto-format
cargo clippy --all-features -- -D warnings  # Clippy (CI enforced, warnings are errors)

# Test — all
cargo nextest run --release --all-features  # CI uses nextest
cargo test --all-features                   # Standard test runner also works

# Test — single test
cargo test de_from_test_success             # Run by test function name
cargo test de_from_                         # Run all tests matching prefix
cargo test --test de                        # Run a specific integration test file (tests/de.rs)

# Test — single test with nextest
cargo nextest run -E 'test(de_from_test_success)'
```

### CI Pipeline (.github/workflows/rust.yml)

- **build**: Runs on macOS + Ubuntu, `cargo nextest run --release --all-features`
- **check**: Runs on Ubuntu, `cargo fmt --check` then `cargo clippy --all-features -- -D warnings`
- Toolchain: `rustup update stable && rustup default stable`

### System Dependencies

```
gcc, libssl-dev, libdbus-1-dev, libsqlite3-dev
```

## Architecture Overview

```
src/
  bin/lc.rs          # Binary entrypoint — builds tokio runtime, calls cli::main()
  cli.rs             # Clap CLI definition (Parser/Subcommand derives), dispatches to cmd/*
  lib.rs             # Crate root — module declarations, re-exports (Cache, Config, Error, Result)
  err.rs             # Error enum (thiserror) + Result type alias
  helper.rs          # Utility traits/functions (Digit, HTML renderer, file paths, filters)
  pym.rs             # Optional Python scripting via PyO3 (feature-gated: "pym")
  cache/
    mod.rs           # Cache struct — wraps LeetCode client, manages SQLite via Diesel
    models.rs        # Data models: Problem, Question, VerifyResult, RunCode (Diesel + serde)
    parser.rs        # JSON response parsers (return Option<T>, not Result<T>)
    schemas.rs       # Diesel table! macros for SQLite schema
    sql.rs           # Raw SQL CREATE TABLE strings
  cmd/
    mod.rs           # Submodule declarations + re-exports of *Args structs
    pick.rs, list.rs, edit.rs, exec.rs, test.rs, stat.rs, data.rs, completions.rs
  config/
    mod.rs           # Config struct (Deserialize/Serialize), locate/sync from TOML
    code.rs, cookies.rs, storage.rs, sys.rs
  plugins/
    mod.rs           # Plugin module (chrome cookies, LeetCode API)
    chrome.rs        # Cookie extraction via `rookie` crate
    leetcode.rs      # HTTP client (reqwest) with GraphQL queries to LeetCode API
tests/
  de.rs              # Integration tests — deserialization of LeetCode API responses
```

## Code Style

### Formatting

- **rustfmt.toml**: `edition = "2021"`, `tab_spaces = 4`
- 4-space indentation, no tabs
- Run `cargo fmt` before committing

### Imports

Imports follow a loose grouping — not strictly enforced but the prevailing pattern is:

1. `self::` / `super::` imports (local submodules)
2. `crate::` imports (internal)
3. External crate imports
4. `std::` imports

```rust
// Example from cache/mod.rs:
use self::models::*;
use self::schemas::{problems::dsl::*, tags::dsl::*};
use self::sql::*;
use crate::helper::test_cases_path;
use crate::{config::Config, err::Error, plugins::LeetCode};
use anyhow::anyhow;
use colored::Colorize;
use diesel::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
```

Some files use `crate::{Error, Result}` shorthand (re-exported from lib.rs).
Function-scoped imports are common for rarely-used types (e.g., `use crate::cache::Cache;` inside `fn run()`).

### Error Handling

- **Custom error enum** in `src/err.rs` using `thiserror::Error` derive
- **Result type alias**: `pub type Result<T> = std::result::Result<T, Error>;` (re-exported from lib.rs)
- **anyhow** for ad-hoc errors: `Err(anyhow!("message").into())`
- **`?` operator** used extensively for propagation
- **`#[error(transparent)]`** for wrapping external errors (reqwest, serde_json, io, etc.)
- Manual `From<diesel::result::Error>` impl converts diesel errors to `Error::Anyhow`
- Parser functions (`cache/parser.rs`) return `Option<T>` instead of `Result<T>`, using `?` on Option

### Async Patterns

- **Runtime**: Tokio multi-threaded (`tokio::runtime::Builder::new_multi_thread()`)
- Binary creates runtime in `main()`, calls `block_on(cli::main())`
- Command handlers are `pub async fn run(&self) -> Result<(), Error>`
- `reqwest` for async HTTP; responses consumed via `.json().await?` or `.text().await?`
- `self.clone()` pattern used with `LeetCode` client before async calls (the client is `Clone`)
- No `#[tokio::main]` — runtime is built manually in `src/bin/lc.rs`

### Naming Conventions

- **Structs**: PascalCase (`LeetCode`, `VerifyResult`, `PickArgs`)
- **Enums**: PascalCase with PascalCase variants (`Error::CookieError`, `Run::Test`, `Mode::Get`)
- **Functions**: snake_case (`get_question`, `download_problems`, `code_path`)
- **Modules**: snake_case, one file per module or `mod.rs` pattern
- **Constants**: SCREAMING_SNAKE_CASE (`CREATE_PROBLEMS_IF_NOT_EXISTS`, `LEETCODE_CSRF_ENV`)
- **Statics**: SCREAMING_SNAKE_CASE (`DONE`, `LOCK`, `QUERY_HELP`)
- **Command args structs**: Named `{Command}Args` (e.g., `PickArgs`, `ListArgs`, `EditArgs`)

### Derive Macros

- Data models: `#[derive(Clone, Debug, Serialize, Deserialize)]` (serde)
- Diesel models: `#[derive(Insertable, Queryable, AsChangeset, Identifiable)]`
- CLI args: `#[derive(Args)]` (clap), CLI entry: `#[derive(Parser)]`, subcommands: `#[derive(Subcommand)]`
- Error enum: `#[derive(thiserror::Error, Debug)]`
- Enums with defaults: `#[derive(Default)]` with `#[default]` on variant

### Visibility

- Public API types are `pub` (structs, their fields, key functions)
- Internal submodules use `mod` (private) — e.g., `mod chrome;` in plugins
- Re-exports via `pub use` in `mod.rs` files to flatten the API
- lib.rs re-exports key types: `pub use cache::Cache; pub use config::Config; pub use err::{Error, Result};`

### Logging

- Uses `log` crate with `env_logger`
- `#[macro_use] extern crate log;` in lib.rs enables `trace!()`, `debug!()`, `info!()` macros globally
- `trace!()` for detailed flow, `debug!()` for data inspection, `info!()` for user-visible status

### String Handling

- Function parameters: `&str` for borrowed strings, `String` for owned
- Conversions: `.to_string()`, `.to_owned()`, `.into()`
- String building: `format!()` macro, manual `push_str` for Display impls
- Raw string literals for help text: `r#"..."#`

### Diesel / SQLite

- `#[macro_use] extern crate diesel;` in lib.rs
- Schema defined via `table!` macro in `cache/schemas.rs`
- Connections created per-operation: `SqliteConnection::establish(&path)`
- DSL imports: `use self::schemas::{problems::dsl::*, tags::dsl::*};`

### Clap CLI

- Clap v4 with derive API (`Parser`, `Subcommand`, `Args`)
- Each subcommand is a separate file in `cmd/` with an `*Args` struct
- `impl *Args { pub async fn run(&self) -> Result<()> }` pattern for all commands
- Arg groups for mutually related options: `#[command(group = clap::ArgGroup::new(...))]`
- Aliases: `#[command(visible_alias = "d")]`

### Conditional Compilation

- `#[cfg(feature = "pym")]` gates Python integration module
- `#[cfg(target_family = "unix")]` for Unix-only signal handling (nix crate)
- `#[cfg(debug_assertions)]` / `#[cfg(not(debug_assertions))]` for debug vs release config paths

### Testing

- Integration tests live in `tests/de.rs` — focused on serde deserialization
- Tests use `#[test]` (synchronous), no `#[tokio::test]` currently
- Test pattern: deserialize JSON string into `VerifyResult`, assert `r.is_ok()`
- No unit tests in `src/` files (no `#[cfg(test)] mod tests` blocks)
- `#[allow(clippy::useless_let_if_seq)]` used selectively to suppress specific clippy lints

### Patterns to Follow

1. New subcommands: create `src/cmd/{name}.rs` with `{Name}Args` struct, add to `cmd/mod.rs` and `cli.rs`
2. New errors: add variant to `Error` enum in `err.rs` with `#[error("...")]`
3. New config fields: add to relevant struct in `config/`, ensure `Serialize + Deserialize`
4. New API calls: add method to `LeetCode` in `plugins/leetcode.rs` using `Req` struct
5. Wrap external errors with `#[error(transparent)]` and `#[from]`
6. Always propagate errors with `?`, convert with `.into()` when needed
