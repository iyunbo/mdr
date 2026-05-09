use crate::markdown::{ImageRef, LinkRef};
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
    pub links: &'a [LinkRef],
    pub selected_link: Option<usize>,
    pub scroll: u16,
    pub title: &'a str,
    pub show_line_numbers: bool,
    pub line_number_color: Color,
}

pub struct RenderedPreview {
    pub body_area: Rect,
    pub gutter_width: u16,
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    params: PreviewParams<'_>,
    picker: Option<&Picker>,
    image_cache: &mut HashMap<PathBuf, StatefulProtocol>,
) -> RenderedPreview {
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

    let image_rows: Vec<bool> = compute_image_rows(params.lines.len(), params.images);

    let selected = params
        .selected_link
        .and_then(|i| params.links.get(i))
        .filter(|link| link.line < params.lines.len());

    let gutter_width: u16 = if params.show_line_numbers {
        let total = params.lines.len();
        let digits = total.to_string().len().max(2);
        let num_style = Style::default().fg(params.line_number_color);
        let numbered: Vec<Line<'_>> = params
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let inner = render_spans(i, line, selected);
                let mut spans: Vec<Span<'_>> = Vec::with_capacity(inner.len() + 1);
                let prefix = if image_rows.get(i).copied().unwrap_or(false) {
                    " ".repeat(digits + 1)
                } else {
                    format!("{:>width$} ", i + 1, width = digits)
                };
                spans.push(Span::styled(prefix, num_style));
                spans.extend(inner);
                Line::from(spans)
            })
            .collect();
        let paragraph = Paragraph::new(numbered).scroll((params.scroll, 0));
        frame.render_widget(paragraph, body_area);
        (digits + 1) as u16
    } else {
        let lines: Vec<Line<'_>> = params
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| Line::from(render_spans(i, line, selected)))
            .collect();
        let paragraph = Paragraph::new(lines).scroll((params.scroll, 0));
        frame.render_widget(paragraph, body_area);
        0
    };

    if let Some(picker) = picker
        && picker.protocol_type() == ratatui_image::picker::ProtocolType::Kitty
    {
        render_images(
            frame,
            body_area,
            params.images,
            params.scroll,
            picker,
            image_cache,
        );
    }

    RenderedPreview {
        body_area,
        gutter_width,
    }
}

/// Borrow each span's text instead of cloning the owned String inside
/// ratatui's `Cow::Owned(String)` — saves O(content) allocations per frame.
/// The selected link's line is the only one whose spans actually need to be
/// rebuilt (split + reverse-modifier).
fn render_spans<'a>(
    i: usize,
    line: &'a Line<'static>,
    selected: Option<&LinkRef>,
) -> Vec<Span<'a>> {
    match selected {
        Some(link) if link.line == i => highlight_range(line, link.col_start, link.col_end).spans,
        _ => line
            .spans
            .iter()
            .map(|s| Span::styled(s.content.as_ref(), s.style))
            .collect(),
    }
}

fn compute_image_rows(total_lines: usize, images: &[ImageRef]) -> Vec<bool> {
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
        let rect = Rect {
            x: body.x,
            y: visible_top as u16,
            width: body.width,
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
                evict_if_full(image_cache, &img.path);
                image_cache.entry(img.path.clone()).or_insert(loaded)
            }
        };
        frame.render_stateful_widget(StatefulImage::new(), rect, state);
    }
}

/// Apply `Modifier::REVERSED` to the character range `[col_start, col_end)` of
/// `line`, splitting spans as needed. Used to highlight the currently selected
/// link in the preview pane.
fn highlight_range(line: &Line<'static>, col_start: usize, col_end: usize) -> Line<'static> {
    if col_start >= col_end {
        return line.clone();
    }
    let mut out: Vec<Span<'static>> = Vec::with_capacity(line.spans.len() + 2);
    let mut col = 0usize;
    for span in &line.spans {
        let span_chars = span.content.chars().count();
        let span_start = col;
        let span_end = col + span_chars;
        col = span_end;
        if span_chars == 0 || span_end <= col_start || span_start >= col_end {
            out.push(span.clone());
            continue;
        }
        let chars: Vec<char> = span.content.chars().collect();
        let pre_len = col_start.saturating_sub(span_start);
        let mid_end = (col_end - span_start).min(span_chars);
        if pre_len > 0 {
            let s: String = chars[..pre_len].iter().collect();
            out.push(Span::styled(s, span.style));
        }
        let mid: String = chars[pre_len..mid_end].iter().collect();
        out.push(Span::styled(
            mid,
            span.style.add_modifier(Modifier::REVERSED),
        ));
        if mid_end < span_chars {
            let s: String = chars[mid_end..].iter().collect();
            out.push(Span::styled(s, span.style));
        }
    }
    Line::from(out)
}

/// Cap on decoded images held in memory across browsing sessions. Images
/// are dropped from the cache only when this is exceeded — drops are
/// arbitrary (HashMap iteration order), which is acceptable since the goal
/// is bounding memory, not optimizing reuse.
const IMAGE_CACHE_CAP: usize = 32;

fn evict_if_full(cache: &mut HashMap<PathBuf, StatefulProtocol>, keep: &PathBuf) {
    if cache.len() < IMAGE_CACHE_CAP {
        return;
    }
    let victim = cache.keys().find(|k| *k != keep).cloned();
    if let Some(k) = victim {
        cache.remove(&k);
    }
}

fn load_image(picker: &Picker, path: &PathBuf) -> Option<StatefulProtocol> {
    let img = image::ImageReader::open(path).ok()?.decode().ok()?;
    Some(picker.new_resize_protocol(img))
}
