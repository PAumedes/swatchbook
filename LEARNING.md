# Swatchbook Learning Journal

A companion document tracking Rust concepts as we build Swatchbook from scratch. Start here if you're new to Rust.

---

## Part 0: Rust Fundamentals (Phase 0 Walkthrough)

### Editions vs. Versions

**Edition** = Language dialect (like PHP 7 vs PHP 8)
- `edition = "2021"` in Cargo.toml means "use 2021 edition Rust syntax"
- Editions: `2015`, `2018`, `2021`
- Independent from compiler version

**Version** = Compiler release (like rustc 1.75)
- `rust-version = "1.75"` means "require Rust compiler 1.75 or newer"
- Released every 6 weeks
- Newer versions can compile older edition code

**In practice:**
```toml
edition = "2021"       # Modern syntax
rust-version = "1.75"  # Requires this compiler or newer
```

---

### Build Systems: Meson + Cargo

**Cargo** = Rust's built-in package manager + compiler driver
- Handles Rust code compilation
- Manages Rust dependencies
- Reads from `Cargo.toml`

**Meson** = Project-wide build orchestrator
- Calls Cargo (via `build-aux/cargo.sh` wrapper)
- Compiles Blueprint UI files to XML
- Validates GSettings schema
- Installs files to system directories
- Handles localization merge

**Why both?** A GTK app isn't pure Rust — it also has UI files, desktop metadata, and translations.

---

### The Module System: `mod` Keyword

```rust
mod config;
mod window;
```

**What it does:** Include another Rust file as a submodule.

Rust looks for:
- `src/config.rs` (or `src/config/mod.rs`)
- `src/window.rs` (or `src/window/mod.rs`)

**In PHP terms:**
```php
// Rust's mod is similar to including a file that defines a namespace
require 'config.php';  // defines namespace Config
require 'window.php';  // defines namespace Window
```

**In Visual Basic terms:**
```vb
' Like referencing another project or namespace
Imports Config
Imports Window
```

Once you `mod window;`, everything public in window.rs is accessible in main.rs.

---

### Imports: `use` Keyword

```rust
use adw::prelude::*;
use gtk::{gio, glib};
use crate::config::{APP_ID, GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR, VERSION};
use crate::window::SwatchbookWindow;
```

**What it does:** Bring names into scope so you don't type full paths.

**`adw::prelude::*`** — Import *everything* from `adw`'s prelude module.
- Prelude = commonly-used items (traits, types, etc.)
- The `*` means "all of them"
- Idiomatic Rust pattern

**`gtk::{gio, glib}`** — Import *just* `gio` and `glib` from the `gtk` crate.
- Selective import = less namespace pollution

**`crate::config::{...}`** — Import from *this project's* code.
- `crate::` = "from the root of my project"
- `config` = the module we declared with `mod config;`
- `{APP_ID, ...}` = specific constants from that module

**In PHP terms:**
```php
use AdwPrelude\*;                  // import everything
use Gtk\{Gio, Glib};              // import selective items
use App\Config\{APP_ID, VERSION};  // import from own project
```

---

### Function Signatures: Return Types

```rust
fn main() -> glib::ExitCode {
```

**The `->` syntax:** declares the return type.

**In Visual Basic:**
```vb
Function Main() As ExitCode
```

**In PHP:**
```php
function main(): ExitCode {
```

**Why it matters in Rust:**
- Rust *requires* explicit return types on public functions
- The compiler checks that you actually return what you promised
- This is a safety feature — no silent type mismatches

---

## Part 1: Application Bootstrap (main.rs)

### The Four Stages of main()

1. **Localization Setup** (lines 17–28)
   - Initialize gettext so UI strings get translated
   - Set the application name

2. **Load Resources** (lines 30–36)
   - Load the compiled Blueprint UI (from GResource bundle)
   - Register it so the app can find it at runtime

3. **Create Application** (lines 38–79)
   - Build an `Adw.Application` (top-level app object)
   - Register CLI options (`--new-canvas`)
   - Connect signal handlers (startup, activate, command-line)

4. **Run Main Loop** (line 81)
   - Hand control to GLib's event loop
   - App runs until user quits

---

### The Builder Pattern (Lines 39–43)

```rust
let app = adw::Application::builder()
    .application_id(APP_ID)
    .resource_base_path("/io/github/patricioaumedes/Swatchbook")
    .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
    .build();
```

