pub mod preview;

use ratatui::{
    Frame,
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

fn draw_browsing(frame: &mut Frame, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" mdr - browse (coming in Phase 5) ");
    frame.render_widget(block, frame.area());
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
