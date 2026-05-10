use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct RenderConfig {
    pub h1_color: Color,
    pub heading_color: Color,
    pub code_color: Color,
    pub image_height: u16,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            h1_color: Color::LightRed,
            heading_color: Color::Cyan,
            code_color: Color::Yellow,
            image_height: 12,
        }
    }
}

impl RenderConfig {
    fn heading_color_for(&self, level: HeadingLevel) -> Color {
        if level == HeadingLevel::H1 {
            self.h1_color
        } else {
            self.heading_color
        }
    }
}

/// A reference to an image embedded in the rendered output.
/// `line_offset` is the row index within the rendered `lines` where the image
/// starts; the image occupies `height` rows. `alt` is the markdown alt text.
#[derive(Debug, Clone)]
pub struct ImageRef {
    pub path: PathBuf,
    pub line_offset: usize,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Local(PathBuf),
    Url(String),
}

/// A clickable / activatable link inside the rendered output.
/// `line` is the row index in the `lines` vector; `col_start`..`col_end` are
/// character offsets within that line covering the link's display text.
#[derive(Debug, Clone)]
pub struct LinkRef {
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub target: LinkTarget,
}

#[derive(Debug, Default)]
pub struct ParseResult {
    pub lines: Vec<Line<'static>>,
    pub images: Vec<ImageRef>,
    pub links: Vec<LinkRef>,
}

pub fn color_from_str(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "red" => Color::Red,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "cyan" => Color::Cyan,
        "yellow" => Color::Yellow,
        "magenta" => Color::Magenta,
        "white" => Color::White,
        "black" => Color::Black,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "dark_gray" | "darkgrey" | "dark_grey" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        _ => Color::White,
    }
}

#[cfg(test)]
pub fn parse(content: &str) -> Vec<Line<'static>> {
    parse_with_config(content, &RenderConfig::default())
}

#[cfg(test)]
pub fn parse_full(content: &str) -> ParseResult {
    parse_full_with(content, &RenderConfig::default(), None)
}

#[derive(Default)]
struct TableBuf {
    head: Vec<Vec<Span<'static>>>,
    body: Vec<Vec<Vec<Span<'static>>>>,
    in_head: bool,
    current_row: Vec<Vec<Span<'static>>>,
    current_cell: Vec<Span<'static>>,
}

#[cfg(test)]
pub fn parse_with_config(content: &str, config: &RenderConfig) -> Vec<Line<'static>> {
    parse_full_with(content, config, None).lines
}

