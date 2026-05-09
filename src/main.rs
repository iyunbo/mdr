mod app;
mod config;
mod error;
mod fs;
mod keys;
mod markdown;
mod ui;
mod wikilink;

use app::{App, AppState, NavDir, SearchDirection};
use clap::Parser;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use keys::Action;
use markdown::LinkTarget;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "mdr", version, about = "A terminal markdown reader")]
struct Cli {
    path: Option<PathBuf>,
}

enum LoadMsg {
    Content { path: PathBuf, content: String },
    Error(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let cfg = config::load();
    let mut app = App::new(cfg);

    match cli.path {
        Some(ref path) if path.is_file() => match fs::read_file(path.to_str().unwrap_or("")) {
            Ok(content) => {
                let base_dir = path.parent().map(|p| p.to_path_buf());
                app.set_content(Some(path.clone()), content, base_dir);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(());
            }
        },
        Some(ref path) if path.is_dir() => match fs::walk_dir(path) {
            Ok(tree) => {
                app.tree = Some(tree);
                app.state = AppState::Browsing;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(());
            }
        },
        Some(ref path) => {
            eprintln!("Path does not exist: {}", path.display());
            return Ok(());
        }
        None => {
            let cwd = std::env::current_dir()?;
            if let Ok(tree) = fs::walk_dir(&cwd) {
                app.tree = Some(tree);
            }
            app.state = AppState::Browsing;
        }
    }

    let (tx, rx) = mpsc::channel::<LoadMsg>();
    let mut terminal = ratatui::init();
    app.picker = ratatui_image::picker::Picker::from_query_stdio().ok();
    let mouse_enabled = app.config.ui.mouse;
    if mouse_enabled {
        let _ = execute!(stdout(), EnableMouseCapture);
    }
    let result = run(&mut terminal, &mut app, tx, rx);
    if mouse_enabled {
        let _ = execute!(stdout(), DisableMouseCapture);
    }
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    tx: mpsc::Sender<LoadMsg>,
    rx: mpsc::Receiver<LoadMsg>,
) -> Result<(), Box<dyn std::error::Error>> {
    while app.running {
        while let Ok(msg) = rx.try_recv() {
            match msg {
                LoadMsg::Content { path, content } => {
                    let base_dir = path.parent().map(|p| p.to_path_buf());
                    app.set_content(Some(path), content, base_dir);
                }
                LoadMsg::Error(e) => app.set_error(e),
            }
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }
        let term_size = terminal.size().unwrap_or_default();
        match event::read()? {
            Event::Key(key) => handle_key(app, key, &tx, term_size.height),
            Event::Mouse(m) => handle_mouse(app, m, &tx),
            _ => {}
        }
    }
    Ok(())
}

fn handle_mouse(app: &mut App, m: MouseEvent, tx: &mpsc::Sender<LoadMsg>) {
    if app.state == AppState::Loading {
        return;
    }
    match m.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(idx) = app.link_at_terminal(m.column, m.row) {
                app.selected_link = Some(idx);
                app.status_message = None;
                activate_link(app, tx);
            }
        }
        MouseEventKind::ScrollDown => {
            app.status_message = None;
            match app.state {
                AppState::Viewing => app.scroll_down(),
                AppState::Browsing => app.cursor_down(),
                AppState::Loading => unreachable!(),
            }
        }
        MouseEventKind::ScrollUp => {
            app.status_message = None;
            match app.state {
                AppState::Viewing => app.scroll_up(),
                AppState::Browsing => app.cursor_up(),
                AppState::Loading => unreachable!(),
            }
        }
        _ => {}
    }
}

fn handle_key(app: &mut App, key: KeyEvent, tx: &mpsc::Sender<LoadMsg>, term_height: u16) {
    // Search input mode captures all keys.
    if app.search_input.is_some() {
        match key.code {
            KeyCode::Esc => app.cancel_search(),
            KeyCode::Enter => app.confirm_search(),
            KeyCode::Backspace => app.search_input_pop(),
            KeyCode::Char(c) => app.search_input_push(c),
            _ => {}
        }
        return;
    }

    // `:N` line-jump prompt captures all keys.
    if app.line_prompt.is_some() {
        match key.code {
            KeyCode::Esc => app.cancel_line_prompt(),
            KeyCode::Enter => app.confirm_line_prompt(),
            KeyCode::Backspace => app.line_prompt_pop(),
            KeyCode::Char(c) => app.line_prompt_push(c),
            _ => {}
        }
        return;
    }

    // `gg` chord — first `g` is recorded as pending; second `g` jumps to top
    // (or to line N if a count prefix is buffered, e.g. `5gg`).
    if app.pending_g {
        app.pending_g = false;
        if matches!(key.code, KeyCode::Char('g')) && key.modifiers == KeyModifiers::NONE {
            let count = app.take_count() as usize;
            app.status_message = None;
            if count > 1 {
                app.goto_line(count);
            } else {
                let half_page = ((term_height.saturating_sub(2) / 2) as usize).max(1);
                let page = (term_height.saturating_sub(2) as usize).max(1);
                dispatch(app, Action::Top, tx, 1, half_page, page);
            }
            return;
        }
        // Non-`g` key after a pending `g` cancels the chord; fall through and
        // process the key normally below.
    }

    // Number prefix: digits accumulate into the count buffer (no modifiers).
    if key.modifiers == KeyModifiers::NONE
        && let KeyCode::Char(c) = key.code
        && app.push_count_digit(c)
    {
        return;
    }

    // First half of the `gg` chord — only when no other binding owns `g`.
    if matches!(key.code, KeyCode::Char('g')) && key.modifiers == KeyModifiers::NONE {
        let combo = keys::normalize_combo(key.code, key.modifiers);
        if !app.keymap.contains_key(&combo) {
            app.pending_g = true;
            return;
        }
    }

    let count = app.take_count() as usize;

    if let Some(&action) = app
        .keymap
        .get(&keys::normalize_combo(key.code, key.modifiers))
    {
        // Clear ephemeral status the moment a real action fires.
        app.status_message = None;
        let half_page = ((term_height.saturating_sub(2) / 2) as usize).max(1);
        let page = (term_height.saturating_sub(2) as usize).max(1);
        dispatch(app, action, tx, count, half_page, page);
        return;
    }

    // Hardcoded fallback: Ctrl+C always quits, even if not in keymap.
    if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.quit();
    }
}

