use crate::config::Config;
use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Down,
    Up,
    Top,
    Bottom,
    HalfPageDown,
    HalfPageUp,
    PageDown,
    PageUp,
    Activate,
    Back,
    SearchForward,
    SearchBackward,
    RepeatNext,
    RepeatPrev,
    LineJumpPrompt,
    NextLink,
    PrevLink,
    NavBack,
    NavForward,
}

pub type KeyCombo = (KeyCode, KeyModifiers);

/// Normalize a (KeyCode, KeyModifiers) pair so character keys never
/// carry the SHIFT modifier — the case is already encoded in the char,
/// but terminals report shifted ASCII as `Char('G') + SHIFT` on most
/// platforms which would otherwise miss a `G` keymap entry.
pub fn normalize_combo(code: KeyCode, mods: KeyModifiers) -> KeyCombo {
    let mut m = mods;
    if matches!(code, KeyCode::Char(_)) {
        m.remove(KeyModifiers::SHIFT);
    }
    (code, m)
}

/// Parse a key string. Supports plain keys (`q`, `Enter`, `Down`) and
/// modifier prefixes joined by `+` (e.g. `Ctrl+d`, `Shift+Tab`).
pub fn parse_key(s: &str) -> Option<KeyCombo> {
    let mut mods = KeyModifiers::NONE;
    let mut tokens: Vec<&str> = s.split('+').collect();
    let last = tokens.pop()?;
    if last.is_empty() && !tokens.is_empty() {
        // s ended with `+` (e.g. `Ctrl++`); not supported.
        return None;
    }
    for tok in tokens {
        match tok.trim().to_lowercase().as_str() {
            "ctrl" | "control" => mods |= KeyModifiers::CONTROL,
            "shift" => mods |= KeyModifiers::SHIFT,
            "alt" | "meta" | "opt" | "option" => mods |= KeyModifiers::ALT,
            _ => return None,
        }
    }
    let code = parse_key_code(last)?;
    Some(normalize_combo(code, mods))
}

fn parse_key_code(s: &str) -> Option<KeyCode> {
    match s {
        "Enter" | "enter" => Some(KeyCode::Enter),
        "Esc" | "esc" | "Escape" | "escape" => Some(KeyCode::Esc),
        "Up" | "up" => Some(KeyCode::Up),
        "Down" | "down" => Some(KeyCode::Down),
        "Left" | "left" => Some(KeyCode::Left),
        "Right" | "right" => Some(KeyCode::Right),
        "Tab" | "tab" => Some(KeyCode::Tab),
        "Space" | "space" => Some(KeyCode::Char(' ')),
        "Backspace" | "backspace" => Some(KeyCode::Backspace),
        "Home" | "home" => Some(KeyCode::Home),
        "End" | "end" => Some(KeyCode::End),
        "PageUp" | "pageup" => Some(KeyCode::PageUp),
        "PageDown" | "pagedown" => Some(KeyCode::PageDown),
        s => {
            let mut chars = s.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            Some(KeyCode::Char(c))
        }
    }
}

pub fn parse_action(s: &str) -> Option<Action> {
    match s {
        "quit" => Some(Action::Quit),
        "down" | "scroll_down" => Some(Action::Down),
        "up" | "scroll_up" => Some(Action::Up),
        "top" => Some(Action::Top),
        "bottom" | "end" => Some(Action::Bottom),
        "halfpage_down" | "half_page_down" => Some(Action::HalfPageDown),
        "halfpage_up" | "half_page_up" => Some(Action::HalfPageUp),
        "page_down" | "pagedown" => Some(Action::PageDown),
        "page_up" | "pageup" => Some(Action::PageUp),
        "activate" | "open" => Some(Action::Activate),
        "back" => Some(Action::Back),
        "search_forward" => Some(Action::SearchForward),
        "search_backward" => Some(Action::SearchBackward),
        "repeat_next" => Some(Action::RepeatNext),
        "repeat_prev" => Some(Action::RepeatPrev),
        "line_jump_prompt" | "goto_line" => Some(Action::LineJumpPrompt),
        "next_link" => Some(Action::NextLink),
        "prev_link" => Some(Action::PrevLink),
        "nav_back" | "history_back" => Some(Action::NavBack),
        "nav_forward" | "history_forward" => Some(Action::NavForward),
        _ => None,
    }
}

