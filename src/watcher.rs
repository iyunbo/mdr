//! Live-reload watcher: when the file the user is viewing changes on disk,
//! re-read it and ship a `LoadMsg::Reload` to the main loop.
//!
//! macOS/FSEvents (and most other backends) work at directory granularity,
//! so we watch the parent directory and filter events by the active file
//! path. The active path is stored behind a mutex shared with the notify
//! handler thread.

use crate::LoadMsg;
use notify::event::{EventKind, ModifyKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

pub struct FileWatcher {
    inner: RecommendedWatcher,
    target: Arc<Mutex<Option<PathBuf>>>,
    watched_dir: Option<PathBuf>,
}

impl FileWatcher {
    pub fn new(tx: mpsc::Sender<LoadMsg>) -> notify::Result<Self> {
        let target: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        let target_for_handler = Arc::clone(&target);
        let inner = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            if !is_relevant(&ev.kind) {
                return;
            }
            let Some(active) = target_for_handler.lock().ok().and_then(|g| g.clone()) else {
                return;
            };
            for path in ev.paths {
                if !same_file(&path, &active) {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(&path) else {
                    return;
                };
                let _ = tx.send(LoadMsg::Reload { path, content });
                return;
            }
        })?;
        Ok(Self {
            inner,
            target,
            watched_dir: None,
        })
    }

    /// Switch the active file. The parent directory is (re)watched only when
    /// it actually changes, so opening sibling files in the same dir is free.
    pub fn watch(&mut self, path: &Path) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Ok(mut g) = self.target.lock() {
            *g = Some(canonical.clone());
        }
        let new_dir = canonical
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        if self.watched_dir.as_deref() == Some(new_dir.as_path()) {
            return;
        }
        if let Some(old) = self.watched_dir.take() {
            let _ = self.inner.unwatch(&old);
        }
        if self
            .inner
            .watch(&new_dir, RecursiveMode::NonRecursive)
            .is_ok()
        {
            self.watched_dir = Some(new_dir);
        }
    }
}

fn is_relevant(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Any | ModifyKind::Name(_))
            | EventKind::Create(_)
    )
}

fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}
