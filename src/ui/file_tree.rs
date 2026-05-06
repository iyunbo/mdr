use crate::fs::FileNode;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

pub fn render(frame: &mut Frame, area: Rect, node: &FileNode, cursor: usize) {
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
        .block(Block::default().borders(Borders::ALL).title(" Files "))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

    let mut state = ListState::default();
    state.select(Some(cursor));
    frame.render_stateful_widget(list, area, &mut state);
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
