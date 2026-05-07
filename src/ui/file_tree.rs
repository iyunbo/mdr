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

    let title = Paragraph::new("Files")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(title, chunks[0]);

    let mut flat: Vec<(usize, &FileNode)> = Vec::new();
    flatten(node, 0, &mut flat);

    let items: Vec<ListItem> = flat
        .iter()
        .map(|(depth, n)| {
            let indent = "  ".repeat(*depth);
            let prefix = match n {
                FileNode::Dir { expanded: true, .. } => "▼ ",
                FileNode::Dir { expanded: false, .. } => "▶ ",
                FileNode::File(_) => "  ",
            };
            let style = if n.is_markdown() {
                Style::default().fg(Color::White)
            } else if matches!(n, FileNode::Dir { .. }) {
                Style::default().fg(Color::Gray)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            ListItem::new(Line::from(vec![Span::raw(format!(
                "{}{}{}",
                indent,
                prefix,
                n.name()
            ))]))
            .style(style)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

    let mut state = ListState::default();
    state.select(Some(cursor));
    frame.render_stateful_widget(list, chunks[1], &mut state);
}

fn flatten<'a>(node: &'a FileNode, depth: usize, items: &mut Vec<(usize, &'a FileNode)>) {
    items.push((depth, node));
    if let FileNode::Dir {
        children,
        expanded: true,
        ..
    } = node
    {
        for child in children {
            flatten(child, depth + 1, items);
        }
    }
}
