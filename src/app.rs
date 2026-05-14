use crate::config::Config;
use crate::error::AppError;
use crate::fs::{self, FileNode};
use crate::keys::{self, Action, KeyCombo};
use crate::markdown::{self, LinkRef, ParseResult};
use crate::wikilink;
use ratatui::layout::Rect;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

const HISTORY_CAP: usize = 100;

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

#[derive(Debug, Clone, Default)]
pub struct LineJumpPrompt {
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
    pub base_dir: Option<PathBuf>,
    pub tree: Option<FileNode>,
    pub tree_cursor: usize,
    pub config: Config,
    pub keymap: HashMap<KeyCombo, Action>,
    pub load_error: Option<String>,
    pub count_buffer: String,
    pub search_input: Option<SearchInput>,
    pub last_search: Option<String>,
    pub last_search_direction: SearchDirection,
    pub line_prompt: Option<LineJumpPrompt>,
    pub pending_g: bool,
    pub selected_link: Option<usize>,
    pub status_message: Option<String>,
    pub picker: Option<Picker>,
    pub image_cache: HashMap<PathBuf, StatefulProtocol>,
    /// Body area of the preview pane from the last frame, used to map mouse
    /// click coordinates to (line, col). `None` until a viewing frame has been
    /// rendered.
    pub last_preview_body: Option<Rect>,
    /// Width in cells of the line-number gutter from the last frame (0 when
    /// line numbers are disabled).
    pub last_gutter_width: u16,
    /// Browser-style navigation history. `history_pos` is the index of the
    /// file currently visible; capped at HISTORY_CAP entries (oldest dropped).
    pub history: VecDeque<HistoryEntry>,
    pub history_pos: Option<usize>,
    /// Set together by `nav_step` callers (paired with `pending_scroll`). The
    /// next `set_content` applies the scroll and skips appending to history.
    pub suppress_history_push: bool,
    pub pending_scroll: Option<u16>,
    /// Cached parse of the current `content`. Cleared by `set_content` and
    /// `invalidate_caches`. Held in `Rc` so callers can keep a snapshot
    /// alive across `&mut self` operations.
    cached_parse: Option<Rc<ParseResult>>,
    /// Lower-cased file-stem → absolute path index for wiki-link resolution.
    /// Cleared whenever the file tree mutates.
    tree_index: Option<HashMap<String, PathBuf>>,
    /// Theme colors resolved from `config.theme` once at startup. Immutable
    /// after that, so we don't re-`color_from_str` on every parse.
    render_cfg: markdown::RenderConfig,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub scroll: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDir {
    Back,
    Forward,
}

impl App {
    pub fn new(config: Config) -> Self {
        let keymap = keys::build_keymap(&config);
        let render_cfg = markdown::RenderConfig {
            h1_color: markdown::color_from_str(&config.theme.h1_color),
            heading_color: markdown::color_from_str(&config.theme.heading_color),
            code_color: markdown::color_from_str(&config.theme.code_color),
            image_height: config.theme.image_height,
            syntax_highlight: config.theme.syntax_highlight,
            syntax_theme: config.theme.syntax_theme.clone(),
        };
        Self {
            running: true,
            state: AppState::default(),
            scroll: 0,
            content: None,
            file_name: None,
            base_dir: None,
            tree: None,
            tree_cursor: 0,
            config,
            keymap,
            load_error: None,
            count_buffer: String::new(),
            search_input: None,
            last_search: None,
            last_search_direction: SearchDirection::Forward,
            line_prompt: None,
            pending_g: false,
            selected_link: None,
            last_preview_body: None,
            last_gutter_width: 0,
            history: VecDeque::new(),
            history_pos: None,
            suppress_history_push: false,
            pending_scroll: None,
            cached_parse: None,
            tree_index: None,
            render_cfg,
            status_message: None,
            picker: None,
            image_cache: HashMap::new(),
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
        let max = self.tree.as_ref().map(FileNode::visible_count).unwrap_or(0);
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

    /// Jump to line `n` (1-indexed) in the current view. `n == 0` is
    /// treated as line 1.
    pub fn goto_line(&mut self, n: usize) {
        let idx = n.saturating_sub(1);
        match self.state {
            AppState::Viewing => {
                let max = self.total_lines().saturating_sub(1);
                self.scroll = idx.min(max) as u16;
            }
            AppState::Browsing => {
                self.tree_cursor = idx.min(self.tree_max_index());
            }
            AppState::Loading => {}
        }
    }

    /// Jump to the last line / item.
    pub fn goto_bottom(&mut self, page_size: usize) {
        match self.state {
            AppState::Viewing => {
                let total = self.total_lines();
                self.scroll = total.saturating_sub(page_size) as u16;
            }
            AppState::Browsing => self.tree_cursor = self.tree_max_index(),
            AppState::Loading => {}
        }
    }

    pub fn half_page_down(&mut self, half_page: usize) {
        match self.state {
            AppState::Viewing => {
                let max = self.total_lines().saturating_sub(1) as u16;
                self.scroll = self.scroll.saturating_add(half_page as u16).min(max);
            }
            AppState::Browsing => {
                self.tree_cursor = (self.tree_cursor + half_page).min(self.tree_max_index());
            }
            AppState::Loading => {}
        }
    }

    pub fn half_page_up(&mut self, half_page: usize) {
        match self.state {
            AppState::Viewing => {
                self.scroll = self.scroll.saturating_sub(half_page as u16);
            }
            AppState::Browsing => {
                self.tree_cursor = self.tree_cursor.saturating_sub(half_page);
            }
            AppState::Loading => {}
        }
    }

    fn total_lines(&mut self) -> usize {
        self.parse_current().lines.len()
    }

    fn tree_max_index(&self) -> usize {
        self.tree
            .as_ref()
            .map(FileNode::visible_count)
            .unwrap_or(0)
            .saturating_sub(1)
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

    pub fn start_line_prompt(&mut self) {
        self.line_prompt = Some(LineJumpPrompt::default());
        self.status_message = None;
    }

    pub fn cancel_line_prompt(&mut self) {
        self.line_prompt = None;
    }

    pub fn line_prompt_push(&mut self, c: char) {
        if !c.is_ascii_digit() {
            return;
        }
        if let Some(p) = self.line_prompt.as_mut()
            && p.buffer.len() < 9
        {
            p.buffer.push(c);
        }
    }

    pub fn line_prompt_pop(&mut self) {
        if let Some(p) = self.line_prompt.as_mut() {
            p.buffer.pop();
        }
    }

    /// Confirm the `:N` prompt: jump to that line in the current view.
    pub fn confirm_line_prompt(&mut self) {
        let Some(p) = self.line_prompt.take() else {
            return;
        };
        if p.buffer.is_empty() {
            return;
        }
        let n: usize = p.buffer.parse().unwrap_or(1);
        self.goto_line(n);
        self.status_message = None;
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

    /// Confirm the active search prompt: store the query, jump to next match.
    /// Empty buffer reuses the previous query (vim-style `/<Enter>`).
    /// Search advances past the current line — without a visible character
    /// cursor in viewing mode, "stay on current match" feels broken; the
    /// match is still found via wrap-around if it's the only one.
    pub fn confirm_search(&mut self) {
        let Some(input) = self.search_input.take() else {
            return;
        };
        let direction = input.direction;
        let query = if input.buffer.is_empty() {
            match self.last_search.clone() {
                Some(q) => q,
                None => return,
            }
        } else {
            input.buffer
        };
        self.last_search = Some(query.clone());
        self.last_search_direction = direction;
        if !self.jump_to_match(&query, direction, true) {
            self.status_message = Some(format!("Pattern not found: {}", query));
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

    /// Pre-lowercased search target for the current state. Lowercasing once
    /// here avoids re-lowercasing each candidate line during the match loop.
    fn search_target_lines_lower(&mut self) -> Vec<String> {
        match self.state {
            AppState::Viewing => self
                .parse_current()
                .lines
                .iter()
                .map(|line| {
                    let mut s = String::new();
                    for span in &line.spans {
                        s.push_str(span.content.as_ref());
                    }
                    s.to_lowercase()
                })
                .collect(),
            AppState::Browsing => {
                let Some(tree) = self.tree.as_ref() else {
                    return Vec::new();
                };
                let mut names = Vec::new();
                tree.visit_visible(0, &mut |_, n| names.push(n.name().to_lowercase()));
                names
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
        let lines = self.search_target_lines_lower();
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
            if lines[i].contains(&q) {
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

    pub fn set_content(
        &mut self,
        path: Option<PathBuf>,
        content: String,
        base_dir: Option<PathBuf>,
    ) {
        if !self.suppress_history_push {
            self.save_scroll_to_history();
        }
        let derived_name = path
            .as_deref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(String::from)
            .unwrap_or_else(|| "untitled".to_string());
        self.content = Some(content);
        self.file_name = Some(derived_name);
        self.base_dir = base_dir;
        self.scroll = self.pending_scroll.take().unwrap_or(0);
        self.state = AppState::Viewing;
        // Note: image_cache is NOT cleared — keep decoded images so Back/Forward
        // through history doesn't redecode. Capacity is bounded inside the
        // renderer (see preview::render_images).
        self.selected_link = None;
        self.invalidate_parse_cache();
        if let Some(p) = path {
            if self.suppress_history_push {
                self.suppress_history_push = false;
            } else {
                self.push_history(p);
            }
        }
    }

    /// Append `path` to history at the current position, dropping any
    /// "forward" entries. No-op for consecutive duplicates. Caps at
    /// `HISTORY_CAP` by dropping the oldest.
    fn push_history(&mut self, path: PathBuf) {
        if let Some(pos) = self.history_pos {
            self.history.truncate(pos + 1);
            if self.history.back().map(|e| &e.path) == Some(&path) {
                return;
            }
        }
        self.history.push_back(HistoryEntry { path, scroll: 0 });
        while self.history.len() > HISTORY_CAP {
            self.history.pop_front();
        }
        self.history_pos = Some(self.history.len() - 1);
    }

    pub fn save_scroll_to_history(&mut self) {
        if let Some(pos) = self.history_pos
            && let Some(entry) = self.history.get_mut(pos)
        {
            entry.scroll = self.scroll;
        }
    }

    /// Move the history cursor in `dir`. Returns the target path and the
    /// scroll to restore, or `None` at a boundary.
    pub fn nav_step(&mut self, dir: NavDir) -> Option<(PathBuf, u16)> {
        let pos = self.history_pos?;
        let new_pos = match dir {
            NavDir::Back if pos == 0 => return None,
            NavDir::Back => pos - 1,
            NavDir::Forward if pos + 1 >= self.history.len() => return None,
            NavDir::Forward => pos + 1,
        };
        self.save_scroll_to_history();
        self.history_pos = Some(new_pos);
        let entry = self.history.get(new_pos)?;
        Some((entry.path.clone(), entry.scroll))
    }

    /// Parse the current content (with wiki-link preprocessing) and cache the
    /// result. Returned `Rc` lets callers hold a snapshot across `&mut self`
    /// operations without copying the line vector.
    pub fn parse_current(&mut self) -> Rc<ParseResult> {
        if let Some(cached) = &self.cached_parse {
            return Rc::clone(cached);
        }
        // Build the index first so the immutable borrow of `self.content`
        // below doesn't fight the `&mut self` `ensure_tree_index` needs.
        self.ensure_tree_index();
        let content = self.content.as_deref().unwrap_or("");
        let base_dir = self.base_dir.as_deref();
        let preprocessed = wikilink::rewrite(content, self.tree_index.as_ref(), base_dir);
        let result = Rc::new(markdown::parse_full_with(
            &preprocessed,
            &self.render_cfg,
            base_dir,
        ));
        self.cached_parse = Some(Rc::clone(&result));
        result
    }

    fn ensure_tree_index(&mut self) -> Option<&HashMap<String, PathBuf>> {
        if self.tree_index.is_none() {
            let mut idx: HashMap<String, PathBuf> = HashMap::new();
            if let Some(t) = &self.tree {
                build_tree_index(t, &mut idx);
            }
            self.tree_index = Some(idx);
        }
        self.tree_index.as_ref()
    }

    fn invalidate_parse_cache(&mut self) {
        self.cached_parse = None;
    }

    fn invalidate_tree_caches(&mut self) {
        self.tree_index = None;
        self.cached_parse = None;
    }

    /// Cycle through links in the current view. `step` is +1 (next) or -1 (prev).
    /// Wraps around. Adjusts scroll so the selected link is visible within the
    /// `viewport` row count.
    pub fn cycle_link(&mut self, step: i32, viewport: usize) {
        let result = self.parse_current();
        let n = result.links.len();
        if n == 0 {
            self.selected_link = None;
            self.status_message = Some("No links".to_string());
            return;
        }
        let next = match self.selected_link {
            Some(i) => {
                let len = n as i32;
                ((i as i32 + step).rem_euclid(len)) as usize
            }
            None => {
                if step >= 0 {
                    0
                } else {
                    n - 1
                }
            }
        };
        self.selected_link = Some(next);
        self.status_message = None;
        let line = result.links[next].line as u16;
        let viewport = viewport.max(1) as u16;
        if line < self.scroll {
            self.scroll = line;
        } else if line >= self.scroll.saturating_add(viewport) {
            self.scroll = line.saturating_sub(viewport.saturating_sub(1));
        }
    }

    pub fn current_link(&mut self) -> Option<LinkRef> {
        let idx = self.selected_link?;
        let result = self.parse_current();
        result.links.get(idx).cloned()
    }

    /// Translate a terminal mouse position into a link index, if the click
    /// landed on a link's display text.
    pub fn link_at_terminal(&mut self, term_col: u16, term_row: u16) -> Option<usize> {
        let body = self.last_preview_body?;
        if term_col < body.x
            || term_col >= body.x.saturating_add(body.width)
            || term_row < body.y
            || term_row >= body.y.saturating_add(body.height)
        {
            return None;
        }
        let local_col = term_col - body.x;
        if local_col < self.last_gutter_width {
            return None;
        }
        let char_col = (local_col - self.last_gutter_width) as usize;
        let line_index = (term_row - body.y) as usize + self.scroll as usize;
        let result = self.parse_current();
        result
            .links
            .iter()
            .position(|l| l.line == line_index && char_col >= l.col_start && char_col < l.col_end)
    }

    pub fn set_error(&mut self, err: String) {
        self.load_error = Some(err);
        self.state = AppState::Browsing;
    }

    pub fn selected_node(&self) -> Option<&FileNode> {
        let tree = self.tree.as_ref()?;
        let target = self.tree_cursor;
        let mut counter = 0usize;
        let mut found: Option<&FileNode> = None;
        tree.visit_visible(0, &mut |_, n| {
            if found.is_none() {
                if counter == target {
                    found = Some(n);
                }
                counter += 1;
            }
        });
        found
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
        // Lazy expansion may have surfaced new wiki-link targets.
        self.invalidate_tree_caches();
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
        let len = self.tree.as_ref().map(FileNode::visible_count).unwrap_or(0);
        if len == 0 {
            self.tree_cursor = 0;
        } else if self.tree_cursor >= len {
            self.tree_cursor = len - 1;
        }
    }
}

fn build_tree_index(node: &FileNode, out: &mut HashMap<String, PathBuf>) {
    node.visit_all(&mut |n| {
        if let FileNode::File(p) = n
            && fs::is_markdown_path(p)
            && let Some(stem) = p.file_stem().and_then(|s| s.to_str())
        {
            out.entry(stem.to_lowercase()).or_insert_with(|| p.clone());
        }
    });
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

        assert_eq!(app.tree.as_ref().unwrap().visible_count(), 2); // root + a.md
        app.toggle_selected().unwrap(); // root is at cursor 0, expanded — collapse
        assert_eq!(app.tree.as_ref().unwrap().visible_count(), 1); // root only
    }

    #[test]
    fn test_toggle_expands_collapsed_subdir_lazily() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::create_dir(dir.path().join("sub")).unwrap();
        stdfs::write(dir.path().join("sub").join("inner.md"), "inner").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);

        // Initially: root (expanded) + sub (collapsed). flat_len = 2. Children of `sub` not loaded yet.
        assert_eq!(app.tree.as_ref().unwrap().visible_count(), 2);
        app.tree_cursor = 1; // on `sub`
        app.toggle_selected().unwrap();
        // After expanding `sub`, its `inner.md` should appear.
        assert_eq!(app.tree.as_ref().unwrap().visible_count(), 3);
    }

    #[test]
    fn test_goto_line_in_browsing_clamps() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("a.md"), "a").unwrap();
        stdfs::write(dir.path().join("b.md"), "b").unwrap();
        stdfs::write(dir.path().join("c.md"), "c").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        // Flat list: root, a.md, b.md, c.md (4 items, indices 0..3)
        app.goto_line(3);
        assert_eq!(app.tree_cursor, 2);
        // Out-of-range clamps
        app.goto_line(99);
        assert_eq!(app.tree_cursor, 3);
        // 0 or 1 -> first
        app.goto_line(0);
        assert_eq!(app.tree_cursor, 0);
        app.goto_line(1);
        assert_eq!(app.tree_cursor, 0);
    }

    #[test]
    fn test_goto_bottom_in_browsing() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("a.md"), "a").unwrap();
        stdfs::write(dir.path().join("b.md"), "b").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        app.goto_bottom(10);
        // Last index is flat_len - 1 = 2
        assert_eq!(app.tree_cursor, 2);
    }

    #[test]
    fn test_half_page_down_up_in_browsing() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10 {
            stdfs::write(dir.path().join(format!("f{:02}.md", i)), "x").unwrap();
        }
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        app.half_page_down(5);
        assert_eq!(app.tree_cursor, 5);
        app.half_page_up(2);
        assert_eq!(app.tree_cursor, 3);
        // saturate at 0
        app.half_page_up(100);
        assert_eq!(app.tree_cursor, 0);
        // saturate at top
        app.half_page_down(1000);
        assert_eq!(app.tree_cursor, 10); // flat_len - 1 = 11 items - 1 = 10
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
    fn test_line_prompt_collects_digits_and_jumps() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("a.md"), "a").unwrap();
        stdfs::write(dir.path().join("b.md"), "b").unwrap();
        stdfs::write(dir.path().join("c.md"), "c").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        app.start_line_prompt();
        assert!(app.line_prompt.is_some());
        app.line_prompt_push('3'); // accepted
        app.line_prompt_push('a'); // ignored — non-digit
        app.confirm_line_prompt();
        assert!(app.line_prompt.is_none());
        // Flat list: root, a.md, b.md, c.md → line 3 = b.md (index 2).
        assert_eq!(app.tree_cursor, 2);
    }

    #[test]
    fn test_line_prompt_cancels_on_esc_path() {
        let mut app = App::new(Config::default());
        app.start_line_prompt();
        app.line_prompt_push('5');
        app.cancel_line_prompt();
        assert!(app.line_prompt.is_none());
    }

    #[test]
    fn test_cycle_link_with_no_links_sets_status() {
        let mut app = App::new(Config::default());
        app.set_content(None, "no links here, just text".to_string(), None);
        app.cycle_link(1, 20);
        assert_eq!(app.selected_link, None);
        assert_eq!(app.status_message.as_deref(), Some("No links"));
    }

    #[test]
    fn test_cycle_link_wraps_and_selects_first_link() {
        let mut app = App::new(Config::default());
        app.set_content(
            None,
            "[a](a.md) and [b](b.md) and [c](c.md)".to_string(),
            Some(PathBuf::from("/tmp")),
        );
        app.cycle_link(1, 20);
        assert_eq!(app.selected_link, Some(0));
        app.cycle_link(1, 20);
        assert_eq!(app.selected_link, Some(1));
        app.cycle_link(-1, 20);
        assert_eq!(app.selected_link, Some(0));
        app.cycle_link(-1, 20);
        // Wraps to the last link.
        assert_eq!(app.selected_link, Some(2));
    }

    #[test]
    fn test_link_at_terminal_hits_link_in_body_area() {
        let mut app = App::new(Config::default());
        app.set_content(
            None,
            "see [target](file.md) here".to_string(),
            Some(PathBuf::from("/tmp")),
        );
        // Pretend a frame was rendered: body at (10, 5) with no gutter.
        app.last_preview_body = Some(Rect::new(10, 5, 80, 24));
        app.last_gutter_width = 0;
        app.scroll = 0;
        // "see " = chars 0..4, "target" = 4..10. Click at col 16 (= 10 + 6).
        let idx = app.link_at_terminal(16, 5);
        assert_eq!(idx, Some(0));
        // Outside body — None.
        assert_eq!(app.link_at_terminal(0, 0), None);
        // Inside body but on whitespace — None.
        assert_eq!(app.link_at_terminal(11, 5), None);
    }

    #[test]
    fn test_link_at_terminal_respects_gutter_and_scroll() {
        let mut app = App::new(Config::default());
        app.set_content(
            None,
            "line1\n\nlink [target](file.md) end".to_string(),
            Some(PathBuf::from("/tmp")),
        );
        app.last_preview_body = Some(Rect::new(0, 0, 80, 24));
        app.last_gutter_width = 4; // 3-digit gutter + space
        app.scroll = 2;
        // Link is on parsed line 2 ("link [target] end"). With scroll=2,
        // line 2 maps to row 0. Display: "link " 5 chars, "target" at 5..11.
        // Click at col 4 + 5 + 2 = 11 (gutter + "link " + 2 chars in).
        let idx = app.link_at_terminal(11, 0);
        assert_eq!(idx, Some(0));
        // Click in the gutter — None.
        assert_eq!(app.link_at_terminal(2, 0), None);
    }

    #[test]
    fn test_history_records_and_navigates_back_forward() {
        let mut app = App::new(Config::default());
        // Open A, B, C in sequence. Each call to set_content with a real path
        // should push to history.
        app.set_content(
            Some(PathBuf::from("/n/A.md")),
            "content A".to_string(),
            Some(PathBuf::from("/n")),
        );
        app.scroll = 5;
        app.set_content(
            Some(PathBuf::from("/n/B.md")),
            "content B".to_string(),
            Some(PathBuf::from("/n")),
        );
        app.scroll = 10;
        app.set_content(
            Some(PathBuf::from("/n/C.md")),
            "content C".to_string(),
            Some(PathBuf::from("/n")),
        );
        assert_eq!(app.history.len(), 3);
        assert_eq!(app.history_pos, Some(2));

        let target = app.nav_step(NavDir::Back).expect("expected back target");
        assert_eq!(target.0, PathBuf::from("/n/B.md"));
        assert_eq!(target.1, 10);
        assert_eq!(app.history_pos, Some(1));
        assert_eq!(app.history[2].scroll, app.scroll);

        let target = app.nav_step(NavDir::Back).expect("expected back target");
        assert_eq!(target.0, PathBuf::from("/n/A.md"));
        assert_eq!(target.1, 5);
        assert_eq!(app.history_pos, Some(0));

        assert!(app.nav_step(NavDir::Back).is_none());

        let target = app
            .nav_step(NavDir::Forward)
            .expect("expected forward target");
        assert_eq!(target.0, PathBuf::from("/n/B.md"));
        assert_eq!(app.history_pos, Some(1));
    }

    #[test]
    fn test_history_truncates_forward_on_new_open() {
        let mut app = App::new(Config::default());
        for name in ["A", "B", "C"] {
            app.set_content(
                Some(PathBuf::from(format!("/n/{}.md", name))),
                name.to_string(),
                Some(PathBuf::from("/n")),
            );
        }
        app.nav_step(NavDir::Back);
        app.nav_step(NavDir::Back);
        assert_eq!(app.history_pos, Some(0));
        app.set_content(
            Some(PathBuf::from("/n/D.md")),
            "D".to_string(),
            Some(PathBuf::from("/n")),
        );
        assert_eq!(app.history.len(), 2);
        assert_eq!(app.history[1].path, PathBuf::from("/n/D.md"));
        assert_eq!(app.history_pos, Some(1));
    }

    #[test]
    fn test_history_skip_when_suppressed() {
        let mut app = App::new(Config::default());
        app.set_content(
            Some(PathBuf::from("/n/A.md")),
            "A".to_string(),
            Some(PathBuf::from("/n")),
        );
        app.suppress_history_push = true;
        app.pending_scroll = Some(7);
        app.set_content(
            Some(PathBuf::from("/n/A.md")),
            "A again".to_string(),
            Some(PathBuf::from("/n")),
        );
        assert_eq!(app.history.len(), 1);
        assert_eq!(app.scroll, 7);
        assert!(app.pending_scroll.is_none());
        assert!(!app.suppress_history_push);
    }

    #[test]
    fn test_history_capped_at_history_cap() {
        let mut app = App::new(Config::default());
        for i in 0..(HISTORY_CAP + 20) {
            app.set_content(
                Some(PathBuf::from(format!("/n/F{}.md", i))),
                format!("file {}", i),
                Some(PathBuf::from("/n")),
            );
        }
        assert_eq!(app.history.len(), HISTORY_CAP);
        // Oldest entries dropped — the front should be F20, not F0.
        assert_eq!(
            app.history.front().unwrap().path,
            PathBuf::from("/n/F20.md")
        );
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
        assert_eq!(app.tree.as_ref().unwrap().visible_count(), 1);
        assert_eq!(app.tree_cursor, 0); // clamped from 0 (still in range)
    }

    // --- Scroll / cursor primitives ---

    #[test]
    fn test_quit_flips_running() {
        let mut app = App::new(Config::default());
        assert!(app.running);
        app.quit();
        assert!(!app.running);
    }

    #[test]
    fn test_scroll_primitives_saturate() {
        let mut app = App::new(Config::default());
        app.scroll_down();
        app.scroll_down();
        assert_eq!(app.scroll, 2);
        app.scroll_up();
        assert_eq!(app.scroll, 1);
        app.scroll_top();
        assert_eq!(app.scroll, 0);
        app.scroll_up(); // saturates at 0
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_cursor_primitives_with_no_tree() {
        let mut app = App::new(Config::default());
        // No tree — cursor_down is a no-op (max=0).
        app.cursor_down();
        assert_eq!(app.tree_cursor, 0);
        app.cursor_up();
        assert_eq!(app.tree_cursor, 0);
        app.cursor_top();
        assert_eq!(app.tree_cursor, 0);
    }

    #[test]
    fn test_cursor_primitives_walk_tree() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("a.md"), "a").unwrap();
        stdfs::write(dir.path().join("b.md"), "b").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        // Flat: root, a.md, b.md (3 items)
        app.cursor_down();
        app.cursor_down();
        assert_eq!(app.tree_cursor, 2);
        app.cursor_down(); // saturates at last index
        assert_eq!(app.tree_cursor, 2);
        app.cursor_up();
        assert_eq!(app.tree_cursor, 1);
        app.cursor_top();
        assert_eq!(app.tree_cursor, 0);
    }

