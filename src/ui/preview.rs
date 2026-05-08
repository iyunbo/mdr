use crate::markdown::ImageRef;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct PreviewParams<'a> {
    pub lines: &'a [Line<'static>],
    pub images: &'a [ImageRef],
    pub scroll: u16,
    pub title: &'a str,
    pub show_line_numbers: bool,
    pub line_number_color: Color,
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    params: PreviewParams<'_>,
    picker: Option<&Picker>,
    image_cache: &mut HashMap<PathBuf, StatefulProtocol>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    let title_area = chunks[0];
    let body_area = chunks[1];

    let title = Paragraph::new(params.title).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(title, title_area);

    let image_rows: Vec<bool> =
        compute_image_rows(params.lines.len(), params.images, params.scroll, body_area);

    if params.show_line_numbers {
        let total = params.lines.len();
        let digits = total.to_string().len().max(2);
        let num_style = Style::default().fg(params.line_number_color);
        let numbered: Vec<Line<'static>> = params
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let mut spans: Vec<Span<'static>> = Vec::with_capacity(line.spans.len() + 1);
                let prefix = if image_rows.get(i).copied().unwrap_or(false) {
                    " ".repeat(digits + 1)
                } else {
                    format!("{:>width$} ", i + 1, width = digits)
                };
                spans.push(Span::styled(prefix, num_style));
                spans.extend(line.spans.iter().cloned());
                Line::from(spans)
            })
            .collect();
        let paragraph = Paragraph::new(numbered).scroll((params.scroll, 0));
        frame.render_widget(paragraph, body_area);
    } else {
        let paragraph = Paragraph::new(params.lines.to_vec()).scroll((params.scroll, 0));
        frame.render_widget(paragraph, body_area);
    }

    if let Some(picker) = picker
        && picker.protocol_type() == ratatui_image::picker::ProtocolType::Kitty
    {
        render_images(frame, body_area, params.images, params.scroll, picker, image_cache);
    }
}

fn compute_image_rows(total_lines: usize, images: &[ImageRef], _scroll: u16, _body: Rect) -> Vec<bool> {
    let mut v = vec![false; total_lines];
    for img in images {
        let start = img.line_offset.min(total_lines);
        let end = (start + img.height as usize).min(total_lines);
        for slot in v.iter_mut().take(end).skip(start) {
            *slot = true;
        }
    }
    v
}

fn render_images(
    frame: &mut Frame,
    body: Rect,
    images: &[ImageRef],
    scroll: u16,
    picker: &Picker,
    image_cache: &mut HashMap<PathBuf, StatefulProtocol>,
) {
    let scroll = scroll as i32;
    let body_top = body.y as i32;
    let body_bottom = (body.y + body.height) as i32;

    let line_offset_gutter = compute_gutter_width(images);

    for img in images {
        let on_screen_y = body_top + img.line_offset as i32 - scroll;
        let on_screen_bottom = on_screen_y + img.height as i32;
        if on_screen_bottom <= body_top || on_screen_y >= body_bottom {
            continue;
        }
        let visible_top = on_screen_y.max(body_top);
        let visible_bottom = on_screen_bottom.min(body_bottom);
        if visible_top >= visible_bottom {
            continue;
        }
        let x = body.x + line_offset_gutter;
        if x >= body.x + body.width {
            continue;
        }
        let rect = Rect {
            x,
            y: visible_top as u16,
            width: body.width.saturating_sub(line_offset_gutter),
            height: (visible_bottom - visible_top) as u16,
        };
        if rect.width == 0 || rect.height == 0 {
            continue;
        }

        let state = match image_cache.get_mut(&img.path) {
            Some(s) => s,
            None => {
                let Some(loaded) = load_image(picker, &img.path) else {
                    continue;
                };
                image_cache.insert(img.path.clone(), loaded);
                image_cache.get_mut(&img.path).expect("just inserted")
            }
        };
        frame.render_stateful_widget(StatefulImage::new(), rect, state);
    }
}

fn compute_gutter_width(_images: &[ImageRef]) -> u16 {
    0
}

fn load_image(picker: &Picker, path: &PathBuf) -> Option<StatefulProtocol> {
    let img = image::ImageReader::open(path).ok()?.decode().ok()?;
    Some(picker.new_resize_protocol(img))
}
