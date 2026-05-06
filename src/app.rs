use crate::fs::FileNode;

#[derive(Debug, Default, PartialEq)]
pub enum AppState {
    #[default]
    Browsing,
    Viewing,
}

pub struct App {
    pub running: bool,
    pub state: AppState,
    pub scroll: u16,
    pub content: Option<String>,
    pub file_name: Option<String>,
    pub tree: Option<FileNode>,
    pub tree_cursor: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            state: AppState::default(),
            scroll: 0,
            content: None,
            file_name: None,
            tree: None,
            tree_cursor: 0,
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_top(&mut self) {
        self.scroll = 0;
    }

    pub fn cursor_down(&mut self) {
        let max = self.tree.as_ref().map(Self::flat_len).unwrap_or(0);
        if max > 0 && self.tree_cursor + 1 < max {
            self.tree_cursor += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        self.tree_cursor = self.tree_cursor.saturating_sub(1);
    }

    pub fn selected_node(&self) -> Option<&FileNode> {
        let tree = self.tree.as_ref()?;
        let mut flat: Vec<&FileNode> = Vec::new();
        Self::flatten_tree(tree, &mut flat);
        flat.get(self.tree_cursor).copied()
    }

    fn flatten_tree<'a>(node: &'a FileNode, out: &mut Vec<&'a FileNode>) {
        out.push(node);
        if let FileNode::Dir {
            children,
            expanded: true,
            ..
        } = node
        {
            for child in children {
                Self::flatten_tree(child, out);
            }
        }
    }

    fn flat_len(node: &FileNode) -> usize {
        let mut flat: Vec<&FileNode> = Vec::new();
        Self::flatten_tree(node, &mut flat);
        flat.len()
    }
}