**Pattern:** Create a builder → set properties → call `.build()` → get final object

**Why?** Ensures the object is fully configured before you use it.

**In PHP:**
```php
$app = ApplicationBuilder::create()
    ->setApplicationId(APP_ID)
    ->setFlags(ApplicationFlags::HANDLES_COMMAND_LINE)
    ->build();
```

---

### Variables: The `let` Keyword (Line 39)

```rust
let app = adw::Application::builder().build();
```

- `let` binds a name to a value
- Variables are **immutable by default** — can't be changed after creation
- Use `let mut app = ...` to make it mutable
- Rust infers the type (`adw::Application`)

**Why immutability by default?** Catches bugs. If you forget to modify something you intended to change, Rust warns you.

---

### Closures: Anonymous Functions (Lines 55–79)

```rust
app.connect_startup(|app| {
    setup_gactions(app);
    setup_accels(app);
});
```

**The `|app| { ... }` syntax** is a closure — an anonymous function.

- `|app|` = parameters (like `function(app) { }` in PHP)
- `{ ... }` = body (what it does)

**Three signal handlers we register:**

1. **`connect_startup`** — Called once at initialization
   - Sets up app actions and keyboard shortcuts

2. **`connect_activate`** — Called when showing the window
   - We pass function name: `build_window` (Rust wraps it in a closure automatically)

3. **`connect_command_line`** — Called when user passes CLI args
   - Checks for `--new-canvas` flag
   - Returns exit code

**In PHP terms:**
```php
$app->onStartup(function($app) {
    setupGactions($app);
    setupAccels($app);
});
```

---

### Borrowing: The `&` Operator

When you pass a variable to a closure, you're **borrowing** it, not giving ownership.

```rust
let app = adw::Application::builder().build();
app.connect_startup(|app| {  // |app| is a borrow (&app)
    app.doSomething();
});
// app is still valid here — we only borrowed it to the closure
```

**Key idea:**
- `app` (ownership) = "I own this value"
- `&app` (borrow) = "I'm using this temporarily, but don't own it"

Rust tracks borrowing at compile time. This prevents bugs like "someone freed this memory while I'm still using it."

---

### Error Handling Chain (Lines 69–73)

```rust
options
    .lookup::<bool>("new-canvas")
    .ok()
    .flatten()
    .unwrap_or(false)
```

This chain:
1. Try to look up the option → `Some(value)` or error
2. `.ok()` → convert error to `None`, now we have `Some(value)` or `None`
3. `.flatten()` → unwrap nested `Some(Some(x))` to `Some(x)`
4. `.unwrap_or(false)` → if still `None`, use `false` as default

**In PHP:**
```php
$isNewCanvas = $options['new-canvas'] ?? false;
```

**Translation:** "Get new-canvas flag if it exists and is valid, otherwise assume false."

This pattern is common for optional/error-prone values.

---

### Key Rust Patterns (Learned So Far)

- ✅ **Builder Pattern** — `.builder().property(...).build()`
- ✅ **Closures** — `|param| { code }`
- ✅ **Borrowing** — `&variable` for temporary access
- ✅ **Error Handling** — `.ok().flatten().unwrap_or(default)`
- ⏳ **Pattern Matching** — `if let Some(x) = ...` (next: window.rs)

---

## Part 2: Window Class (window.rs)

*To be filled in as we study window.rs...*

---

## Part 3: Blueprint UI (window.blp)

*To be filled in as we study window.blp...*

---

## Vocabulary

| Term | Meaning |
|---|---|
| **Crate** | A Rust package (like an npm package or a PHP library) |
| **Module** | A namespace/organizational unit within a crate |
| **Trait** | An interface/contract (similar to PHP interfaces or VB abstract classes) |
| **Ownership** | Rust's memory management model (who owns which data) |
| **Borrow** | Temporary access to data without taking ownership |
| **Mutable** | Can be changed (`mut` keyword) |
| **Immutable** | Cannot be changed (default in Rust) |
| **Closure** | An anonymous function (like PHP arrow functions or VB lambdas) |

---

## Next Steps

- [ ] Understand main() line by line
- [ ] Understand window.rs and GObject subclassing
- [ ] Understand window.blp and Blueprint syntax
- [ ] Phase 1: Add Markdown parsing
