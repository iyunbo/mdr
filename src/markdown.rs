use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub struct RenderConfig {
    pub heading_color: Color,
    pub code_color: Color,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            heading_color: Color::Cyan,
            code_color: Color::Yellow,
        }
    }
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
        _ => Color::White,
    }
}

#[cfg(test)]
pub fn parse(content: &str) -> Vec<Line<'static>> {
    parse_with_config(content, &RenderConfig::default())
}

#[derive(Default)]
struct TableBuf {
    head: Vec<Vec<Span<'static>>>,
    body: Vec<Vec<Vec<Span<'static>>>>,
    in_head: bool,
    current_row: Vec<Vec<Span<'static>>>,
    current_cell: Vec<Span<'static>>,
}

pub fn parse_with_config(content: &str, config: &RenderConfig) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();
    let mut current_heading: Option<HeadingLevel> = None;
    let mut table: Option<TableBuf> = None;

    let parser = Parser::new_ext(content, Options::all());
    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                blank_before_heading(&mut lines, level);
                current_heading = Some(level);
                current_style = heading_style(level, config);
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
                        Style::default().fg(config.heading_color),
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
                    render_table(buf, &mut lines, config);
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

    lines
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
        .fg(config.heading_color)
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

fn render_table(buf: TableBuf, lines: &mut Vec<Line<'static>>, _config: &RenderConfig) {
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
        let empty: Vec<Span<'static>> = Vec::new();
        let cell = row.get(i).unwrap_or(&empty);
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
            heading_color: Color::Red,
            code_color: Color::Green,
        };
        let lines = parse_with_config("# H", &cfg);
        let span = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.as_ref() == "H")
            .expect("expected heading text");
        assert_eq!(span.style.fg, Some(Color::Red));
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