fn dispatch(
    app: &mut App,
    action: Action,
    tx: &mpsc::Sender<LoadMsg>,
    count: usize,
    half_page: usize,
    page: usize,
) {
    let count = count.max(1);
    match (app.state, action) {
        (_, Action::Quit) => app.quit(),
        (AppState::Loading, _) => {}
        (AppState::Viewing, Action::Down) => repeat(count, || app.scroll_down()),
        (AppState::Viewing, Action::Up) => repeat(count, || app.scroll_up()),
        (AppState::Viewing, Action::Top) => app.scroll_top(),
        (_, Action::Bottom) => app.goto_bottom(page),
        (_, Action::HalfPageDown) => app.half_page_down(half_page * count),
        (_, Action::HalfPageUp) => app.half_page_up(half_page * count),
        (_, Action::PageDown) => app.half_page_down(page * count),
        (_, Action::PageUp) => app.half_page_up(page * count),
        (AppState::Viewing, Action::Back) => {
            if app.tree.is_some() {
                app.state = AppState::Browsing;
            }
        }
        (AppState::Viewing, Action::Activate) => activate_link(app, tx),
        (AppState::Browsing, Action::Down) => repeat(count, || app.cursor_down()),
        (AppState::Browsing, Action::Up) => repeat(count, || app.cursor_up()),
        (AppState::Browsing, Action::Top) => app.cursor_top(),
        (AppState::Browsing, Action::Activate) => activate_selected(app, tx),
        (AppState::Browsing, Action::Back) => app.collapse_selected(),
        (_, Action::SearchForward) => app.start_search(SearchDirection::Forward),
        (_, Action::SearchBackward) => app.start_search(SearchDirection::Backward),
        (_, Action::RepeatNext) => app.repeat_search(false),
        (_, Action::RepeatPrev) => app.repeat_search(true),
        (_, Action::LineJumpPrompt) => app.start_line_prompt(),
        (AppState::Viewing, Action::NextLink) => app.cycle_link(1, page),
        (AppState::Viewing, Action::PrevLink) => app.cycle_link(-1, page),
        (_, Action::NextLink) | (_, Action::PrevLink) => {}
        (_, Action::NavBack) => nav_step(app, tx, NavDir::Back),
        (_, Action::NavForward) => nav_step(app, tx, NavDir::Forward),
    }
}

fn nav_step(app: &mut App, tx: &mpsc::Sender<LoadMsg>, dir: NavDir) {
    match app.nav_step(dir) {
        Some((path, scroll)) => {
            app.suppress_history_push = true;
            app.pending_scroll = Some(scroll);
            spawn_load_path(app, tx, path);
        }
        None => {
            let msg = match dir {
                NavDir::Back => "Already at oldest file",
                NavDir::Forward => "Already at newest file",
            };
            app.status_message = Some(msg.to_string());
        }
    }
}

fn repeat<F: FnMut()>(count: usize, mut f: F) {
    for _ in 0..count {
        f();
    }
}

fn activate_selected(app: &mut App, tx: &mpsc::Sender<LoadMsg>) {
    let Some(node) = app.selected_node() else {
        return;
    };
    match node {
        fs::FileNode::Dir { .. } => {
            if let Err(e) = app.toggle_selected() {
                app.load_error = Some(e.to_string());
            }
        }
        fs::FileNode::File(_) if node.is_markdown() => {
            spawn_load(app, tx);
        }
        _ => {}
    }
}

fn activate_link(app: &mut App, tx: &mpsc::Sender<LoadMsg>) {
    let Some(link) = app.current_link() else {
        return;
    };
    match link.target {
        LinkTarget::Local(path) => {
            let is_md = matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("md") | Some("markdown")
            );
            if !is_md {
                app.status_message = Some(format!("Not a markdown file: {}", path.display()));
                return;
            }
            // Existence is verified by the async read; failure surfaces via
            // LoadMsg::Error → status bar.
            spawn_load_path(app, tx, path);
        }
        LinkTarget::Url(url) => match open_url(&url) {
            Ok(()) => app.status_message = Some(format!("Opening: {}", url)),
            Err(e) => app.status_message = Some(format!("Open failed: {}", e)),
        },
    }
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = "xdg-open";
    std::process::Command::new(cmd)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}

fn spawn_load_path(app: &mut App, tx: &mpsc::Sender<LoadMsg>, path: PathBuf) {
    let tx = tx.clone();
    let path_for_msg = path.clone();
    app.set_loading();
    tokio::spawn(async move {
        let msg = match fs::read_file_async(path).await {
            Ok(content) => LoadMsg::Content {
                path: path_for_msg,
                content,
            },
            Err(e) => LoadMsg::Error(e.to_string()),
        };
        let _ = tx.send(msg);
    });
}

fn spawn_load(app: &mut App, tx: &mpsc::Sender<LoadMsg>) {
    let Some(node) = app.selected_node().cloned() else {
        return;
    };
    let fs::FileNode::File(path) = &node else {
        return;
    };
    let path = path.clone();
    spawn_load_path(app, tx, path);
}
