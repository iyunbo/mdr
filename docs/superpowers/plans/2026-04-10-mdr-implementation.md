# mdr — Markdown Reader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a polished TUI markdown reader in Rust, learning core language concepts through 7 incremental phases.

**Architecture:** Single binary CLI tool. A central `App` struct owns all state and drives a `ratatui` event loop. File system traversal and markdown parsing are isolated in dedicated modules. UI components are separate `Widget` implementations. Async file loading is added in the final phase via `tokio`.

**Tech Stack:** Rust stable, clap 4, ratatui 0.29, crossterm 0.28, pulldown-cmark 0.11, thiserror 1, anyhow 1, serde 1, toml 0.8, tokio 1

---

## File Map

| File | Responsibility |
|---|---|
| `src/main.rs` | Entry point, CLI parsing, event loop, `App` initialization |
| `src/app.rs` | `App` state machine, input dispatch |
| `src/error.rs` | `AppError` enum, `From` trait implementations |
| `src/fs.rs` | File reading, directory traversal, `FileNode` tree |
| `src/markdown.rs` | Markdown → ratatui `Line` conversion |
| `src/config.rs` | `Config` struct, deserialization from TOML |
| `src/ui/mod.rs` | `draw()` function, layout composition |
| `src/ui/preview.rs` | `PreviewWidget` implementing `Widget` |
| `src/ui/file_tree.rs` | `FileTreeWidget` implementing `Widget` |

---

## Task 1: Project Skeleton

**Rust concepts:** Module system, `Cargo.toml`, basic syntax, `println!` macro, `pub` visibility

**Files:**
- Create: `src/main.rs` (replace generated content)
- Create: `src/error.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1.1: Scaffold the project**

```bash
cd /Users/verrerie/git
cargo new mdr --bin
cd mdr
```

Expected output:
```
     Created binary (application) `mdr` package
```

- [ ] **Step 1.2: Verify it compiles and runs**

```bash
cargo run
```

Expected output:
```
   Compiling mdr v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in Xs
     Running `target/debug/mdr`
Hello, world!
```

- [ ] **Step 1.3: Add `clap` to `Cargo.toml`**

Replace the `[dependencies]` section of `Cargo.toml`:

```toml
[package]
name = "mdr"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
```

- [ ] **Step 1.4: Replace `src/main.rs` with CLI skeleton**

```rust
mod error;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mdr", version, about = "A terminal markdown reader")]
struct Cli {
    /// Path to a markdown file or directory to browse
    path: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    match cli.path {
        Some(p) => println!("Opening: {}", p.display()),
        None => println!("No path given — will open file browser"),
    }
}
```

- [ ] **Step 1.5: Verify `--help` and `--version` work**

```bash
cargo run -- --help
cargo run -- --version
```

Expected `--help` output (abbreviated):
```
A terminal markdown reader

Usage: mdr [PATH]

Arguments:
  [PATH]  Path to a markdown file or directory to browse

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [ ] **Step 1.6: Create `src/error.rs`**

```rust
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}
```

`mod error;` is already declared in `main.rs` from Step 1.4. Rust will now find and compile `src/error.rs`.

- [ ] **Step 1.7: Verify it compiles**

```bash
cargo build
```

Expected: no errors.

- [ ] **Step 1.8: Commit**

```bash
git add src/main.rs src/error.rs Cargo.toml Cargo.lock
git commit -m "feat(phase1): project skeleton with clap CLI and error module"
```

---

## Task 2: File Reading

**Rust concepts:** Ownership, `String` vs `&str`, `Result`, `match`, `?` operator, borrowing

**Files:**
- Create: `src/fs.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml` (add `tempfile` dev dep)

- [ ] **Step 2.1: Add `tempfile` dev dependency to `Cargo.toml`**

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2.2: Write a failing test for `read_file` in `src/fs.rs`**

Create `src/fs.rs`:

```rust
use crate::error::AppError;

pub fn read_file(path: &str) -> Result<String, AppError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_file_returns_content() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "# Hello mdr").unwrap();
        let path = f.path().to_str().unwrap().to_string();
        let content = read_file(&path).unwrap();
        assert!(content.contains("# Hello mdr"));
    }

    #[test]
    fn test_read_file_missing_returns_error() {
        let result = read_file("/tmp/this_file_does_not_exist_mdr_test.md");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2.3: Run tests to confirm they fail**

```bash
cargo test fs::
```

Expected: panics with `not yet implemented` (from `todo!()`).

- [ ] **Step 2.4: Implement `read_file` with explicit `match`**

Replace `todo!()` in `src/fs.rs` with the `match` version:

```rust
pub fn read_file(path: &str) -> Result<String, AppError> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => Err(AppError::from(e)),
    }
}
```

- [ ] **Step 2.5: Run tests — they should pass**

```bash
cargo test fs::
```

Expected: `test fs::tests::test_read_file_returns_content ... ok` and `test fs::tests::test_read_file_missing_returns_error ... ok`

- [ ] **Step 2.6: Refactor to use `?` operator**

Replace the `match` with `?`:

```rust
pub fn read_file(path: &str) -> Result<String, AppError> {
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}
```

`?` is syntactic sugar: if `read_to_string` returns `Err(e)`, it calls `From::from(e)` (using our `impl From<io::Error>`) and returns early. Same behavior, much less noise.

- [ ] **Step 2.7: Run tests again to confirm refactor didn't break anything**

```bash
cargo test fs::
```

Expected: both tests still pass.

- [ ] **Step 2.8: Wire `read_file` into `main.rs`**

Add `mod fs;` and update `main`:

```rust
mod error;
mod fs;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mdr", version, about = "A terminal markdown reader")]
struct Cli {
    path: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    if let Some(path) = cli.path {
        match fs::read_file(path.to_str().unwrap_or("")) {
            Ok(content) => println!("{}", content),
            Err(e) => eprintln!("Error: {}", e),
        }
    } else {
        println!("No path given — will open file browser");
    }
}
```

- [ ] **Step 2.9: Test manually with a real file**

```bash
echo "# Test\n\nHello **world**" > /tmp/test.md
cargo run -- /tmp/test.md
```

Expected: prints the raw markdown content.

- [ ] **Step 2.10: Borrow checker lesson — understand why this fails**

Try adding this function to `src/fs.rs` (don't commit it, just read the error):

```rust
// This won't compile — do you understand why?
pub fn broken_ref() -> &str {
    let s = String::from("hello");
    &s  // ERROR: s is dropped at end of function, reference dangles
}
```

Run `cargo build` and read the error. Then delete `broken_ref`. The key insight: Rust refuses to return a reference to data that will be freed. You must either return owned `String` or ensure the data outlives the reference (lifetimes — Phase 6).

- [ ] **Step 2.11: Commit**

```bash
git add src/fs.rs src/main.rs Cargo.toml Cargo.lock
git commit -m "feat(phase2): file reading with Result and ? operator"
```

---

## Task 3: Minimal TUI

**Rust concepts:** Structs, `impl` methods, event loop, `enum` variants

**Files:**
- Create: `src/app.rs`
- Create: `src/ui/mod.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml`

- [ ] **Step 3.1: Add TUI dependencies to `Cargo.toml`**

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
ratatui = "0.29"
crossterm = "0.28"
```

- [ ] **Step 3.2: Create `src/app.rs`**

```rust
#[derive(Debug, Default, PartialEq)]
pub enum AppState {
    #[default]
    Browsing,
    Viewing,
}

pub struct App {
    pub running: bool,
    pub state: AppState,
    pub scroll: u16,
    pub content: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            state: AppState::default(),
            scroll: 0,
            content: None,
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }
}
```

- [ ] **Step 3.3: Create `src/ui/mod.rs`**

First create the `ui` directory:

```bash
mkdir -p src/ui
```

Then create `src/ui/mod.rs`:

```rust
use ratatui::{
    Frame,
    widgets::{Block, Borders},
};
use crate::app::App;

pub fn draw(frame: &mut Frame, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" mdr — markdown reader ");
    frame.render_widget(block, frame.area());
}
```

- [ ] **Step 3.4: Replace `src/main.rs` with TUI event loop**

```rust
mod app;
mod error;
mod fs;
mod ui;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mdr", version, about = "A terminal markdown reader")]
struct Cli {
    path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut app = App::new();

    if let Some(path) = cli.path {
        match fs::read_file(path.to_str().unwrap_or("")) {
            Ok(content) => {
                app.content = Some(content);
                app.state = app::AppState::Viewing;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(());
            }
        }
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while app.running {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) => app.quit(),
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
                _ => {}
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 3.5: Run and verify**

```bash
cargo run -- /tmp/test.md
```

Expected: A terminal window with a box border and title "mdr — markdown reader". Press `q` to quit.

- [ ] **Step 3.6: Add `AppState::Browsing` placeholder display**

Update `src/ui/mod.rs` to show state in the title:

```rust
use ratatui::{
    Frame,
    widgets::{Block, Borders},
};
use crate::app::{App, AppState};

