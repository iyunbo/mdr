mod app;
mod error;
mod fs;
mod markdown;
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
                (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.scroll_down(),
                (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.scroll_up(),
                (KeyCode::Char('g'), _) => app.scroll_top(),
                _ => {}
            }
        }
    }
    Ok(())
}
