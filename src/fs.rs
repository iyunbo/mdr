use crate::error::AppError;
use std::path::{Path, PathBuf};

pub fn read_file(path: &str) -> Result<String, AppError> {
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

pub fn is_markdown_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("markdown")
    )
}

pub async fn read_file_async(path: PathBuf) -> Result<String, AppError> {
    tokio::fs::read_to_string(&path)
        .await
        .map_err(AppError::from)
}

#[derive(Debug, Clone)]
pub enum FileNode {
    File(PathBuf),
    Dir {
        path: PathBuf,
        name: String,
        children: Vec<FileNode>,
        expanded: bool,
    },
}

impl FileNode {
    pub fn name(&self) -> &str {
        match self {
            FileNode::File(p) => p.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
            FileNode::Dir { name, .. } => name,
        }
    }

    pub fn is_markdown(&self) -> bool {
        match self {
            FileNode::File(p) => is_markdown_path(p),
            FileNode::Dir { .. } => false,
        }
    }

    /// Visit this node and (recursively) the children of any *expanded* dir,
    /// in DFS order. Collapsed dirs are not descended; their `children` are
    /// also typically empty since walks are lazy on first expansion. Closure
    /// receives `(depth, node)` with depth 0 at the root.
    pub fn visit_visible<'a, F: FnMut(usize, &'a FileNode)>(&'a self, depth: usize, f: &mut F) {
        f(depth, self);
        if let FileNode::Dir {
            children,
            expanded: true,
            ..
        } = self
        {
            for child in children {
                child.visit_visible(depth + 1, f);
            }
        }
    }

    pub fn visible_count(&self) -> usize {
        let mut n = 0;
        self.visit_visible(0, &mut |_, _| n += 1);
        n
    }

    /// Visit this node and (recursively) every descendant regardless of
    /// expanded state. Use when the visitor cares about the *known* tree
    /// (e.g. wiki-link index), not the visible projection.
    pub fn visit_all<'a, F: FnMut(&'a FileNode)>(&'a self, f: &mut F) {
        f(self);
        if let FileNode::Dir { children, .. } = self {
            for child in children {
                child.visit_all(f);
            }
        }
    }
}

pub fn walk_dir(path: &Path) -> Result<FileNode, AppError> {
    if !path.is_dir() {
        return Err(AppError::Other(format!(
            "'{}' is not a directory",
            path.display()
        )));
    }

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".")
        .to_string();

    let mut children: Vec<FileNode> = std::fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| !entry.file_name().to_str().unwrap_or("").starts_with('.'))
        .map(|entry| {
            let p = entry.path();
            if p.is_dir() {
                FileNode::Dir {
                    name: p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                        .to_string(),
                    path: p,
                    children: Vec::new(),
                    expanded: false,
                }
            } else {
                FileNode::File(p)
            }
        })
        .collect();

    children.sort_by(|a, b| match (a, b) {
        (FileNode::Dir { name: a, .. }, FileNode::Dir { name: b, .. }) => a.cmp(b),
        (FileNode::File(a), FileNode::File(b)) => a.cmp(b),
        (FileNode::Dir { .. }, FileNode::File(_)) => std::cmp::Ordering::Less,
        (FileNode::File(_), FileNode::Dir { .. }) => std::cmp::Ordering::Greater,
    });

    Ok(FileNode::Dir {
        path: path.to_path_buf(),
        name,
        children,
        expanded: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_read_file_returns_content() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "# Hello mdr").unwrap();
        let path = f.path().to_str().unwrap().to_string();
        let content = read_file(&path).unwrap();
        assert!(content.contains("# Hello mdr"));
    }

    #[test]
    fn test_read_file_missing_returns_error() {
        let result = read_file("/tmp/this_file_does_not_exist_mdr.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_walk_dir_finds_md_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("readme.md"), "# Hello").unwrap();
        fs::write(dir.path().join("notes.md"), "notes").unwrap();
        fs::write(dir.path().join("other.txt"), "text").unwrap();

        let node = walk_dir(dir.path()).unwrap();
        match node {
            FileNode::Dir { children, .. } => {
                let md_count = children.iter().filter(|n| n.is_markdown()).count();
                assert_eq!(md_count, 2);
            }
            _ => panic!("Expected Dir variant"),
        }
    }

    #[test]
    fn test_walk_dir_sorts_dirs_before_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("zzz.md"), "z").unwrap();
        fs::create_dir(dir.path().join("aaa_subdir")).unwrap();

        let node = walk_dir(dir.path()).unwrap();
        match node {
            FileNode::Dir { children, .. } => {
                assert!(matches!(children[0], FileNode::Dir { .. }));
                assert!(matches!(children[1], FileNode::File(_)));
            }
            _ => panic!("Expected Dir variant"),
        }
    }

    #[test]
    fn test_walk_dir_on_file_returns_error() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let result = walk_dir(f.path());
        assert!(result.is_err(), "Expected error for file path");
    }
}