pub fn build_keymap(config: &Config) -> HashMap<KeyCombo, Action> {
    let mut map = HashMap::new();
    for (name, binding) in &config.keys {
        let Some(action) = parse_action(name) else {
            continue;
        };
        for key_str in binding.as_slice() {
            if let Some(combo) = parse_key(key_str) {
                map.insert(combo, action);
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_chars_and_specials() {
        assert_eq!(
            parse_key("q"),
            Some((KeyCode::Char('q'), KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("Enter"),
            Some((KeyCode::Enter, KeyModifiers::NONE))
        );
        assert_eq!(parse_key("Esc"), Some((KeyCode::Esc, KeyModifiers::NONE)));
        assert_eq!(parse_key("Down"), Some((KeyCode::Down, KeyModifiers::NONE)));
        assert_eq!(parse_key("ab"), None);
        assert_eq!(parse_key(""), None);
    }

    #[test]
    fn test_parse_key_with_modifiers() {
        assert_eq!(
            parse_key("Ctrl+d"),
            Some((KeyCode::Char('d'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key("ctrl+u"),
            Some((KeyCode::Char('u'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key("Shift+Tab"),
            Some((KeyCode::Tab, KeyModifiers::SHIFT))
        );
        assert_eq!(
            parse_key("Alt+Enter"),
            Some((KeyCode::Enter, KeyModifiers::ALT))
        );
    }

    #[test]
    fn test_normalize_strips_shift_from_char_keys() {
        // Capital G typed as Shift+g should match a 'G' binding.
        let (code, mods) = normalize_combo(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert_eq!(code, KeyCode::Char('G'));
        assert_eq!(mods, KeyModifiers::NONE);
        // SHIFT is preserved on non-Char keys (e.g. Shift+Tab).
        let (code, mods) = normalize_combo(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(code, KeyCode::Tab);
        assert_eq!(mods, KeyModifiers::SHIFT);
    }

    #[test]
    fn test_build_keymap_from_default_config() {
        let cfg = Config::default();
        let map = build_keymap(&cfg);
        assert_eq!(
            map.get(&(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(&Action::Quit)
        );
        assert_eq!(
            map.get(&(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(&Action::Down)
        );
        assert_eq!(
            map.get(&(KeyCode::Down, KeyModifiers::NONE)),
            Some(&Action::Down)
        );
        assert_eq!(
            map.get(&(KeyCode::Enter, KeyModifiers::NONE)),
            Some(&Action::Activate)
        );
        assert_eq!(
            map.get(&(KeyCode::Char('G'), KeyModifiers::NONE)),
            Some(&Action::Bottom)
        );
        assert_eq!(
            map.get(&(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Some(&Action::HalfPageDown)
        );
        assert_eq!(
            map.get(&(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            Some(&Action::HalfPageUp)
        );
        assert_eq!(
            map.get(&(KeyCode::Char('f'), KeyModifiers::CONTROL)),
            Some(&Action::PageDown)
        );
        assert_eq!(
            map.get(&(KeyCode::Char('b'), KeyModifiers::CONTROL)),
            Some(&Action::PageUp)
        );
        assert_eq!(
            map.get(&(KeyCode::Char(':'), KeyModifiers::NONE)),
            Some(&Action::LineJumpPrompt)
        );
        assert_eq!(
            map.get(&(KeyCode::Tab, KeyModifiers::NONE)),
            Some(&Action::NextLink)
        );
        // `g` is no longer a single-key binding; the chord is handled in main.rs.
        assert_eq!(map.get(&(KeyCode::Char('g'), KeyModifiers::NONE)), None);
    }
}
