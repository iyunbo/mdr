use crate::config::Config;
use crate::error::AppError;
use crate::fs::{self, FileNode};
use crate::keys::{self, Action};
use crossterm::event::KeyCode;
use std::collections::HashMap;

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum AppState {
    #[default]
    Browsing,
    Viewing,
    Loading,
}

pub struct App {
    pub running: bool,
    pub state: AppState,
    pub scroll: u16,
    pub content: Option<String>,
    pub file_name: Option<String>,
    pub tree: Option<FileNode>,
    pub tree_cursor: usize,
    pub config: Config,
    pub keymap: HashMap<KeyCode, Action>,
    pub load_error: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let keymap = keys::build_keymap(&config);
        Self {
            running: true,
            state: AppState::default(),
            scroll: 0,
            content: None,
            file_name: None,
            tree: None,
            tree_cursor: 0,
            config,
            keymap,
            load_error: None,
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

    pub fn cursor_top(&mut self) {
        self.tree_cursor = 0;
    }

    pub fn set_loading(&mut self) {
        self.state = AppState::Loading;
        self.content = None;
        self.load_error = None;
    }

    pub fn set_content(&mut self, content: String, name: String) {
        self.content = Some(content);
        self.file_name = Some(name);
        self.scroll = 0;
        self.state = AppState::Viewing;
    }

    pub fn set_error(&mut self, err: String) {
        self.load_error = Some(err);
        self.state = AppState::Browsing;
    }

    pub fn selected_node(&self) -> Option<&FileNode> {
        let tree = self.tree.as_ref()?;
        let mut flat: Vec<&FileNode> = Vec::new();
        Self::flatten_tree(tree, &mut flat);
        flat.get(self.tree_cursor).copied()
    }

    /// Toggle expand/collapse on the dir at the cursor.
    /// Lazy-walks children the first time a dir is expanded.
    pub fn toggle_selected(&mut self) -> Result<(), AppError> {
        let target = self.tree_cursor;
        let Some(tree) = self.tree.as_mut() else {
            return Ok(());
        };
        let mut counter = 0usize;
        Self::toggle_at(tree, target, &mut counter)?;
        self.clamp_cursor();
        Ok(())
    }

    /// Collapse the dir at the cursor (no-op on files or already-collapsed dirs).
    pub fn collapse_selected(&mut self) {
        let target = self.tree_cursor;
        let Some(tree) = self.tree.as_mut() else {
            return;
        };
        let mut counter = 0usize;
        Self::collapse_at(tree, target, &mut counter);
        self.clamp_cursor();
    }

    fn toggle_at(
        node: &mut FileNode,
        target: usize,
        counter: &mut usize,
    ) -> Result<bool, AppError> {
        if *counter == target {
            if let FileNode::Dir {
                expanded,
                children,
                path,
                ..
            } = node
            {
                if *expanded {
                    *expanded = false;
                } else {
                    if children.is_empty() {
                        let walked = fs::walk_dir(path)?;
                        if let FileNode::Dir {
                            children: new_children,
                            ..
                        } = walked
                        {
                            *children = new_children;
                        }
                    }
                    *expanded = true;
                }
            }
            return Ok(true);
        }
        *counter += 1;
        if let FileNode::Dir {
            children,
            expanded: true,
            ..
        } = node
        {
            for child in children.iter_mut() {
                if Self::toggle_at(child, target, counter)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn collapse_at(node: &mut FileNode, target: usize, counter: &mut usize) -> bool {
        if *counter == target {
            if let FileNode::Dir { expanded, .. } = node {
                if *expanded {
                    *expanded = false;
                }
            }
            return true;
        }
        *counter += 1;
        if let FileNode::Dir {
            children,
            expanded: true,
            ..
        } = node
        {
            for child in children.iter_mut() {
                if Self::collapse_at(child, target, counter) {
                    return true;
                }
            }
        }
        false
    }

    fn clamp_cursor(&mut self) {
        let len = self.tree.as_ref().map(Self::flat_len).unwrap_or(0);
        if len == 0 {
            self.tree_cursor = 0;
        } else if self.tree_cursor >= len {
            self.tree_cursor = len - 1;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as stdfs;

    fn make_app_with_tree(tree: FileNode) -> App {
        let mut app = App::new(Config::default());
        app.tree = Some(tree);
        app
    }

    #[test]
    fn test_toggle_collapses_root() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("a.md"), "a").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);

        assert_eq!(App::flat_len(app.tree.as_ref().unwrap()), 2); // root + a.md
        app.toggle_selected().unwrap(); // root is at cursor 0, expanded — collapse
        assert_eq!(App::flat_len(app.tree.as_ref().unwrap()), 1); // root only
    }

    #[test]
    fn test_toggle_expands_collapsed_subdir_lazily() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::create_dir(dir.path().join("sub")).unwrap();
        stdfs::write(dir.path().join("sub").join("inner.md"), "inner").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);

        // Initially: root (expanded) + sub (collapsed). flat_len = 2. Children of `sub` not loaded yet.
        assert_eq!(App::flat_len(app.tree.as_ref().unwrap()), 2);
        app.tree_cursor = 1; // on `sub`
        app.toggle_selected().unwrap();
        // After expanding `sub`, its `inner.md` should appear.
        assert_eq!(App::flat_len(app.tree.as_ref().unwrap()), 3);
    }

    #[test]
    fn test_collapse_clamps_cursor() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("a.md"), "a").unwrap();
        stdfs::write(dir.path().join("b.md"), "b").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);

        app.tree_cursor = 2; // on b.md
        app.collapse_selected(); // cursor on b.md (file) — no-op on collapse_at
        assert_eq!(app.tree_cursor, 2);

        app.tree_cursor = 0; // on root
        app.collapse_selected(); // collapses root
        assert_eq!(App::flat_len(app.tree.as_ref().unwrap()), 1);
        assert_eq!(app.tree_cursor, 0); // clamped from 0 (still in range)
    }
}
