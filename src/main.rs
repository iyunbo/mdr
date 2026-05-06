mod app;
mod error;
mod fs;
mod markdown;
mod ui;

use app::{App, AppState};
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

    match cli.path {
        Some(ref path) if path.is_file() => {
            match fs::read_file(path.to_str().unwrap_or("")) {
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
            }
        }
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
                    (KeyCode::Enter, _) => open_selected(app),
                    _ => {}
                },
            }
        }
    }
    Ok(())
}

fn open_selected(app: &mut App) {
    let Some(node) = app.selected_node().cloned() else {
        return;
    };
    let fs::FileNode::File(path) = &node else {
        return;
    };
    if !node.is_markdown() {
        return;
    }
    if let Ok(content) = fs::read_file(path.to_str().unwrap_or("")) {
        app.file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from);
        app.content = Some(content);
        app.scroll = 0;
        app.state = AppState::Viewing;
    }
}
