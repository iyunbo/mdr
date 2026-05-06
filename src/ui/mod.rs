pub mod file_tree;
pub mod preview;

use crate::app::{App, AppState};
use crate::markdown;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
};

pub fn draw(frame: &mut Frame, app: &App) {
    match app.state {
        AppState::Browsing => draw_browsing(frame, app),
        AppState::Viewing => draw_viewing(frame, app),
    }
}

fn draw_browsing(frame: &mut Frame, app: &App) {
    let Some(tree) = &app.tree else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" mdr - no directory loaded ");
        frame.render_widget(block, frame.area());
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.area());

    let tree_widget = file_tree::FileTreeWidget {
        node: tree,
        cursor: app.tree_cursor,
    };
    frame.render_widget(tree_widget, chunks[0]);

    if let Some(content) = &app.content {
        let lines = markdown::parse(content);
        let widget = preview::PreviewWidget {
            lines: &lines,
            scroll: app.scroll,
            title: app.file_name.as_deref().unwrap_or(""),
        };
        frame.render_widget(widget, chunks[1]);
    } else {
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .title(" Preview ");
        frame.render_widget(preview_block, chunks[1]);
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
