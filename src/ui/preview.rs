use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct PreviewWidget<'a> {
    pub lines: &'a [Line<'a>],
    pub scroll: u16,
    pub title: &'a str,
}

impl<'a> Widget for PreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.title));

        let paragraph = Paragraph::new(self.lines.to_vec())
            .block(block)
            .scroll((self.scroll, 0));

        paragraph.render(area, buf);
    }
}