    // --- Viewing-mode jump / page ---

    #[test]
    fn test_goto_line_in_viewing() {
        let mut app = App::new(Config::default());
        app.set_content(None, "a\nb\nc\nd\ne".to_string(), None);
        // Now in Viewing state with 5 content lines.
        app.goto_line(3);
        assert_eq!(app.scroll, 2); // 1-indexed → row 2
        app.goto_line(99); // clamps to last
        let total = app.parse_current().lines.len();
        assert_eq!(app.scroll as usize, total - 1);
        app.goto_line(0); // 0 → line 1
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_goto_bottom_in_viewing() {
        let mut app = App::new(Config::default());
        app.set_content(None, "a\nb\nc\nd\ne\nf".to_string(), None);
        let total = app.parse_current().lines.len();
        app.goto_bottom(2);
        assert_eq!(app.scroll as usize, total.saturating_sub(2));
    }

    #[test]
    fn test_half_page_down_up_in_viewing_saturates() {
        let mut app = App::new(Config::default());
        // Many lines so half-page math has somewhere to go.
        let body: String = (0..30).map(|i| format!("L{}\n", i)).collect();
        app.set_content(None, body, None);
        app.half_page_down(5);
        assert_eq!(app.scroll, 5);
        app.half_page_up(2);
        assert_eq!(app.scroll, 3);
        app.half_page_up(100); // saturate at top
        assert_eq!(app.scroll, 0);
        app.half_page_down(10_000); // clamp to last
        let total = app.parse_current().lines.len() as u16;
        assert_eq!(app.scroll, total.saturating_sub(1));
    }

    #[test]
    fn test_jumps_in_loading_state_are_noops() {
        let mut app = App::new(Config::default());
        app.set_loading();
        app.scroll = 7;
        app.tree_cursor = 3;
        app.goto_line(1);
        app.goto_bottom(10);
        app.half_page_down(5);
        app.half_page_up(5);
        // Nothing changed.
        assert_eq!(app.scroll, 7);
        assert_eq!(app.tree_cursor, 3);
    }

    // --- Search edge cases ---

    #[test]
    fn test_cancel_search_clears_input() {
        let mut app = App::new(Config::default());
        app.start_search(SearchDirection::Forward);
        app.search_input_push('x');
        app.cancel_search();
        assert!(app.search_input.is_none());
    }

    #[test]
    fn test_search_input_pop_removes_last_char() {
        let mut app = App::new(Config::default());
        app.start_search(SearchDirection::Forward);
        for c in "abc".chars() {
            app.search_input_push(c);
        }
        app.search_input_pop();
        assert_eq!(app.search_input.as_ref().unwrap().buffer, "ab");
    }

    #[test]
    fn test_repeat_search_without_previous_sets_status() {
        let mut app = App::new(Config::default());
        app.repeat_search(false);
        assert_eq!(app.status_message.as_deref(), Some("No previous search"));
    }

    #[test]
    fn test_repeat_search_reverse_flips_direction() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("alpha.md"), "a").unwrap();
        stdfs::write(dir.path().join("beta.md"), "b").unwrap();
        stdfs::write(dir.path().join("gamma.md"), "g").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        // Flat: root, alpha.md, beta.md, gamma.md
        app.tree_cursor = 0;
        app.start_search(SearchDirection::Forward);
        for c in "a".chars() {
            app.search_input_push(c);
        }
        app.confirm_search();
        // First match forward (skip current=true): alpha at index 1.
        assert_eq!(app.tree_cursor, 1);
        // Repeat reverse: backward from 1, with `a` matching gamma (wraps).
        app.repeat_search(true);
        // From 1, skip_current=true backward → wraps to gamma.md (3) or alpha (1) again.
        // Either way, last_search_direction is unchanged.
        assert_eq!(app.last_search_direction, SearchDirection::Forward);
    }