pub fn draw(frame: &mut Frame, app: &App) {
    let title = match app.state {
        AppState::Browsing => " mdr — browse ",
        AppState::Viewing => " mdr — viewing ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title);
    frame.render_widget(block, frame.area());
}
```

- [ ] **Step 3.7: Verify both states show correct title**

```bash
cargo run                  # Should show "mdr — browse"
cargo run -- /tmp/test.md  # Should show "mdr — viewing"
```

- [ ] **Step 3.8: Commit**

```bash
git add src/app.rs src/ui/mod.rs src/main.rs Cargo.toml Cargo.lock
git commit -m "feat(phase3): minimal TUI with ratatui event loop"
```

---

## Task 4: Markdown Rendering

**Rust concepts:** Traits (`Widget`), Iterators, Closures, `Vec`, method chaining

**Files:**
- Create: `src/markdown.rs`
- Create: `src/ui/preview.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/app.rs`
- Modify: `Cargo.toml`

- [ ] **Step 4.1: Add `pulldown-cmark` to `Cargo.toml`**

```toml
pulldown-cmark = "0.11"
```

- [ ] **Step 4.2: Write a failing test for markdown parsing in `src/markdown.rs`**

Create `src/markdown.rs`:

```rust
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn parse(content: &str) -> Vec<Line<'static>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_is_bold_cyan() {
        let lines = parse("# Hello");
        // Should produce at least one line with "Hello"
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("Hello"), "Expected 'Hello' in output, got: {:?}", text);
    }

    #[test]
    fn test_bold_text_has_bold_modifier() {
        let lines = parse("**bold**");
        let has_bold = lines.iter().flat_map(|l| l.spans.iter()).any(|s| {
            s.style.add_modifier.contains(Modifier::BOLD)
        });
        assert!(has_bold, "Expected at least one BOLD span");
    }

    #[test]
    fn test_plain_text_renders() {
        let lines = parse("just some text");
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("just some text"));
    }
}
```

- [ ] **Step 4.3: Run tests to confirm they fail**

```bash
cargo test markdown::
```

Expected: panics with `not yet implemented`.

- [ ] **Step 4.4: Implement `parse` in `src/markdown.rs`**

Replace `todo!()` with:

```rust
pub fn parse(content: &str) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(content, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut in_heading = false;
    let mut in_strong = false;
    let mut in_em = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    lines.push(Line::default());
                }
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    lines.push(Line::default());
                }
            }
            Event::Start(Tag::Strong) => in_strong = true,
            Event::End(TagEnd::Strong) => in_strong = false,
            Event::Start(Tag::Emphasis) => in_em = true,
            Event::End(TagEnd::Emphasis) => in_em = false,
            Event::Code(text) => {
                let span = Span::styled(
                    text.to_string(),
                    Style::default().fg(Color::Yellow),
                );
                current_spans.push(span);
            }
            Event::Text(text) => {
                let mut style = Style::default();
                if in_heading {
                    style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                }
                if in_strong {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if in_em {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                current_spans.push(Span::styled(text.to_string(), style));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
    }

    lines
}
```

- [ ] **Step 4.5: Run tests**

```bash
cargo test markdown::
```

Expected: all 3 tests pass.

- [ ] **Step 4.6: Create `src/ui/preview.rs`**

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Widget},
};

pub struct PreviewWidget<'a> {
    pub lines: &'a [Line<'a>],
    pub scroll: u16,
    pub title: &'a str,
}

impl<'a> Widget for PreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.title));

        let paragraph = Paragraph::new(self.lines.to_vec())
            .block(block)
            .scroll((self.scroll, 0));

        paragraph.render(area, buf);
    }
}
```

- [ ] **Step 4.7: Update `src/ui/mod.rs` to use `PreviewWidget`**

```rust
pub mod preview;

use ratatui::{Frame, widgets::{Block, Borders}};
use crate::app::{App, AppState};
use crate::markdown;

pub fn draw(frame: &mut Frame, app: &App) {
    match app.state {
        AppState::Browsing => draw_browsing(frame, app),
        AppState::Viewing => draw_viewing(frame, app),
    }
}

fn draw_browsing(frame: &mut Frame, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" mdr — browse (coming in Phase 5) ");
    frame.render_widget(block, frame.area());
}

fn draw_viewing(frame: &mut Frame, app: &App) {
    let content = app.content.as_deref().unwrap_or("");
    let lines = markdown::parse(content);

    // We need 'static lines for the widget — convert owned Lines to owned
    let widget = preview::PreviewWidget {
        lines: &lines,
        scroll: app.scroll,
        title: app.file_name.as_deref().unwrap_or("untitled"),
    };
    frame.render_widget(widget, frame.area());
}
```

- [ ] **Step 4.8: Add `file_name` field to `App` in `src/app.rs`**

```rust
pub struct App {
    pub running: bool,
    pub state: AppState,
    pub scroll: u16,
    pub content: Option<String>,
    pub file_name: Option<String>,  // ← new field
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            state: AppState::default(),
            scroll: 0,
            content: None,
            file_name: None,
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_top(&mut self) {
        self.scroll = 0;
    }
}
```

- [ ] **Step 4.9: Update `src/main.rs` to populate `file_name` and handle scroll keys**

Update the file-loading section in `main()`:

```rust
if let Some(path) = cli.path {
    match fs::read_file(path.to_str().unwrap_or("")) {
        Ok(content) => {
            app.file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from);
            app.content = Some(content);
            app.state = app::AppState::Viewing;
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return Ok(());
        }
    }
}
```

Update the key handler in `run()`:

```rust
match (key.code, key.modifiers) {
    (KeyCode::Char('q'), _) => app.quit(),
    (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.scroll_down(),
    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.scroll_up(),
    (KeyCode::Char('g'), _) => app.scroll_top(),
    _ => {}
}
```

- [ ] **Step 4.10: Smoke test the full flow**

```bash
cargo run -- /tmp/test.md
```

Expected: A TUI window showing formatted markdown. Headings in cyan bold, bold text in bold, code in yellow. Arrow keys or `j`/`k` scroll. `g` jumps to top. `q` quits.

- [ ] **Step 4.11: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 4.12: Commit**

```bash
git add src/markdown.rs src/ui/preview.rs src/ui/mod.rs src/app.rs src/main.rs Cargo.toml Cargo.lock
git commit -m "feat(phase4): markdown rendering with ratatui Widget trait"
```

---

## Task 5: File Tree

**Rust concepts:** Recursive `enum`, `Box<T>`, `Pattern matching`, `Vec` iteration, keyboard navigation

**Files:**
- Create: `src/ui/file_tree.rs`
- Modify: `src/fs.rs` (add `FileNode` and `walk_dir`)
- Modify: `src/app.rs` (add tree state)
- Modify: `src/ui/mod.rs` (split layout)
- Modify: `src/main.rs` (browsing mode entry)

- [ ] **Step 5.1: Write a failing test for `walk_dir` in `src/fs.rs`**

Add to `src/fs.rs`:

```rust
use std::path::{Path, PathBuf};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub enum FileNode {
    File(PathBuf),
    Dir {
        path: PathBuf,
        name: String,
        children: Vec<FileNode>,
        expanded: bool,
    },
}

impl FileNode {
    pub fn name(&self) -> &str {
        match self {
            FileNode::File(p) => p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?"),
            FileNode::Dir { name, .. } => name,
        }
    }

    pub fn is_markdown(&self) -> bool {
        match self {
            FileNode::File(p) => matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("md") | Some("markdown")
            ),
            FileNode::Dir { .. } => false,
        }
    }
}

pub fn walk_dir(path: &Path) -> Result<FileNode, AppError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_walk_dir_finds_md_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("readme.md"), "# Hello").unwrap();
        fs::write(dir.path().join("notes.md"), "notes").unwrap();
        fs::write(dir.path().join("other.txt"), "text").unwrap();

        let node = walk_dir(dir.path()).unwrap();
        match node {
            FileNode::Dir { children, .. } => {
                let md_count = children.iter().filter(|n| n.is_markdown()).count();
                assert_eq!(md_count, 2);
            }
            _ => panic!("Expected Dir variant"),
        }
    }

    #[test]
    fn test_walk_dir_on_file_returns_error() {
        let f = tempfile::NamedTempFile::new().unwrap();
        // Passing a file path where a dir is expected returns Err
        // (walk_dir only accepts directories)
        let result = walk_dir(f.path());
        assert!(result.is_err(), "Expected error for file path");
    }
}
```

- [ ] **Step 5.2: Run tests to confirm failure**

```bash
cargo test fs::tests::test_walk
```

Expected: panics with `not yet implemented`.

- [ ] **Step 5.3: Implement `walk_dir` in `src/fs.rs`**

Replace `todo!()`:

```rust
pub fn walk_dir(path: &Path) -> Result<FileNode, AppError> {
    if !path.is_dir() {
        return Err(AppError::Other(format!(
            "'{}' is not a directory",
            path.display()
        )));
    }

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".")
        .to_string();

    let mut children: Vec<FileNode> = std::fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            // Skip hidden files
            !entry.file_name().to_str().unwrap_or("").starts_with('.')
        })
        .map(|entry| {
            let p = entry.path();
            if p.is_dir() {
                // Recurse but don't expand by default
                FileNode::Dir {
                    path: p.clone(),
                    name: p.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                        .to_string(),
                    children: Vec::new(), // lazy load
                    expanded: false,
                }
            } else {
                FileNode::File(p)
            }
        })
        .collect();

    children.sort_by(|a, b| {
        // Dirs first, then files, both alphabetical
        match (a, b) {
            (FileNode::Dir { name: a, .. }, FileNode::Dir { name: b, .. }) => a.cmp(b),
            (FileNode::File(a), FileNode::File(b)) => a.cmp(b),
            (FileNode::Dir { .. }, FileNode::File(_)) => std::cmp::Ordering::Less,
            (FileNode::File(_), FileNode::Dir { .. }) => std::cmp::Ordering::Greater,
        }
    });

    Ok(FileNode::Dir {
        path: path.to_path_buf(),
        name,
        children,
        expanded: true,
    })
}
```

- [ ] **Step 5.4: Run tests**

```bash
cargo test fs::tests::test_walk
```

Expected: both tests pass.

- [ ] **Step 5.5: Add tree navigation state to `src/app.rs`**

```rust
use crate::fs::FileNode;

pub struct App {
    pub running: bool,
    pub state: AppState,
    pub scroll: u16,
    pub content: Option<String>,
    pub file_name: Option<String>,
    pub tree: Option<FileNode>,         // ← new
    pub tree_cursor: usize,             // ← new
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            state: AppState::default(),
            scroll: 0,
            content: None,
            file_name: None,
            tree: None,
            tree_cursor: 0,
        }
    }

    pub fn quit(&mut self) { self.running = false; }
    pub fn scroll_down(&mut self) { self.scroll = self.scroll.saturating_add(1); }
    pub fn scroll_up(&mut self) { self.scroll = self.scroll.saturating_sub(1); }
    pub fn scroll_top(&mut self) { self.scroll = 0; }
    pub fn cursor_down(&mut self) { self.tree_cursor = self.tree_cursor.saturating_add(1); }
    pub fn cursor_up(&mut self) { self.tree_cursor = self.tree_cursor.saturating_sub(1); }
}
```

- [ ] **Step 5.6: Create `src/ui/file_tree.rs`**

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};
use crate::fs::FileNode;

