use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct PreviewWidget<'a> {
    pub lines: &'a [Line<'static>],
    pub scroll: u16,
    pub title: &'a str,
    pub show_line_numbers: bool,
    pub line_number_color: Color,
}

impl<'a> Widget for PreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        let title = Paragraph::new(self.title).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        title.render(chunks[0], buf);

        if self.show_line_numbers {
            let total = self.lines.len();
            let digits = total.to_string().len().max(2);
            let num_style = Style::default().fg(self.line_number_color);
            let numbered: Vec<Line<'static>> = self
                .lines
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    let mut spans: Vec<Span<'static>> = Vec::with_capacity(line.spans.len() + 1);
                    spans.push(Span::styled(
                        format!("{:>width$} ", i + 1, width = digits),
                        num_style,
                    ));
                    spans.extend(line.spans.iter().cloned());
                    Line::from(spans)
                })
                .collect();
            let paragraph = Paragraph::new(numbered).scroll((self.scroll, 0));
            paragraph.render(chunks[1], buf);
        } else {
            let paragraph = Paragraph::new(self.lines.to_vec()).scroll((self.scroll, 0));
            paragraph.render(chunks[1], buf);
        }
    }
}
