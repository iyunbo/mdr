use crate::fs::FileNode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, node: &FileNode, cursor: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let title = Paragraph::new("Files").style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(title, chunks[0]);

    let mut items: Vec<ListItem> = Vec::new();
    node.visit_visible(0, &mut |depth, n| {
        let indent = "  ".repeat(depth);
        let prefix = match n {
            FileNode::Dir { expanded: true, .. } => "▼ ",
            FileNode::Dir {
                expanded: false, ..
            } => "▶ ",
            FileNode::File(_) => "  ",
        };
        let style = if n.is_markdown() {
            Style::default().fg(Color::White)
        } else if matches!(n, FileNode::Dir { .. }) {
            Style::default().fg(Color::Gray)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        items.push(
            ListItem::new(Line::from(vec![Span::raw(format!(
                "{}{}{}",
                indent,
                prefix,
                n.name()
            ))]))
            .style(style),
        );
    });

    let list = List::new(items).highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

    let mut state = ListState::default();
    state.select(Some(cursor));
    frame.render_stateful_widget(list, chunks[1], &mut state);
}
