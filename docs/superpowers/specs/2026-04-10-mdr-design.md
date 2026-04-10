# mdr — Markdown Reader: Design Spec

**Date:** 2026-04-10  
**Project:** Build Rust from scratch — TUI Markdown Reader  
**Goal:** Learn Rust core concepts through a polished, daily-usable CLI tool

---

## Overview

`mdr` is a terminal-based markdown reader built with Rust. The project is structured as a 7-phase learning journey, where each phase introduces specific Rust concepts in the context of real, tangible features. The end goal is a daily-usable tool, not just a learning exercise.

**Target user:** The developer building it — someone who has read Rust docs but has no real project experience yet.

---

## Architecture

```
mdr/
├── src/
│   ├── main.rs          # Entry point, CLI argument parsing
│   ├── app.rs           # App state machine (core)
│   ├── ui/
│   │   ├── mod.rs       # UI render entry point
│   │   ├── file_tree.rs # File tree component
│   │   └── preview.rs   # Markdown preview component
│   ├── fs.rs            # File system operations
│   ├── markdown.rs      # Markdown parsing and rendering
│   └── error.rs         # Unified error types
├── Cargo.toml
└── docs/
    └── superpowers/specs/
```

### Core Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` | TUI framework |
| `crossterm` | Cross-platform terminal control |
| `pulldown-cmark` | Markdown parsing |
| `clap` | CLI argument parsing |
| `tokio` | Async runtime (Phase 7) |
| `thiserror` | Error type derivation |
| `anyhow` | Error propagation |
| `serde` + `toml` | Config file parsing |

---

## Learning Phases

### Phase 1 — Project Skeleton
**Rust concepts:** Module system, `Cargo.toml`, basic syntax, `println!` macro

1. `cargo new mdr --bin` — understand generated file structure
2. Add first dependency (`clap`) to `Cargo.toml`, understand semantic versioning
3. Use `clap` to parse `--version` and `--help`, run `mdr --help`
4. Create `src/error.rs`, define placeholder `AppError` type, learn `mod` keyword
5. `use` own module in `main.rs`, understand `pub` visibility

### Phase 2 — File Reading
**Rust concepts:** Ownership, `String` vs `&str`, `Result`, `?` operator

1. Accept CLI path argument (`clap` positional argument)
2. Read file with `std::fs::read_to_string`, understand `Result<String, io::Error>`
3. Explicitly handle `Ok`/`Err` with `match`, print content or error
4. Refactor with `?` operator, understand its semantics
5. Write `fn read_file(path: &str) -> Result<String>`, understand `&str` borrowing
6. Understand why returning a reference to a local `String` fails (borrow checker lesson 1)

### Phase 3 — Minimal TUI
**Rust concepts:** Structs, methods (`impl`), event loop, basic `enum`

1. Add `ratatui` + `crossterm` dependencies, run the official minimal example
2. Define `struct App { running: bool }`, write `impl App` with `new()` and `quit()`
3. Implement event loop: read key events, `q` to quit
4. Render a bordered empty `Block` with `ratatui`
5. Move UI rendering to `src/ui/mod.rs`, understand module splitting
6. Define `enum AppState { Browsing, Viewing }` to prepare for later phases

### Phase 4 — Markdown Rendering
**Rust concepts:** Traits (`Widget`), Iterators, Closures, `Vec`

1. Add `pulldown-cmark`, parse a `.md` file, `println!` the AST tokens
2. Write `fn render_markdown(content: &str) -> Vec<Line>`, convert tokens to ratatui `Line`
3. Handle Heading / Bold / Italic / Code styling (`Style`, `Span`)
4. Replace for loops with `Iterator::map` + closures, experience chained style
5. Create `src/ui/preview.rs`, implement `Widget` trait, display in TUI
6. Support up/down scrolling with arrow keys (scroll offset state)
7. Support `g`/`G` to jump to top/bottom

### Phase 5 — File Tree
**Rust concepts:** Generics, recursive Enum, Pattern matching, `Box<T>`

1. Define recursive structure: `enum FileNode { File(PathBuf), Dir(String, Vec<FileNode>) }`
2. Write `fn walk_dir(path: &Path) -> Result<FileNode>` to recursively traverse directories
3. Create `src/ui/file_tree.rs`, implement `Widget` to render tree structure
4. Implement keyboard navigation (up/down cursor, maintain `selected_index`)
5. `Enter` to expand/collapse directories, understand `match` exhaustiveness
6. `Enter` on a file loads its content into the preview panel
7. Implement split layout (left/right panels via `ratatui::layout`)

### Phase 6 — Error Handling + Config
**Rust concepts:** `thiserror`, Lifetimes, `serde`, `Deserialize`

1. Use `thiserror` to define a complete `AppError` enum, replace placeholder
2. Implement `From` trait conversions for each module's errors, unify error chain
3. Define `struct Config<'a>` with string references, understand lifetime annotations
4. Refactor to `struct Config { ... }` with owned data, compare the two approaches
5. Add `serde` + `toml`, read config from `~/.config/mdr/config.toml`
6. Support configurable key bindings (`HashMap<String, Action>`)
7. Support theme color config (foreground/background)

### Phase 7 — Async Loading
**Rust concepts:** `async/await`, `tokio`, `mpsc` channel, `Arc<Mutex<T>>`

1. Add `tokio`, convert `main` to `#[tokio::main] async fn main()`
2. Write `async fn load_file(path: PathBuf) -> Result<String>`
3. Use `tokio::spawn` for background loading, main thread stays unblocked
4. Use `std::sync::mpsc::channel` to send results back to the UI thread
5. Understand why `Arc<Mutex<T>>` is needed for shared state
6. Show a loading spinner in TUI while file is loading
7. Handle load errors: display friendly error message in preview panel

---

## Data Flow

```
User Input (keyboard)
       │
       ▼
  Event Loop (app.rs)
       │
  ┌────┴────┐
  │         │
  ▼         ▼
AppState  File I/O (fs.rs)
  │         │
  │         ▼
  │    Markdown Parser (markdown.rs)
  │         │
  └────┬────┘
       │
       ▼
  UI Render (ui/)
  ├── FileTree widget
  └── Preview widget
```

---

## Error Handling Strategy

- All internal errors flow through a unified `AppError` enum (Phase 6)
- `?` operator used throughout for propagation
- UI layer catches errors and displays them inline (never crash)
- File not found, parse errors, and IO errors each have distinct variants

---

## Testing Strategy

- Phase 1–3: Manual testing only (TUI is hard to unit test)
- Phase 4: Unit tests for `render_markdown()` — input markdown string → expected `Vec<Line>`
- Phase 5: Unit tests for `walk_dir()` using temp directories (`tempfile` crate)
- Phase 6: Unit tests for config parsing
- Phase 7: Integration tests for async load behavior

---

## Success Criteria

- `mdr <file>` opens any `.md` file with formatted rendering
- `mdr` (no args) opens a file browser in the current directory
- Handles large files without freezing the UI
- Config file allows customizing key bindings and colors
- Works on macOS and Linux