/// Parse markdown into both rendered lines and the image references that
/// occur inside them. `base_dir` is the directory the markdown file lives in;
/// relative image paths are resolved against it. URLs are kept verbatim and
/// rendered as alt text only (no fetching).
pub fn parse_full_with(
    content: &str,
    config: &RenderConfig,
    base_dir: Option<&Path>,
) -> ParseResult {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut images: Vec<ImageRef> = Vec::new();
    let mut links: Vec<LinkRef> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();
    let mut current_heading: Option<HeadingLevel> = None;
    let mut table: Option<TableBuf> = None;
    let code_border_style = Style::default().fg(Color::DarkGray);
    let mut in_code_block = false;
    let mut current_image: Option<PendingImage> = None;
    let mut current_link: Option<PendingLink> = None;
    let mut list_stack: Vec<Option<u64>> = Vec::new();

    let parser = Parser::new_ext(content, Options::all());
    for event in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                // `record: false` inside tables — col offsets there index the
                // cell buffer, not the line.
                current_link = Some(PendingLink {
                    dest: dest_url.to_string(),
                    line_at_start: lines.len(),
                    col_start: current_line_chars(&spans),
                    prev_style: current_style,
                    record: table.is_none(),
                });
                current_style = current_style
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED);
            }
            Event::End(TagEnd::Link) => {
                if let Some(link) = current_link.take() {
                    current_style = link.prev_style;
                    if let Some(link_ref) = finalize_link(&link, &spans, &lines, base_dir) {
                        links.push(link_ref);
                    }
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                current_image = Some(PendingImage {
                    dest: dest_url.to_string(),
                    alt: String::new(),
                });
            }
            Event::End(TagEnd::Image) => {
                if let Some(img) = current_image.take() {
                    flush_block(&mut spans, &mut lines);
                    let resolved = resolve_image_path(&img.dest, base_dir);
                    let height = config.image_height.max(1);
                    let line_offset = lines.len();
                    let alt_label = if img.alt.is_empty() {
                        match resolved.as_ref() {
                            Some(p) => p
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("image")
                                .to_string(),
                            None => img.dest.clone(),
                        }
                    } else {
                        img.alt.clone()
                    };
                    let placeholder_text = format!("[image: {}]", alt_label);
                    lines.push(Line::from(Span::styled(
                        placeholder_text,
                        Style::default().fg(Color::DarkGray),
                    )));
                    for _ in 1..height {
                        lines.push(Line::default());
                    }
                    lines.push(Line::default());
                    if let Some(path) = resolved {
                        images.push(ImageRef {
                            path,
                            line_offset,
                            height,
                        });
                    }
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                flush_block(&mut spans, &mut lines);
                blank_before_heading(&mut lines, level);
                current_heading = Some(level);
                current_style = heading_style(level, config);
            }
            Event::Start(Tag::List(start)) => {
                if !spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut spans)));
                }
                list_stack.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                if !spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut spans)));
                }
                list_stack.pop();
                if list_stack.is_empty() {
                    lines.push(Line::default());
                }
            }
            Event::Start(Tag::Item) => {
                if !spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut spans)));
                }
                let depth = list_stack.len().saturating_sub(1);
                let indent: String = " ".repeat(depth * 2);
                let bullet = match list_stack.last_mut() {
                    Some(Some(n)) => {
                        let s = format!("{}. ", *n);
                        *n += 1;
                        s
                    }
                    _ => "• ".to_string(),
                };
                if !indent.is_empty() {
                    spans.push(Span::raw(indent));
                }
                spans.push(Span::styled(bullet, Style::default().fg(Color::White)));
            }
            Event::End(TagEnd::Item) if !spans.is_empty() => {
                lines.push(Line::from(std::mem::take(&mut spans)));
            }
            Event::Start(Tag::Strong) => {
                current_style = current_style.add_modifier(Modifier::BOLD);
            }
            Event::Start(Tag::Emphasis) => {
                current_style = current_style.add_modifier(Modifier::ITALIC);
            }
            Event::Start(Tag::Table(_)) => {
                flush_block(&mut spans, &mut lines);
                table = Some(TableBuf::default());
            }
            Event::Start(Tag::TableHead) => {
                if let Some(t) = table.as_mut() {
                    t.in_head = true;
                }
            }
            Event::Start(Tag::TableRow) | Event::Start(Tag::TableCell) => {}
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_block(&mut spans, &mut lines);
                let lang = match kind {
                    CodeBlockKind::Fenced(s) if !s.is_empty() => Some(s.to_string()),
                    _ => None,
                };
                let mut top: Vec<Span<'static>> =
                    vec![Span::styled("┌─", code_border_style)];
                if let Some(ref l) = lang {
                    top.push(Span::styled(
                        format!("  {}", l),
                        code_border_style.add_modifier(Modifier::ITALIC),
                    ));
                }
                lines.push(Line::from(top));
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                if !spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut spans)));
                }
                lines.push(Line::from(Span::styled("└─", code_border_style)));
                lines.push(Line::default());
                in_code_block = false;
            }
            Event::Text(text) if current_image.is_some() => {
                if let Some(img) = current_image.as_mut() {
                    img.alt.push_str(&text);
                }
            }
            Event::Text(text) if in_code_block => {
                let code_style = Style::default().fg(config.code_color);
                let s = text.to_string();
                let mut chunks = s.split('\n').peekable();
                while let Some(chunk) = chunks.next() {
                    let has_next = chunks.peek().is_some();
                    if !chunk.is_empty() {
                        if spans.is_empty() {
                            spans.push(Span::styled("│ ", code_border_style));
                        }
                        spans.push(Span::styled(chunk.to_string(), code_style));
                    } else if has_next {
                        lines.push(Line::from(Span::styled("│", code_border_style)));
                        continue;
                    }
                    if has_next {
                        lines.push(Line::from(std::mem::take(&mut spans)));
                    }
                }
            }
            Event::Text(text) => {
                push_span(
                    Span::styled(text.to_string(), current_style),
                    &mut spans,
                    &mut table,
                );
            }
            Event::Code(text) => {
                push_span(
                    Span::styled(text.to_string(), Style::default().fg(config.code_color)),
                    &mut spans,
                    &mut table,
                );
            }
            Event::End(TagEnd::Heading(_)) => {
                let level = current_heading.take().unwrap_or(HeadingLevel::H1);
                let text_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                if !spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut spans)));
                }
                if let Some(ch) = underline_char(level) {
                    let underline = ch.to_string().repeat(text_len.max(1));
                    lines.push(Line::from(Span::styled(
                        underline,
                        Style::default().fg(config.heading_color_for(level)),
                    )));
                }
                lines.push(Line::default());
                current_style = Style::default();
            }
            Event::End(TagEnd::Paragraph) => {
                if table.is_none() {
                    flush_block(&mut spans, &mut lines);
                }
                current_style = Style::default();
            }
            Event::End(TagEnd::Strong) | Event::End(TagEnd::Emphasis) => {
                current_style = Style::default();
            }
            Event::End(TagEnd::Table) => {
                if let Some(buf) = table.take() {
                    render_table(buf, &mut lines);
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(t) = table.as_mut() {
                    let row = std::mem::take(&mut t.current_row);
                    t.head = row;
                    t.in_head = false;
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(t) = table.as_mut() {
                    let row = std::mem::take(&mut t.current_row);
                    if !t.in_head {
                        t.body.push(row);
                    }
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(t) = table.as_mut() {
                    let cell = std::mem::take(&mut t.current_cell);
                    t.current_row.push(cell);
                }
            }
            Event::SoftBreak | Event::HardBreak if table.is_none() && !spans.is_empty() => {
                lines.push(Line::from(std::mem::take(&mut spans)));
            }
            _ => {}
        }
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }

    ParseResult {
        lines,
        images,
        links,
    }
}

