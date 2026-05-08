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
    Activate,
    Back,
    SearchForward,
    SearchBackward,
    RepeatNext,
    RepeatPrev,
}

pub type KeyCombo = (KeyCode, KeyModifiers);

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
    Some((code, mods))
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
        "activate" | "open" => Some(Action::Activate),
        "back" => Some(Action::Back),
        "search_forward" => Some(Action::SearchForward),
        "search_backward" => Some(Action::SearchBackward),
        "repeat_next" => Some(Action::RepeatNext),
        "repeat_prev" => Some(Action::RepeatPrev),
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
    }
}
