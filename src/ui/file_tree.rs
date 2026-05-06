use crate::fs::FileNode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

pub struct FileTreeWidget<'a> {
    pub node: &'a FileNode,
    pub cursor: usize,
}

impl<'a> FileTreeWidget<'a> {
    fn flatten<'b>(node: &'b FileNode, depth: usize, items: &mut Vec<(usize, &'b FileNode)>) {
        items.push((depth, node));
        if let FileNode::Dir {
            children,
            expanded: true,
            ..
        } = node
        {
            for child in children {
                Self::flatten(child, depth + 1, items);
            }
        }
    }
}

impl<'a> Widget for FileTreeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut flat: Vec<(usize, &FileNode)> = Vec::new();
        Self::flatten(self.node, 0, &mut flat);

        let items: Vec<ListItem> = flat
            .iter()
            .enumerate()
            .map(|(i, (depth, node))| {
                let indent = "  ".repeat(*depth);
                let prefix = match node {
                    FileNode::Dir { expanded: true, .. } => "▼ ",
                    FileNode::Dir { expanded: false, .. } => "▶ ",
                    FileNode::File(_) => "  ",
                };
                let style = if i == self.cursor {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else if node.is_markdown() {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                ListItem::new(Line::from(vec![Span::raw(format!(
                    "{}{}{}",
                    indent,
                    prefix,
                    node.name()
                ))]))
                .style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Files "));

        list.render(area, buf);
    }
}