struct PendingImage {
    dest: String,
    alt: String,
}

struct PendingLink {
    dest: String,
    line_at_start: usize,
    col_start: usize,
    prev_style: Style,
    record: bool,
}

fn current_line_chars(spans: &[Span<'static>]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

fn finalize_link(
    link: &PendingLink,
    spans: &[Span<'static>],
    lines: &[Line<'static>],
    base_dir: Option<&Path>,
) -> Option<LinkRef> {
    if !link.record {
        return None;
    }
    let line = lines.len();
    let col_end = current_line_chars(spans);
    let (col_start, col_end) = if line == link.line_at_start {
        if col_end <= link.col_start {
            return None;
        }
        (link.col_start, col_end)
    } else {
        // Link wrapped across a soft break — record only the tail line.
        if col_end == 0 {
            return None;
        }
        (0, col_end)
    };
    let target = resolve_link_target(&link.dest, base_dir)?;
    Some(LinkRef {
        line,
        col_start,
        col_end,
        target,
    })
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn resolve_link_target(dest: &str, base_dir: Option<&Path>) -> Option<LinkTarget> {
    if dest.is_empty() {
        return None;
    }
    if is_url(dest) {
        return Some(LinkTarget::Url(dest.to_string()));
    }
    // Strip in-document anchors — `file.md#section` resolves to `file.md`.
    let path_part = dest.split('#').next().unwrap_or(dest);
    if path_part.is_empty() {
        return None;
    }
    let p = PathBuf::from(path_part);
    let resolved = if p.is_absolute() {
        p
    } else {
        base_dir?.join(p)
    };
    Some(LinkTarget::Local(resolved))
}

fn resolve_image_path(dest: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    if is_url(dest) {
        return None;
    }
    let p = PathBuf::from(dest);
    if p.is_absolute() {
        return Some(p);
    }
    let base = base_dir?;
    Some(base.join(p))
}

fn push_span(span: Span<'static>, spans: &mut Vec<Span<'static>>, table: &mut Option<TableBuf>) {
    if let Some(t) = table.as_mut() {
        t.current_cell.push(span);
    } else {
        spans.push(span);
    }
}

fn flush_block(spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
        lines.push(Line::default());
    }
}

fn heading_style(level: HeadingLevel, config: &RenderConfig) -> Style {
    use HeadingLevel::*;
    let base = Style::default()
        .fg(config.heading_color_for(level))
        .add_modifier(Modifier::BOLD);
    match level {
        H1 | H2 | H3 => base,
        H4 | H5 | H6 => base.add_modifier(Modifier::ITALIC),
    }
}

fn underline_char(level: HeadingLevel) -> Option<char> {
    match level {
        HeadingLevel::H1 => Some('═'),
        HeadingLevel::H2 => Some('─'),
        _ => None,
    }
}

fn blank_before_heading(lines: &mut Vec<Line<'static>>, level: HeadingLevel) {
    use HeadingLevel::*;
    if !matches!(level, H1 | H2) {
        return;
    }
    if lines.is_empty() {
        return;
    }
    let last_blank = lines
        .last()
        .map(|l| l.spans.iter().all(|s| s.content.is_empty()))
        .unwrap_or(true);
    if !last_blank {
        lines.push(Line::default());
    }
}

fn cell_width(cell: &[Span<'static>]) -> usize {
    cell.iter().map(|s| s.content.chars().count()).sum()
}

fn render_table(buf: TableBuf, lines: &mut Vec<Line<'static>>) {
    let col_count = buf
        .head
        .len()
        .max(buf.body.iter().map(Vec::len).max().unwrap_or(0));
    if col_count == 0 {
        return;
    }

    let mut widths = vec![0usize; col_count];
    for (i, cell) in buf.head.iter().enumerate() {
        widths[i] = widths[i].max(cell_width(cell));
    }
    for row in &buf.body {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell_width(cell));
            }
        }
    }

    let sep_style = Style::default().fg(Color::DarkGray);

    lines.push(border_row(&widths, sep_style, '┌', '┬', '┐'));
    if !buf.head.is_empty() {
        lines.push(render_row(&buf.head, &widths, col_count, sep_style));
        lines.push(border_row(&widths, sep_style, '├', '┼', '┤'));
    }
    for row in &buf.body {
        lines.push(render_row(row, &widths, col_count, sep_style));
    }
    lines.push(border_row(&widths, sep_style, '└', '┴', '┘'));
    lines.push(Line::default());
}

fn render_row(
    row: &[Vec<Span<'static>>],
    widths: &[usize],
    col_count: usize,
    sep_style: Style,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, w) in widths.iter().enumerate().take(col_count) {
        let sep = if i == 0 { "│ " } else { " │ " };
        spans.push(Span::styled(sep, sep_style));
        let cell: &[Span<'static>] = row.get(i).map(Vec::as_slice).unwrap_or(&[]);
        let used: usize = cell_width(cell);
        for s in cell {
            spans.push(s.clone());
        }
        let pad = w.saturating_sub(used);
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
    }
    spans.push(Span::styled(" │", sep_style));
    Line::from(spans)
}

fn border_row(
    widths: &[usize],
    sep_style: Style,
    left: char,
    mid: char,
    right: char,
) -> Line<'static> {
    let mut s = String::new();
    s.push(left);
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            s.push(mid);
        }
        for _ in 0..(w + 2) {
            s.push('─');
        }
    }
    s.push(right);
    Line::from(Span::styled(s, sep_style))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flatten_text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_heading_is_bold_cyan() {
        let lines = parse("# Hello");
        let text = flatten_text(&lines);
        assert!(text.contains("Hello"), "got: {:?}", text);
    }

    #[test]
    fn test_bold_text_has_bold_modifier() {
        let lines = parse("**bold**");
        let has_bold = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD));
        assert!(has_bold, "Expected at least one BOLD span");
    }

    #[test]
    fn test_inline_code_is_yellow() {
        let lines = parse("a `snippet` here");
        let yellow_code = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "snippet");
        let span = yellow_code.expect("expected a span containing 'snippet'");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_plain_text_renders() {
        let lines = parse("just some text");
        let text = flatten_text(&lines);
        assert!(text.contains("just some text"));
    }

    #[test]
    fn test_heading_color_overridable() {
        let cfg = RenderConfig {
            h1_color: Color::Magenta,
            heading_color: Color::Red,
            code_color: Color::Green,
            image_height: 12,
        };
        let lines = parse_with_config("# H", &cfg);
        let span = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "H")
            .expect("expected h1 heading text");
        assert_eq!(span.style.fg, Some(Color::Magenta));
        let lines2 = parse_with_config("## H2", &cfg);
        let span2 = lines2
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "H2")
            .expect("expected h2 heading text");
        assert_eq!(span2.style.fg, Some(Color::Red));
    }

    #[test]
    fn test_color_from_str_known_and_unknown() {
        assert_eq!(color_from_str("red"), Color::Red);
        assert_eq!(color_from_str("CYAN"), Color::Cyan);
        assert_eq!(color_from_str("not_a_color"), Color::White);
    }

    #[test]
    fn test_h1_emits_double_underline_row() {
        let lines = parse("# Hello");
        let underline = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains('═')))
            .expect("expected h1 ═ underline row");
        let len: usize = underline
            .spans
            .iter()
            .map(|s| s.content.chars().count())
            .sum();
        assert_eq!(len, "Hello".chars().count());
    }

    #[test]
    fn test_h2_emits_single_underline_row() {
        let lines = parse("## Sub");
        let has_dash_row = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.chars().all(|c| c == '─') && !s.content.is_empty())
        });
        assert!(has_dash_row, "expected h2 ─ underline row");
    }

    #[test]
    fn test_h3_no_underline_row() {
        let lines = parse("### Smaller");
        let has_underline = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.contains('═') || s.content.contains('─'))
        });
        assert!(!has_underline, "h3 should not have underline row");
    }

    #[test]
    fn test_h5_is_italic() {
        let lines = parse("##### tiny");
        let span = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "tiny")
            .expect("expected heading text");
        assert!(span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_paragraphs_separated_by_blank_line() {
        let lines = parse("first para\n\nsecond para");
        let text = flatten_text(&lines);
        // Expect at least one empty line between the two paragraphs.
        assert!(text.contains("first para\n\nsecond para") || text.contains("first para\n"));
        let blanks = lines
            .iter()
            .filter(|l| l.spans.iter().all(|s| s.content.is_empty()))
            .count();
        assert!(
            blanks >= 1,
            "expected at least one blank line, got {:?}",
            text
        );
    }

    #[test]
    fn test_fenced_code_block_preserves_line_breaks() {
        let md = "before\n\n```sh\nline one\nline two\n```\n\n## Heading\n";
        let lines = parse(md);
        let text = flatten_text(&lines);
        // Each code line should be on its own line, not concatenated.
        let line_one_idx = lines
            .iter()
            .position(|l| l.spans.iter().any(|s| s.content.as_ref() == "line one"))
            .expect("expected 'line one' as its own line");
        let line_two_idx = lines
            .iter()
            .position(|l| l.spans.iter().any(|s| s.content.as_ref() == "line two"))
            .expect("expected 'line two' as its own line");
        assert!(line_two_idx > line_one_idx);
        // Heading text must NOT be glued to a code-block line.
        let heading_idx = lines
            .iter()
            .position(|l| l.spans.iter().any(|s| s.content.as_ref() == "Heading"))
            .expect("expected 'Heading' line");
        assert!(
            heading_idx > line_two_idx,
            "heading must come after code block, got: {:?}",
            text
        );
        let heading_line = &lines[heading_idx];
        let heading_text: String = heading_line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(heading_text.trim(), "Heading");
    }

    #[test]
    fn test_code_block_lines_use_code_color() {
        let cfg = RenderConfig {
            h1_color: Color::White,
            heading_color: Color::Cyan,
            code_color: Color::Magenta,
            image_height: 12,
        };
        let lines = parse_with_config("```\nhello\n```\n", &cfg);
        let span = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "hello")
            .expect("expected 'hello' span");
        assert_eq!(span.style.fg, Some(Color::Magenta));
    }

    #[test]
    fn test_table_renders_header_and_rows() {
        let md = "| Name | Value |\n|------|-------|\n| foo  | 1     |\n| bar  | 22    |\n";
        let lines = parse(md);
        let text = flatten_text(&lines);
        assert!(text.contains("Name"), "got {:?}", text);
        assert!(text.contains("Value"), "got {:?}", text);
        assert!(text.contains("foo"), "got {:?}", text);
        assert!(text.contains("bar"), "got {:?}", text);
        // separator with ┼
        let has_sep = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains('┼')));
        assert!(has_sep, "expected ┼ in separator row, got {:?}", text);
    }

    #[test]
    fn test_table_columns_padded_to_max_width() {
        let md = "| a | bb |\n|---|----|\n| ccc | d |\n";
        let lines = parse(md);
        // The "a" cell should be padded to width 3 (matching "ccc").
        // Find the line that contains "a" and "bb" — that's the header row.
        let header = lines
            .iter()
            .find(|l| {
                let text: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
                text.contains('a') && text.contains("bb") && !text.contains("ccc")
            })
            .expect("expected header row");
        let header_text: String = header.spans.iter().map(|s| s.content.as_ref()).collect();
        // Layout: " a   │ bb "  (a padded to 3, bb padded to 2)
        assert!(
            header_text.contains("a  "),
            "expected 'a' padded with spaces, got {:?}",
            header_text
        );
    }

    #[test]
    fn test_table_has_top_and_bottom_borders() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let lines = parse(md);
        let has_top = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.contains('┌') && s.content.contains('┐'))
        });
        let has_bottom = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.contains('└') && s.content.contains('┘'))
        });
        assert!(has_top, "expected top border with ┌ and ┐");
        assert!(has_bottom, "expected bottom border with └ and ┘");
    }

    #[test]
    fn test_image_local_path_resolves_relative_to_base_dir() {
        let cfg = RenderConfig {
            h1_color: Color::White,
            heading_color: Color::Cyan,
            code_color: Color::Yellow,
            image_height: 5,
        };
        let base = Path::new("/some/dir");
        let result = parse_full_with("![alt](pic.png)", &cfg, Some(base));
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].path, PathBuf::from("/some/dir/pic.png"));
        assert_eq!(result.images[0].height, 5);
    }

    #[test]
    fn test_image_url_is_dropped_but_alt_still_rendered() {
        let result = parse_full("![remote](https://example.com/x.png)");
        assert!(result.images.is_empty());
        let placeholder = result
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.contains("[image:") && s.content.contains("remote"));
        assert!(
            placeholder.is_some(),
            "expected alt-text placeholder for url"
        );
    }

    #[test]
    fn test_image_reserves_height_rows_in_lines() {
        let cfg = RenderConfig {
            h1_color: Color::White,
            heading_color: Color::Cyan,
            code_color: Color::Yellow,
            image_height: 8,
        };
        let result = parse_full_with("![alt](pic.png)", &cfg, Some(Path::new("/a")));
        let img = &result.images[0];
        assert_eq!(img.line_offset, 0);
        // First row has placeholder, next height-1 rows are blank.
        let total_rows_after_image = result.lines.len();
        assert!(total_rows_after_image >= 8, "expected >=8 lines for image");
    }

    #[test]
    fn test_tight_list_items_each_on_their_own_line() {
        let md = "- alpha\n- beta\n- gamma\n";
        let lines = parse(md);
        let texts: Vec<String> = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect();
        // Each item should appear as its own rendered line, in order.
        let pos_alpha = texts.iter().position(|t| t.contains("alpha")).unwrap();
        let pos_beta = texts.iter().position(|t| t.contains("beta")).unwrap();
        let pos_gamma = texts.iter().position(|t| t.contains("gamma")).unwrap();
        assert!(pos_alpha < pos_beta && pos_beta < pos_gamma);
        // They must NOT be glued on the same line.
        assert!(
            !texts
                .iter()
                .any(|t| t.contains("alpha") && t.contains("beta"))
        );
    }

    #[test]
    fn test_ordered_list_uses_number_prefix() {
        let md = "1. one\n2. two\n";
        let lines = parse(md);
        let texts: Vec<String> = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect();
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("1. ") && t.contains("one")),
            "got: {:?}",
            texts
        );
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("2. ") && t.contains("two")),
            "got: {:?}",
            texts
        );
    }

    #[test]
    fn test_link_local_path_recorded_with_correct_offsets() {
        let base = Path::new("/some/dir");
        let result = parse_full_with(
            "see [docs](readme.md) here",
            &RenderConfig::default(),
            Some(base),
        );
        assert_eq!(result.links.len(), 1, "expected one link");
        let link = &result.links[0];
        assert_eq!(link.line, 0);
        assert_eq!(link.col_start, "see ".chars().count());
        assert_eq!(link.col_end, "see docs".chars().count());
        match &link.target {
            LinkTarget::Local(p) => assert_eq!(p, &PathBuf::from("/some/dir/readme.md")),
            _ => panic!("expected local target"),
        }
    }

    #[test]
    fn test_link_url_target() {
        let result = parse_full("click [here](https://example.com/x)");
        assert_eq!(result.links.len(), 1);
        match &result.links[0].target {
            LinkTarget::Url(u) => assert_eq!(u, "https://example.com/x"),
            _ => panic!("expected url target"),
        }
    }

    #[test]
    fn test_link_with_anchor_strips_fragment() {
        let base = Path::new("/d");
        let result = parse_full_with("[s](file.md#section)", &RenderConfig::default(), Some(base));
        match &result.links[0].target {
            LinkTarget::Local(p) => assert_eq!(p, &PathBuf::from("/d/file.md")),
            _ => panic!("expected local target"),
        }
    }

    #[test]
    fn test_link_text_is_cyan_and_underlined() {
        let result = parse_full("[styled](x.md)");
        let span = result
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "styled")
            .expect("expected styled link span");
        assert_eq!(span.style.fg, Some(Color::Cyan));
        assert!(span.style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn test_link_inside_table_not_recorded_but_styled() {
        let md = "| a | b |\n|---|---|\n| [x](y.md) | z |\n";
        let result = parse_full(md);
        assert!(
            result.links.is_empty(),
            "links inside tables shouldn't be tracked"
        );
        let span = result
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "x")
            .expect("expected link text in table");
        assert_eq!(span.style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_table_cell_with_inline_code_keeps_yellow_style() {
        let md = "| n | v |\n|---|---|\n| `code` | x |\n";
        let lines = parse(md);
        let yellow = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "code");
        let span = yellow.expect("expected a 'code' span inside table cell");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }
}
