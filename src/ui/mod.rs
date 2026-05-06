pub mod file_tree;
pub mod preview;

use crate::app::{App, AppState};
use crate::markdown;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, app: &App) {
    match app.state {
        AppState::Browsing => draw_browsing(frame, app),
        AppState::Viewing => draw_viewing(frame, app),
        AppState::Loading => draw_loading(frame),
    }
}

fn draw_browsing(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let (main_area, error_area) = split_for_error(area, app.load_error.is_some());

    if let Some(tree) = &app.tree {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(main_area);

        let tree_widget = file_tree::FileTreeWidget {
            node: tree,
            cursor: app.tree_cursor,
        };
        frame.render_widget(tree_widget, chunks[0]);

        if let Some(content) = &app.content {
            let lines = markdown::parse_with_config(content, &render_config(app));
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
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" mdr - no directory loaded ");
        frame.render_widget(block, main_area);
    }

    if let (Some(err), Some(err_area)) = (&app.load_error, error_area) {
        let error_widget = Paragraph::new(format!("Error: {}", err))
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title(" Error "));
        frame.render_widget(error_widget, err_area);
    }
}

fn draw_viewing(frame: &mut Frame, app: &App) {
    let content = app.content.as_deref().unwrap_or("");
    let lines = markdown::parse_with_config(content, &render_config(app));
    let widget = preview::PreviewWidget {
        lines: &lines,
        scroll: app.scroll,
        title: app.file_name.as_deref().unwrap_or("untitled"),
    };
    frame.render_widget(widget, frame.area());
}

fn draw_loading(frame: &mut Frame) {
    let block = Block::default().borders(Borders::ALL).title(" mdr ");
    let inner = block.inner(frame.area());
    frame.render_widget(block, frame.area());

    let loading = Paragraph::new("Loading...")
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    frame.render_widget(loading, inner);
}

fn split_for_error(area: Rect, has_error: bool) -> (Rect, Option<Rect>) {
    if !has_error {
        return (area, None);
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);
    (chunks[0], Some(chunks[1]))
}

fn render_config(app: &App) -> markdown::RenderConfig {
    markdown::RenderConfig {
        heading_color: markdown::color_from_str(&app.config.theme.heading_color),
        code_color: markdown::color_from_str(&app.config.theme.code_color),
    }
}
