use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Rewrite Obsidian-style `[[WikiLink]]` syntax inside `content` into
/// standard CommonMark `[text](path)` links so the regular markdown parser
/// can handle them.
///
/// Supported forms:
/// - `[[Foo]]`           → `[Foo](path)`
/// - `[[Foo|Display]]`   → `[Display](path)`
/// - `[[Foo#section]]`   → `[Foo#section](path)`  (anchor kept in display)
/// - `[[Foo.md]]`        → `[Foo](path)`          (`.md` stripped from display)
///
/// Targets are resolved via `tree_index` (a lower-cased file-stem → absolute
/// path map built once from the file tree). When the index lacks the name or
/// is `None`, the link target falls back to `<base_dir>/<name>.md`. Wiki-links
/// inside fenced code blocks or inline code spans are left untouched.
pub fn rewrite(
    content: &str,
    tree_index: Option<&HashMap<String, PathBuf>>,
    base_dir: Option<&Path>,
) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_fence = false;
    for line in content.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            out.push_str(line);
            continue;
        }
        if in_fence {
            out.push_str(line);
            continue;
        }
        rewrite_line(line, tree_index, base_dir, &mut out);
    }
    out
}

fn rewrite_line(
    line: &str,
    tree_index: Option<&HashMap<String, PathBuf>>,
    base_dir: Option<&Path>,
    out: &mut String,
) {
    let mut in_inline = false;
    let mut iter = line.char_indices().peekable();
    while let Some((i, ch)) = iter.next() {
        if ch == '`' {
            in_inline = !in_inline;
            out.push('`');
            continue;
        }
        if !in_inline
            && ch == '['
            && matches!(iter.peek(), Some((_, '[')))
            && let Some((inner_len, replacement)) =
                parse_wikilink(&line[i + 2..], tree_index, base_dir)
        {
            out.push_str(&replacement);
            // Skip past `[[` + inner + `]]`. The leading `[` was already
            // consumed by the iterator; advance past the rest by stepping
            // until the byte index moves past `i + 2 + inner_len + 2`.
            let target = i + 2 + inner_len + 2;
            // We've already accepted the first `[`; skip forward.
            while let Some(&(j, _)) = iter.peek() {
                if j >= target {
                    break;
                }
                iter.next();
            }
            continue;
        }
        out.push(ch);
    }
}

/// Try to parse a wiki-link body that follows `[[`. Returns `(inner_len, replacement)`
/// where `inner_len` is the byte length of the inner text between `[[` and `]]`
/// (the `]]` itself is NOT included in the count — the caller adds 2 for it).
fn parse_wikilink(
    after_open: &str,
    tree_index: Option<&HashMap<String, PathBuf>>,
    base_dir: Option<&Path>,
) -> Option<(usize, String)> {
    let close = after_open.find("]]")?;
    let inner = &after_open[..close];
    if inner.contains('\n') || inner.is_empty() {
        return None;
    }
    let (target_part, alias_part) = match inner.split_once('|') {
        Some((t, a)) => (t.trim(), Some(a.trim())),
        None => (inner.trim(), None),
    };
    if target_part.is_empty() {
        return None;
    }
    let (file_part, anchor) = match target_part.split_once('#') {
        Some((f, a)) => (f.trim(), Some(a.trim())),
        None => (target_part, None),
    };
    let file_stem = file_part
        .trim_end_matches(".md")
        .trim_end_matches(".markdown");
    if file_stem.is_empty() {
        return None;
    }

    let resolved = tree_index
        .and_then(|idx| idx.get(&file_stem.to_lowercase()).cloned())
        .or_else(|| base_dir.map(|d| d.join(format!("{}.md", file_stem))));

    let target_for_link = match resolved {
        Some(p) => p.to_string_lossy().into_owned(),
        None => format!("{}.md", file_stem),
    };

    let display = match alias_part {
        Some(a) if !a.is_empty() => a.to_string(),
        _ => match anchor {
            Some(sec) if !sec.is_empty() => format!("{}#{}", file_stem, sec),
            _ => file_stem.to_string(),
        },
    };

    let escaped_display = escape_link_text(&display);
    let escaped_target = escape_link_target(&target_for_link);
    Some((close, format!("[{}]({})", escaped_display, escaped_target)))
}