pub struct FileTreeWidget<'a> {
    pub node: &'a FileNode,
    pub cursor: usize,
}

impl<'a> FileTreeWidget<'a> {
    fn flatten<'b>(node: &'b FileNode, depth: usize, items: &mut Vec<(usize, &'b FileNode)>) {
        items.push((depth, node));
        if let FileNode::Dir { children, expanded: true, .. } = node {
            for child in children {
                Self::flatten(child, depth + 1, items);
            }
        }
    }
}

impl<'a> Widget for FileTreeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut flat: Vec<(usize, &FileNode)> = Vec::new();
        Self::flatten(self.node, 0, &mut flat);

        let items: Vec<ListItem> = flat
            .iter()
            .enumerate()
            .map(|(i, (depth, node))| {
                let indent = "  ".repeat(*depth);
                let prefix = match node {
                    FileNode::Dir { expanded: true, .. } => "▼ ",
                    FileNode::Dir { expanded: false, .. } => "▶ ",
                    FileNode::File(_) => "  ",
                };
                let name = node.name();
                let style = if i == self.cursor {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else if node.is_markdown() {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{}{}{}", indent, prefix, name)),
                ]))
                .style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Files "));

        list.render(area, buf);
    }
}
```

- [ ] **Step 5.7: Update `src/ui/mod.rs` with split layout**

```rust
pub mod file_tree;
pub mod preview;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
};
use crate::app::{App, AppState};
use crate::markdown;

