pub mod file_tree;
pub mod preview;

use crate::app::{App, AppState, SearchDirection};
use crate::markdown::color_from_str;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let (main_area, status_area, error_area) = split_layout(area, app);

    match app.state {
        AppState::Browsing => draw_browsing(frame, main_area, app),
        AppState::Viewing => draw_viewing(frame, main_area, app),
        AppState::Loading => draw_loading(frame, main_area),
    }

    if let Some(area) = status_area {
        draw_status_bar(frame, area, app);
    }
    if let (Some(area), Some(err)) = (error_area, app.load_error.as_ref()) {
        let widget =
            Paragraph::new(format!("Error: {}", err)).style(Style::default().fg(Color::Red));
        frame.render_widget(widget, area);
    }
}

fn draw_browsing(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.tree.is_none() {
        let widget = Paragraph::new("No directory loaded")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(widget, area);
        app.last_preview_body = None;
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // Tree pane: borrow `&app.tree` directly. The borrow ends before the
    // `&mut app` for the preview pane below — no need to clone the tree.
    file_tree::render(
        frame,
        chunks[0],
        app.tree.as_ref().expect("checked above"),
        app.tree_cursor,
    );

    if app.content.is_some() {
        render_preview(frame, chunks[1], app);
    } else {
        app.last_preview_body = None;
    }
}

fn draw_viewing(frame: &mut Frame, area: Rect, app: &mut App) {
    render_preview(frame, area, app);
}

fn render_preview(frame: &mut Frame, area: Rect, app: &mut App) {
    let result = app.parse_current();
    let title = app
        .file_name
        .clone()
        .unwrap_or_else(|| "untitled".to_string());

    let params = preview::PreviewParams {
        lines: &result.lines,
        images: &result.images,
        links: &result.links,
        selected_link: app.selected_link,
        scroll: app.scroll,
        title: &title,
        show_line_numbers: app.config.theme.show_line_numbers,
        line_number_color: color_from_str(&app.config.theme.line_number_color),
    };
    let rendered = preview::render(
        frame,
        area,
        params,
        app.picker.as_ref(),
        &mut app.image_cache,
    );
    app.last_preview_body = Some(rendered.body_area);
    app.last_gutter_width = rendered.gutter_width;
}

fn draw_loading(frame: &mut Frame, area: Rect) {
    let widget = Paragraph::new("Loading...")
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    frame.render_widget(widget, area);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(input) = &app.search_input {
        let prefix = match input.direction {
            SearchDirection::Forward => '/',
            SearchDirection::Backward => '?',
        };
        let text = format!("{}{}", prefix, input.buffer);
        let widget = Paragraph::new(text).style(Style::default().fg(Color::Yellow));
        frame.render_widget(widget, area);
        return;
    }
    if let Some(prompt) = &app.line_prompt {
        let text = format!(":{}", prompt.buffer);
        let widget = Paragraph::new(text).style(Style::default().fg(Color::Yellow));
        frame.render_widget(widget, area);
        return;
    }
    if let Some(msg) = &app.status_message {
        let widget = Paragraph::new(msg.clone()).style(Style::default().fg(Color::Red));
        frame.render_widget(widget, area);
        return;
    }
    if !app.count_buffer.is_empty() {
        let widget = Paragraph::new(app.count_buffer.clone())
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Right);
        frame.render_widget(widget, area);
    }
}

fn split_layout(area: Rect, app: &App) -> (Rect, Option<Rect>, Option<Rect>) {
    let want_status = app.search_input.is_some()
        || app.line_prompt.is_some()
        || app.status_message.is_some()
        || !app.count_buffer.is_empty();
    let want_error = app.load_error.is_some();

    let mut constraints: Vec<Constraint> = vec![Constraint::Min(0)];
    if want_status {
        constraints.push(Constraint::Length(1));
    }
    if want_error {
        constraints.push(Constraint::Length(1));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let main = chunks[0];
    let mut idx = 1;
    let status = if want_status {
        let r = Some(chunks[idx]);
        idx += 1;
        r
    } else {
        None
    };
    let error = if want_error { Some(chunks[idx]) } else { None };
    (main, status, error)
}
