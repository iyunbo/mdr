use ratatui::{
    Frame,
    widgets::{Block, Borders},
};
use crate::app::{App, AppState};

pub fn draw(frame: &mut Frame, app: &App) {
    let title = match app.state {
        AppState::Browsing => " mdr - browse ",
        AppState::Viewing => " mdr - viewing ",
    };
    let block = Block::default()
         .borders(Borders::ALL)
         .title(title);
    frame.render_widget(block, frame.area());
}
