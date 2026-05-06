mod app;
mod config;
mod error;
mod fs;
mod markdown;
mod ui;

use app::{App, AppState};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
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
                app.file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(String::from);
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

        match app.state {
            AppState::Viewing => match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) => app.quit(),
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
                (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.scroll_down(),
                (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.scroll_up(),
                (KeyCode::Char('g'), _) => app.scroll_top(),
                (KeyCode::Esc, _) => {
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
                (KeyCode::Enter, _) => spawn_open_selected(app, &tx),
                _ => {}
            },
            AppState::Loading => match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) => app.quit(),
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
                _ => {}
            },
        }
    }
    Ok(())
}

fn spawn_open_selected(app: &mut App, tx: &mpsc::Sender<LoadMsg>) {
    let Some(node) = app.selected_node().cloned() else {
        return;
    };
    let fs::FileNode::File(path) = &node else {
        return;
    };
    if !node.is_markdown() {
        return;
    }
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