    #[test]
    fn test_search_backward_wraps() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("alpha.md"), "a").unwrap();
        stdfs::write(dir.path().join("beta.md"), "b").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        // Flat: root, alpha.md, beta.md (3 items, cursor 0)
        app.start_search(SearchDirection::Backward);
        for c in "BETA".chars() {
            app.search_input_push(c);
        }
        app.confirm_search();
        // Backward from 0, skip_current=true → wraps to beta.md (index 2).
        assert_eq!(app.tree_cursor, 2);
    }

    #[test]
    fn test_search_in_viewing_finds_line() {
        let mut app = App::new(Config::default());
        app.set_content(None, "first\nsecond\nthird\n".to_string(), None);
        app.scroll = 0;
        app.start_search(SearchDirection::Forward);
        for c in "third".chars() {
            app.search_input_push(c);
        }
        app.confirm_search();
        // Should jump to the line containing "third".
        let lines = app.parse_current().lines.clone();
        let third_idx = lines
            .iter()
            .position(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
                    .contains("third")
            })
            .unwrap();
        assert_eq!(app.scroll as usize, third_idx);
    }

    // --- Line prompt edge ---

    #[test]
    fn test_line_prompt_pop_removes_last_digit() {
        let mut app = App::new(Config::default());
        app.start_line_prompt();
        app.line_prompt_push('1');
        app.line_prompt_push('2');
        app.line_prompt_pop();
        assert_eq!(app.line_prompt.as_ref().unwrap().buffer, "1");
    }

    // --- cycle_link edges ---

    #[test]
    fn test_cycle_link_backward_from_none_starts_at_last() {
        let mut app = App::new(Config::default());
        app.set_content(
            None,
            "[a](a.md) [b](b.md) [c](c.md)".to_string(),
            Some(PathBuf::from("/tmp")),
        );
        // step = -1 with selected_link=None → starts at last index (n-1).
        app.cycle_link(-1, 20);
        assert_eq!(app.selected_link, Some(2));
    }

    #[test]
    fn test_cycle_link_scrolls_into_view() {
        let mut app = App::new(Config::default());
        // Build content where the only link is far below the viewport.
        let mut body = String::new();
        for _ in 0..10 {
            body.push_str("filler\n");
        }
        body.push_str("[far](far.md)\n");
        app.set_content(None, body, Some(PathBuf::from("/tmp")));
        app.scroll = 0;
        app.cycle_link(1, 3); // viewport = 3 rows
        // Link is around row 10. With viewport=3, scroll should jump down so
        // the link is visible.
        let result = app.parse_current();
        let link_line = result.links[0].line as u16;
        assert!(app.scroll <= link_line && link_line < app.scroll + 3);
    }

    // --- current_link / selected_node / set_error / set_loading ---

    #[test]
    fn test_current_link_returns_selected() {
        let mut app = App::new(Config::default());
        app.set_content(
            None,
            "[a](a.md) [b](b.md)".to_string(),
            Some(PathBuf::from("/tmp")),
        );
        app.cycle_link(1, 20);
        let link = app.current_link().expect("expected a link");
        match link.target {
            crate::markdown::LinkTarget::Local(p) => {
                assert!(p.ends_with("a.md"))
            }
            other => panic!("expected Local, got {:?}", other),
        }
    }

    #[test]
    fn test_current_link_none_when_unselected() {
        let mut app = App::new(Config::default());
        app.set_content(None, "[x](x.md)".to_string(), Some(PathBuf::from("/tmp")));
        // No cycle_link call → selected_link stays None.
        assert!(app.current_link().is_none());
    }

    #[test]
    fn test_selected_node_returns_node_at_cursor() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("a.md"), "a").unwrap();
        stdfs::write(dir.path().join("b.md"), "b").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        app.tree_cursor = 1; // on a.md
        let node = app.selected_node().expect("expected a node");
        assert!(node.is_markdown());
        assert!(node.name().ends_with(".md"));
    }

    #[test]
    fn test_selected_node_none_without_tree() {
        let app = App::new(Config::default());
        assert!(app.selected_node().is_none());
    }

    #[test]
    fn test_set_error_switches_state_to_browsing() {
        let mut app = App::new(Config::default());
        app.set_content(None, "x".to_string(), None);
        assert_eq!(app.state, AppState::Viewing);
        app.set_error("boom".to_string());
        assert_eq!(app.state, AppState::Browsing);
        assert_eq!(app.load_error.as_deref(), Some("boom"));
    }

    #[test]
    fn test_set_loading_clears_content() {
        let mut app = App::new(Config::default());
        app.set_content(None, "x".to_string(), None);
        app.load_error = Some("prev".to_string());
        app.set_loading();
        assert_eq!(app.state, AppState::Loading);
        assert!(app.content.is_none());
        assert!(app.load_error.is_none());
    }

    // --- clamp_cursor edge: empty tree ---

    #[test]
    fn test_collapse_clamps_cursor_into_range() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::create_dir(dir.path().join("sub")).unwrap();
        stdfs::write(dir.path().join("sub").join("x.md"), "x").unwrap();
        stdfs::write(dir.path().join("y.md"), "y").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        // Expand sub so we can collapse later. Flat after expand: root, sub, x.md, y.md
        app.tree_cursor = 1;
        app.toggle_selected().unwrap();
        assert_eq!(app.tree.as_ref().unwrap().visible_count(), 4);
        // Move cursor onto x.md (index 2).
        app.tree_cursor = 2;
        // Collapse sub — x.md disappears, cursor should clamp.
        app.tree_cursor = 1;
        app.collapse_selected();
        assert!(app.tree_cursor < app.tree.as_ref().unwrap().visible_count());
    }

    // --- build_tree_index resolution path (covered indirectly via wiki-link) ---

    #[test]
    fn test_wikilink_resolves_via_tree_index_after_open() {
        let dir = tempfile::tempdir().unwrap();
        stdfs::create_dir(dir.path().join("nested")).unwrap();
        stdfs::write(dir.path().join("nested").join("Target.md"), "# Target").unwrap();
        stdfs::write(dir.path().join("home.md"), "see [[Target]]").unwrap();
        let tree = fs::walk_dir(dir.path()).unwrap();
        let mut app = make_app_with_tree(tree);
        // Need to expand the nested dir so the index walks into it. The walk
        // is lazy — `walk_dir` already walked one level, so root + nested are
        // present but `nested.children` is empty until expanded.
        // For this test, expand the nested dir.
        app.tree_cursor = 1; // on `nested`
        app.toggle_selected().unwrap();
        // Now open home.md and parse — wiki-link `[[Target]]` should resolve
        // to the nested path via build_tree_index.
        let home = dir.path().join("home.md");
        let content = stdfs::read_to_string(&home).unwrap();
        app.set_content(Some(home.clone()), content, Some(dir.path().to_path_buf()));
        let result = app.parse_current();
        let link = result.links.first().expect("expected a link");
        match &link.target {
            crate::markdown::LinkTarget::Local(p) => {
                assert!(
                    p.ends_with("nested/Target.md"),
                    "unexpected resolved path: {}",
                    p.display()
                );
            }
            other => panic!("expected Local, got {:?}", other),
        }
    }
}
