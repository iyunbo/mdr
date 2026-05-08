mod app;
mod config;
mod error;
mod fs;
mod keys;
mod markdown;
mod ui;

use app::{App, AppState, SearchDirection};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use keys::Action;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "mdr", version, about = "A terminal markdown reader")]
struct Cli {
    path: Option<PathBuf>,
}

enum LoadMsg {
    Content { name: String, content: String },
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
                app.file_name = path.file_name().and_then(|n| n.to_str()).map(String::from);
                app.content = Some(content);
                app.state = AppState::Viewing;
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
    let result = run(&mut terminal, &mut app, tx, rx);
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
                LoadMsg::Content { name, content } => app.set_content(content, name),
                LoadMsg::Error(e) => app.set_error(e),
            }
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };

        let term_size = terminal.size().unwrap_or_default();
        handle_key(app, key, &tx, term_size.height);
    }
    Ok(())
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

    // Number prefix: digits accumulate into the count buffer (no modifiers).
    if key.modifiers == KeyModifiers::NONE
        && let KeyCode::Char(c) = key.code
        && app.push_count_digit(c)
    {
        return;
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
        (_, Action::Top) if count > 1 => app.goto_line(count),
        (AppState::Viewing, Action::Top) => app.scroll_top(),
        (_, Action::Bottom) => app.goto_bottom(page),
        (_, Action::HalfPageDown) => app.half_page_down(half_page * count),
        (_, Action::HalfPageUp) => app.half_page_up(half_page * count),
        (AppState::Viewing, Action::Back) => {
            if app.tree.is_some() {
                app.state = AppState::Browsing;
            }
        }
        (AppState::Viewing, Action::Activate) => {}
        (AppState::Browsing, Action::Down) => repeat(count, || app.cursor_down()),
        (AppState::Browsing, Action::Up) => repeat(count, || app.cursor_up()),
        (AppState::Browsing, Action::Top) => app.cursor_top(),
        (AppState::Browsing, Action::Activate) => activate_selected(app, tx),
        (AppState::Browsing, Action::Back) => app.collapse_selected(),
        (_, Action::SearchForward) => app.start_search(SearchDirection::Forward),
        (_, Action::SearchBackward) => app.start_search(SearchDirection::Backward),
        (_, Action::RepeatNext) => app.repeat_search(false),
        (_, Action::RepeatPrev) => app.repeat_search(true),
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

fn spawn_load(app: &mut App, tx: &mpsc::Sender<LoadMsg>) {
    let Some(node) = app.selected_node().cloned() else {
        return;
    };
    let fs::FileNode::File(path) = &node else {
        return;
    };
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string();
    let path = path.clone();
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