fn escape_link_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(']', "\\]")
        .replace('[', "\\[")
}

fn escape_link_target(s: &str) -> String {
    if s.contains(' ') || s.contains('(') || s.contains(')') {
        format!("<{}>", s.replace('>', "\\>"))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(pairs: &[(&str, &str)]) -> HashMap<String, PathBuf> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_lowercase(), PathBuf::from(v)))
            .collect()
    }

    #[test]
    fn rewrites_simple_wikilink_to_markdown_link() {
        let out = rewrite("see [[Foo]] later", None, Some(Path::new("/d")));
        assert_eq!(out, "see [Foo](/d/Foo.md) later");
    }

    #[test]
    fn rewrites_alias_with_pipe() {
        let out = rewrite("read [[Foo|the doc]]!", None, Some(Path::new("/d")));
        assert_eq!(out, "read [the doc](/d/Foo.md)!");
    }

    #[test]
    fn keeps_anchor_in_display_strips_from_target() {
        let out = rewrite("see [[Notes#intro]]", None, Some(Path::new("/d")));
        assert_eq!(out, "see [Notes#intro](/d/Notes.md)");
    }

    #[test]
    fn accepts_explicit_md_extension() {
        let out = rewrite("see [[Foo.md]]", None, Some(Path::new("/d")));
        assert_eq!(out, "see [Foo](/d/Foo.md)");
    }

    #[test]
    fn skips_inside_inline_code() {
        let out = rewrite(
            "plain [[A]] but `code [[B]] here` and [[C]]",
            None,
            Some(Path::new("/d")),
        );
        assert_eq!(
            out,
            "plain [A](/d/A.md) but `code [[B]] here` and [C](/d/C.md)"
        );
    }

    #[test]
    fn skips_inside_fenced_code_block() {
        let md = "before [[A]]\n```\nverbatim [[B]] kept\n```\nafter [[C]]\n";
        let out = rewrite(md, None, Some(Path::new("/d")));
        assert!(out.contains("[A](/d/A.md)"));
        assert!(out.contains("verbatim [[B]] kept"));
        assert!(out.contains("[C](/d/C.md)"));
    }

    #[test]
    fn resolves_via_tree_index_case_insensitively() {
        let map = idx(&[("readme", "/n/sub/readme.md")]);
        let out = rewrite("[[ReadMe]]", Some(&map), Some(Path::new("/n")));
        assert!(out.contains("/n/sub/readme.md"), "got: {out}");
    }

    #[test]
    fn falls_back_to_base_dir_when_no_tree_match() {
        let map: HashMap<String, PathBuf> = HashMap::new();
        let out = rewrite("[[Missing]]", Some(&map), Some(Path::new("/base")));
        assert_eq!(out, "[Missing](/base/Missing.md)");
    }

    #[test]
    fn malformed_wikilinks_left_alone() {
        assert_eq!(rewrite("see [[Foo", None, None), "see [[Foo");
        assert_eq!(rewrite("see [[Fo\no]] x", None, None), "see [[Fo\no]] x");
        assert_eq!(rewrite("[[ ]]", None, None), "[[ ]]");
    }

    #[test]
    fn target_with_space_uses_angle_brackets() {
        let out = rewrite("[[My Notes]]", None, Some(Path::new("/d")));
        assert!(out.contains("</d/My Notes.md>"), "got: {out}");
    }

    #[test]
    fn handles_multibyte_chars_around_wikilink() {
        let out = rewrite("熊 [[Foo]] 老师", None, Some(Path::new("/d")));
        assert_eq!(out, "熊 [Foo](/d/Foo.md) 老师");
    }
}