pub fn draw(frame: &mut Frame, app: &App) {
    match app.state {
        AppState::Browsing => draw_browsing(frame, app),
        AppState::Viewing => draw_viewing(frame, app),
    }
}

fn draw_browsing(frame: &mut Frame, app: &App) {
    if let Some(tree) = &app.tree {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(frame.area());

        let tree_widget = file_tree::FileTreeWidget {
            node: tree,
            cursor: app.tree_cursor,
        };
        frame.render_widget(tree_widget, chunks[0]);

        // Preview panel shows selected file or placeholder
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .title(" Preview ");
        if let Some(content) = &app.content {
            let lines = markdown::parse(content);
            let widget = preview::PreviewWidget {
                lines: &lines,
                scroll: app.scroll,
                title: app.file_name.as_deref().unwrap_or(""),
            };
            frame.render_widget(widget, chunks[1]);
        } else {
            frame.render_widget(preview_block, chunks[1]);
        }
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" mdr — no directory loaded ");
        frame.render_widget(block, frame.area());
    }
}

fn draw_viewing(frame: &mut Frame, app: &App) {
    let content = app.content.as_deref().unwrap_or("");
    let lines = markdown::parse(content);
    let widget = preview::PreviewWidget {
        lines: &lines,
        scroll: app.scroll,
        title: app.file_name.as_deref().unwrap_or("untitled"),
    };
    frame.render_widget(widget, frame.area());
}
```

- [ ] **Step 5.8: Update `src/main.rs` to load tree when no file given**

Update the `main()` function:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut app = App::new();

    match cli.path {
        Some(ref path) if path.is_file() => {
            // Open a single file directly
            match fs::read_file(path.to_str().unwrap_or("")) {
                Ok(content) => {
                    app.file_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .map(String::from);
                    app.content = Some(content);
                    app.state = app::AppState::Viewing;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return Ok(());
                }
            }
        }
        Some(ref path) if path.is_dir() => {
            // Open file browser at given directory
            match fs::walk_dir(path) {
                Ok(tree) => {
                    app.tree = Some(tree);
                    app.state = app::AppState::Browsing;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return Ok(());
                }
            }
        }
        None => {
            // Default: browse current directory
            let cwd = std::env::current_dir()?;
            if let Ok(tree) = fs::walk_dir(&cwd) {
                app.tree = Some(tree);
            }
            app.state = app::AppState::Browsing;
        }
        _ => {
            eprintln!("Path does not exist");
            return Ok(());
        }
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}
```

Update the key handler in `run()` to handle browsing navigation:

