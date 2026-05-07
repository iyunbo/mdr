use crate::config::Config;
use crate::error::AppError;
use crate::fs::{self, FileNode};
use crate::keys::{self, Action};
use crate::markdown;
use crossterm::event::KeyCode;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
pub struct SearchInput {
    pub direction: SearchDirection,
    pub buffer: String,
}

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
    pub count_buffer: String,
    pub search_input: Option<SearchInput>,
    pub last_search: Option<String>,
    pub last_search_direction: SearchDirection,
    pub status_message: Option<String>,
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
            count_buffer: String::new(),
            search_input: None,
            last_search: None,
            last_search_direction: SearchDirection::Forward,
            status_message: None,
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

    /// Append a digit to the count buffer (used for vi-style `Nj`, `Nk`).
    /// `0` is only treated as a count when the buffer is non-empty.
    pub fn push_count_digit(&mut self, c: char) -> bool {
        if !c.is_ascii_digit() {
            return false;
        }
        if c == '0' && self.count_buffer.is_empty() {
            return false;
        }
        if self.count_buffer.len() < 6 {
            self.count_buffer.push(c);
        }
        true
    }

    /// Consume the count buffer, returning the count (>= 1).
    pub fn take_count(&mut self) -> u32 {
        let n = self
            .count_buffer
            .parse::<u32>()
            .unwrap_or(1)
            .clamp(1, 10_000);
        self.count_buffer.clear();
        n
    }

    pub fn start_search(&mut self, direction: SearchDirection) {
        self.search_input = Some(SearchInput {
            direction,
            buffer: String::new(),
        });
        self.status_message = None;
    }

    pub fn cancel_search(&mut self) {
        self.search_input = None;
    }

    pub fn search_input_push(&mut self, c: char) {
        if let Some(input) = self.search_input.as_mut() {
            input.buffer.push(c);
        }
    }

    pub fn search_input_pop(&mut self) {
        if let Some(input) = self.search_input.as_mut() {
            input.buffer.pop();
        }
    }

    /// Confirm the active search prompt: store the query, jump to first match.
    pub fn confirm_search(&mut self) {
        let Some(input) = self.search_input.take() else {
            return;
        };
        if input.buffer.is_empty() {
            return;
        }
        self.last_search = Some(input.buffer.clone());
        self.last_search_direction = input.direction;
        if !self.jump_to_match(&input.buffer, input.direction, false) {
            self.status_message = Some(format!("Pattern not found: {}", input.buffer));
        } else {
            self.status_message = None;
        }
    }

    /// Repeat the last search. `reverse` flips direction (used by `N`).
    pub fn repeat_search(&mut self, reverse: bool) {
        let Some(query) = self.last_search.clone() else {
            self.status_message = Some("No previous search".to_string());
            return;
        };
        let direction = if reverse {
            match self.last_search_direction {
                SearchDirection::Forward => SearchDirection::Backward,
                SearchDirection::Backward => SearchDirection::Forward,
            }
        } else {
            self.last_search_direction
        };
        if !self.jump_to_match(&query, direction, true) {
            self.status_message = Some(format!("Pattern not found: {}", query));
        } else {
            self.status_message = None;
        }
    }

    /// Search target lines for the current state.
    fn search_target_lines(&self) -> Vec<String> {
        match self.state {
            AppState::Viewing => {
                let content = self.content.as_deref().unwrap_or("");
                let cfg = markdown::RenderConfig {
                    heading_color: markdown::color_from_str(&self.config.theme.heading_color),
                    code_color: markdown::color_from_str(&self.config.theme.code_color),
                };
                markdown::parse_with_config(content, &cfg)
                    .iter()
                    .map(|line| {
                        line.spans
                            .iter()
                            .map(|s| s.content.as_ref())
                            .collect::<String>()
                    })
                    .collect()
            }
            AppState::Browsing => {
                let Some(tree) = self.tree.as_ref() else {
                    return Vec::new();
                };
                let mut flat: Vec<&FileNode> = Vec::new();
                Self::flatten_tree(tree, &mut flat);
                flat.iter().map(|n| n.name().to_string()).collect()
            }
            AppState::Loading => Vec::new(),
        }
    }

    fn current_search_index(&self) -> usize {
        match self.state {
            AppState::Viewing => self.scroll as usize,
            AppState::Browsing => self.tree_cursor,
            AppState::Loading => 0,
        }
    }

    fn jump_to(&mut self, idx: usize) {
        match self.state {
            AppState::Viewing => self.scroll = idx as u16,
            AppState::Browsing => self.tree_cursor = idx,
            AppState::Loading => {}
        }
    }

    /// Search for `query` in the current view, wrapping around. `skip_current`
    /// is true when called from `n`/`N` so we don't re-match the line we're
    /// already on.
    fn jump_to_match(
        &mut self,
        query: &str,
        direction: SearchDirection,
        skip_current: bool,
    ) -> bool {
        if query.is_empty() {
            return false;
        }
        let lines = self.search_target_lines();
        let n = lines.len();
        if n == 0 {
            return false;
        }
        let from = self.current_search_index().min(n.saturating_sub(1));
        let q = query.to_lowercase();

        let order: Vec<usize> = match direction {
            SearchDirection::Forward => {
                let start = if skip_current { from + 1 } else { from };
                let mut v: Vec<usize> = (start..n).collect();
                v.extend(0..start.min(n));
                v
            }
            SearchDirection::Backward => {
                let mut v: Vec<usize> = if skip_current {
                    (0..from).rev().collect()
                } else {
                    (0..=from).rev().collect()
                };
                let wrap_start = if skip_current { from } else { from + 1 };
                v.extend((wrap_start.min(n)..n).rev());
                v
            }
        };

        for i in order {
            if lines[i].to_lowercase().contains(&q) {
                self.jump_to(i);
                return true;
            }
        }
        false
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
            if let FileNode::Dir { expanded, .. } = node
                && *expanded
            {
                *expanded = false;
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
    fn test_count_buffer_digits_and_take() {
        let mut app = App::new(Config::default());
        assert!(!app.push_count_digit('0')); // leading 0 ignored
        assert!(app.push_count_digit('1'));
        assert!(app.push_count_digit('2'));
        assert!(app.push_count_digit('0')); // 0 ok now
        assert!(!app.push_count_digit('a'));
        assert_eq!(app.take_count(), 120);
        assert_eq!(app.count_buffer, "");
        assert_eq!(app.take_count(), 1); // empty -> 1
    }

    #[test]
    fn test_search_in_browsing_jumps_cursor() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("alpha.md"), "a").unwrap();
        stdfs::write(dir.path().join("beta.md"), "b").unwrap();
        stdfs::write(dir.path().join("gamma.md"), "g").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        // Flat order: root, alpha.md, beta.md, gamma.md
        app.tree_cursor = 0;
        app.start_search(SearchDirection::Forward);
        for c in "BETA".chars() {
            app.search_input_push(c);
        }
        app.confirm_search();
        assert_eq!(app.tree_cursor, 2);
        assert_eq!(app.last_search.as_deref(), Some("BETA"));

        // n repeats forward: nothing else matches "BETA", so wraps and lands on same.
        app.repeat_search(false);
        assert_eq!(app.tree_cursor, 2);
    }

    #[test]
    fn test_search_not_found_sets_status_message() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("only.md"), "x").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        app.start_search(SearchDirection::Forward);
        for c in "zzz".chars() {
            app.search_input_push(c);
        }
        app.confirm_search();
        assert!(app.status_message.is_some());
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
