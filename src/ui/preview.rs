use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

pub struct PreviewWidget<'a> {
    pub lines: &'a [Line<'a>],
    pub scroll: u16,
    pub title: &'a str,
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

        let paragraph = Paragraph::new(self.lines.to_vec()).scroll((self.scroll, 0));
        paragraph.render(chunks[1], buf);
    }
}