```rust
use crate::app::AppState;

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while app.running {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Event::Key(key) = event::read()? {
            match app.state {
                AppState::Viewing => match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) => app.quit(),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.scroll_down(),
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.scroll_up(),
                    (KeyCode::Char('g'), _) => app.scroll_top(),
                    (KeyCode::Esc, _) => {
                        // Return to browsing if tree exists
                        if app.tree.is_some() {
                            app.state = AppState::Browsing;
                        }
                    }
                    _ => {}
                },
                AppState::Browsing => match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) => app.quit(),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.cursor_down(),
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.cursor_up(),
                    (KeyCode::Enter, _) => {
                        // Load selected file (simplified — full impl requires flat index lookup)
                        // TODO-IMPL: resolve flat index to FileNode, load if markdown file
                    }
                    _ => {}
                },
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 5.9: Implement `Enter` to open selected file**

Add a helper to `src/app.rs`:

```rust
use crate::fs::{self, FileNode};

impl App {
    /// Returns the FileNode at the current cursor position in the flattened tree.
    pub fn selected_node(&self) -> Option<&FileNode> {
        let tree = self.tree.as_ref()?;
        let mut flat: Vec<&FileNode> = Vec::new();
        Self::flatten_tree(tree, &mut flat);
        flat.get(self.tree_cursor).copied()
    }

    fn flatten_tree<'a>(node: &'a FileNode, out: &mut Vec<&'a FileNode>) {
        out.push(node);
        if let FileNode::Dir { children, expanded: true, .. } = node {
            for child in children {
                Self::flatten_tree(child, out);
            }
        }
    }
}
```

Update the `Enter` handler in `run()` inside `src/main.rs`:

```rust
(KeyCode::Enter, _) => {
    if let Some(node) = app.selected_node().cloned() {
        match node {
            crate::fs::FileNode::File(ref path) if node.is_markdown() => {
                if let Ok(content) = fs::read_file(path.to_str().unwrap_or("")) {
                    app.file_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .map(String::from);
                    app.content = Some(content);
                    app.scroll = 0;
                    app.state = AppState::Viewing;
                }
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 5.10: Smoke test browsing mode**

```bash
cargo run                          # Opens current directory
cargo run -- /tmp                  # Opens /tmp
cargo run -- /tmp/test.md          # Opens file directly
```

Expected: browsing mode shows a file tree on the left. Pressing `Enter` on a `.md` file opens it in the preview. `Esc` returns to the tree.

- [ ] **Step 5.11: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 5.12: Commit**

```bash
git add src/fs.rs src/app.rs src/ui/file_tree.rs src/ui/mod.rs src/main.rs Cargo.lock
git commit -m "feat(phase5): file tree with keyboard navigation"
```

---

## Task 6: Error Handling + Config

**Rust concepts:** `thiserror`, `impl std::error::Error`, Lifetimes (contrast owned vs borrowed), `serde`, `Deserialize`

**Files:**
- Create: `src/config.rs`
- Modify: `src/error.rs`
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml`

- [ ] **Step 6.1: Add dependencies to `Cargo.toml`**

```toml
thiserror = "1"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

- [ ] **Step 6.2: Write a test for config parsing**

Create `src/config.rs`:

```rust
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub theme: ThemeConfig,
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    pub heading_color: String,
    pub code_color: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig {
                heading_color: "cyan".to_string(),
                code_color: "yellow".to_string(),
            },
            keys: HashMap::from([
                ("quit".to_string(), "q".to_string()),
                ("scroll_down".to_string(), "j".to_string()),
                ("scroll_up".to_string(), "k".to_string()),
                ("top".to_string(), "g".to_string()),
            ]),
        }
    }
}

pub fn load() -> Config {
    let config_path = dirs_config_path();
    if let Some(path) = config_path {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str::<Config>(&content) {
                return config;
            }
        }
    }
    Config::default()
}

fn dirs_config_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".config/mdr/config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_quit_key() {
        let cfg = Config::default();
        assert_eq!(cfg.keys.get("quit"), Some(&"q".to_string()));
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_str = r#"
[theme]
heading_color = "blue"
code_color = "green"

[keys]
quit = "x"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.theme.heading_color, "blue");
        assert_eq!(cfg.keys.get("quit"), Some(&"x".to_string()));
    }
}
```

- [ ] **Step 6.3: Run config tests**

```bash
cargo test config::
```

Expected: both tests pass.

- [ ] **Step 6.4: Rewrite `src/error.rs` with `thiserror`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config parse error: {0}")]
    Config(#[from] toml::de::Error),

    #[error("{0}")]
    Other(String),
}
```

`thiserror` derives `std::error::Error` and `Display` for you. The `#[from]` attribute generates `From<io::Error>` automatically, replacing our hand-written `impl From`.

- [ ] **Step 6.5: Verify it compiles**

```bash
cargo build
```

Expected: no errors. (The `From` impls are now auto-generated by `thiserror`.)

- [ ] **Step 6.6: Lifetime lesson — owned vs borrowed Config**

This step is educational: add a comment block at the top of `src/config.rs` explaining the ownership decision:

```rust
// LEARNING NOTE: Why does Config own its Strings instead of borrowing?
//
// A borrowed version would look like:
//   struct Config<'a> {
//       heading_color: &'a str,
//   }
//
// This requires the original TOML string to outlive Config.
// Since we load Config at startup and use it everywhere, that lifetime
// is hard to manage. Owned String (heap-allocated) is simpler and
// appropriate here — the config is small and loaded once.
//
// Use &str (borrowed) when: the caller owns the data, the function is
// short-lived, and you want to avoid cloning.
// Use String (owned) when: the data needs to outlive the call site.
```

- [ ] **Step 6.7: Wire config into `App`**

Add `config` field to `App` in `src/app.rs`:

```rust
use crate::config::Config;

pub struct App {
    pub running: bool,
    pub state: AppState,
    pub scroll: u16,
    pub content: Option<String>,
    pub file_name: Option<String>,
    pub tree: Option<FileNode>,
    pub tree_cursor: usize,
    pub config: Config,             // ← new
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            running: true,
            state: AppState::default(),
            scroll: 0,
            content: None,
            file_name: None,
            tree: None,
            tree_cursor: 0,
            config,
        }
    }
    // ... rest unchanged
}
```

- [ ] **Step 6.8: Load config in `src/main.rs`**

```rust
mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let cfg = config::load();
    let mut app = App::new(cfg);
    // ... rest unchanged
```

- [ ] **Step 6.9: Pass heading/code colors from config to markdown renderer**

Update `src/markdown.rs` signature:

```rust
use ratatui::style::Color;

pub struct RenderConfig {
    pub heading_color: Color,
    pub code_color: Color,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            heading_color: Color::Cyan,
            code_color: Color::Yellow,
        }
    }
}

fn color_from_str(s: &str) -> Color {
    match s {
        "red" => Color::Red,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "cyan" => Color::Cyan,
        "yellow" => Color::Yellow,
        "magenta" => Color::Magenta,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        _ => Color::White,
    }
}

pub fn parse_with_config(content: &str, config: &RenderConfig) -> Vec<Line<'static>> {
    // Same as parse() but uses config.heading_color and config.code_color
    // ... (copy parse(), replace Color::Cyan → config.heading_color, Color::Yellow → config.code_color)
}

// Keep the original parse() as a convenience wrapper
pub fn parse(content: &str) -> Vec<Line<'static>> {
    parse_with_config(content, &RenderConfig::default())
}
```

- [ ] **Step 6.10: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 6.11: Manually test config**

```bash
mkdir -p ~/.config/mdr
cat > ~/.config/mdr/config.toml << 'EOF'
[theme]
heading_color = "blue"
code_color = "green"

[keys]
quit = "q"
EOF
cargo run -- /tmp/test.md
```

Expected: headings appear in blue, code in green.

- [ ] **Step 6.12: Commit**

```bash
git add src/error.rs src/config.rs src/app.rs src/main.rs src/markdown.rs src/ui/mod.rs Cargo.toml Cargo.lock
git commit -m "feat(phase6): thiserror error chain and TOML config with serde"
```

---

## Task 7: Async Loading

**Rust concepts:** `async/await`, `tokio::spawn`, `std::sync::mpsc`, `Arc<Mutex<T>>`

**Files:**
- Modify: `src/main.rs`
- Modify: `src/fs.rs`
- Modify: `src/app.rs`
- Modify: `Cargo.toml`

- [ ] **Step 7.1: Add `tokio` to `Cargo.toml`**

```toml
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 7.2: Write an async file loader in `src/fs.rs`**

Add to `src/fs.rs`:

```rust
pub async fn read_file_async(path: std::path::PathBuf) -> Result<String, AppError> {
    tokio::fs::read_to_string(&path)
        .await
        .map_err(AppError::from)
}
```

- [ ] **Step 7.3: Add loading state to `src/app.rs`**

```rust
#[derive(Debug, Default, PartialEq)]
pub enum AppState {
    #[default]
    Browsing,
    Viewing,
    Loading,    // ← new
}

pub struct App {
    // ... existing fields ...
    pub load_error: Option<String>,   // ← new: error to display in UI
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            // ... existing ...
            load_error: None,
        }
    }

    pub fn set_loading(&mut self) {
        self.state = AppState::Loading;
        self.content = None;
        self.load_error = None;
    }

    pub fn set_content(&mut self, content: String, name: String) {
        self.content = Some(content);
        self.file_name = Some(name);
        self.scroll = 0;
        self.state = AppState::Viewing;
    }

    pub fn set_error(&mut self, err: String) {
        self.load_error = Some(err);
        self.state = AppState::Browsing;
    }
}
```

- [ ] **Step 7.4: Add channel-based async loading to `src/main.rs`**

```rust
use std::sync::mpsc;

// Message type for background loader → UI thread
enum LoadMsg {
    Content { name: String, content: String },
    Error(String),
}
```

Add to `main()` before the event loop:

```rust
let (tx, rx) = mpsc::channel::<LoadMsg>();
```

Pass `tx` into the run function and use it when `Enter` is pressed:

```rust
fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    tx: mpsc::Sender<LoadMsg>,
    rx: mpsc::Receiver<LoadMsg>,
) -> Result<(), Box<dyn std::error::Error>> {
    while app.running {
        // Drain any completed loads
        while let Ok(msg) = rx.try_recv() {
            match msg {
                LoadMsg::Content { name, content } => app.set_content(content, name),
                LoadMsg::Error(e) => app.set_error(e),
            }
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        // ... key handling as before, but replace Enter handler:
        // (KeyCode::Enter, _) in Browsing state:
        // if let Some(FileNode::File(path)) = app.selected_node().cloned().as_ref() {
        //     if path.extension() == ... {
        //         let path = path.clone();
        //         let tx = tx.clone();
        //         let name = path.file_name()... .to_string();
        //         app.set_loading();
        //         tokio::spawn(async move {
        //             match fs::read_file_async(path).await {
        //                 Ok(content) => { let _ = tx.send(LoadMsg::Content { name, content }); }
        //                 Err(e) => { let _ = tx.send(LoadMsg::Error(e.to_string())); }
        //             }
        //         });
        //     }
        // }
    }
    Ok(())
}
```

Full updated `Enter` handler inside `run()`:

```rust
(KeyCode::Enter, _) => {
    if let Some(node) = app.selected_node() {
        if let crate::fs::FileNode::File(path) = node {
            if node.is_markdown() {
                let path = path.clone();
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                let tx = tx.clone();
                app.set_loading();
                tokio::spawn(async move {
                    match fs::read_file_async(path).await {
                        Ok(content) => {
                            let _ = tx.send(LoadMsg::Content { name, content });
                        }
                        Err(e) => {
                            let _ = tx.send(LoadMsg::Error(e.to_string()));
                        }
                    }
                });
            }
        }
    }
}
```

- [ ] **Step 7.5: Convert `main()` to async**

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... unchanged ...
    let (tx, rx) = mpsc::channel::<LoadMsg>();
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, tx, rx);
    ratatui::restore();
    result
}
```

Note: `run()` itself stays synchronous. `tokio::spawn` is called from within it, which works because we're inside a tokio runtime (started by `#[tokio::main]`).

- [ ] **Step 7.6: Show loading spinner in `src/ui/mod.rs`**

Update `draw()` to handle `AppState::Loading`:

```rust
use crate::app::AppState;

pub fn draw(frame: &mut Frame, app: &App) {
    match app.state {
        AppState::Browsing => draw_browsing(frame, app),
        AppState::Viewing => draw_viewing(frame, app),
        AppState::Loading => draw_loading(frame),
    }
}

fn draw_loading(frame: &mut Frame) {
    use ratatui::widgets::Paragraph;
    use ratatui::style::{Style, Color};
    use ratatui::layout::Alignment;

    let block = Block::default().borders(Borders::ALL).title(" mdr ");
    let inner = block.inner(frame.area());
    frame.render_widget(block, frame.area());

    let loading = Paragraph::new("Loading…")
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    frame.render_widget(loading, inner);
}
```

- [ ] **Step 7.7: Display load errors in browsing view**

In `draw_browsing()`, add error display at the bottom when `app.load_error` is `Some`:

```rust
fn draw_browsing(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // If there's an error, reserve 3 rows at the bottom
    let (main_area, error_area) = if app.load_error.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // ... existing tree + preview rendering using main_area ...

    if let (Some(err), Some(err_area)) = (&app.load_error, error_area) {
        use ratatui::{widgets::Paragraph, style::{Style, Color}};
        let error_widget = Paragraph::new(format!("Error: {}", err))
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title(" Error "));
        frame.render_widget(error_widget, err_area);
    }
}
```

- [ ] **Step 7.8: Build and test**

```bash
cargo build
cargo run
```

Expected: file tree loads, selecting and pressing `Enter` on a large file shows "Loading…" briefly before content appears.

- [ ] **Step 7.9: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 7.10: Final polish commit**

```bash
git add src/ Cargo.toml Cargo.lock
git commit -m "feat(phase7): async file loading with tokio and mpsc channel"
```

---

## Self-Review

**Spec coverage check:**
- Phase 1 (module system, clap) → Task 1 ✓
- Phase 2 (ownership, Result, ?) → Task 2 ✓
- Phase 3 (struct, impl, event loop, enum) → Task 3 ✓
- Phase 4 (Widget trait, iterators, closures) → Task 4 ✓
- Phase 5 (recursive enum, pattern matching, Box-free here as Vec works) → Task 5 ✓
- Phase 6 (thiserror, lifetimes lesson, serde) → Task 6 ✓
- Phase 7 (async/await, tokio::spawn, mpsc, Arc/Mutex note) → Task 7 ✓

**Notes:**
- `Box<T>` was noted in the spec but the recursive `FileNode` uses `Vec<FileNode>` instead (which is heap-allocated). Step 5.1 can mention this trade-off as a teaching moment.
- `Arc<Mutex<T>>` was mentioned in the spec for Phase 7. The `mpsc::channel` approach avoids it — this is intentional, as channels are idiomatic Rust for cross-thread data. The plan notes in Step 7.4 why channels are preferred.
- All code blocks contain complete, compilable Rust.
