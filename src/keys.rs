use crate::config::Config;
use crossterm::event::KeyCode;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Down,
    Up,
    Top,
    Activate,
    Back,
    SearchForward,
    SearchBackward,
    RepeatNext,
    RepeatPrev,
}

pub fn parse_key(s: &str) -> Option<KeyCode> {
    match s {
        "Enter" | "enter" => Some(KeyCode::Enter),
        "Esc" | "esc" | "Escape" | "escape" => Some(KeyCode::Esc),
        "Up" | "up" => Some(KeyCode::Up),
        "Down" | "down" => Some(KeyCode::Down),
        "Left" | "left" => Some(KeyCode::Left),
        "Right" | "right" => Some(KeyCode::Right),
        "Tab" | "tab" => Some(KeyCode::Tab),
        "Space" | "space" => Some(KeyCode::Char(' ')),
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
        "activate" | "open" => Some(Action::Activate),
        "back" => Some(Action::Back),
        "search_forward" => Some(Action::SearchForward),
        "search_backward" => Some(Action::SearchBackward),
        "repeat_next" => Some(Action::RepeatNext),
        "repeat_prev" => Some(Action::RepeatPrev),
        _ => None,
    }
}

pub fn build_keymap(config: &Config) -> HashMap<KeyCode, Action> {
    let mut map = HashMap::new();
    for (name, binding) in &config.keys {
        let Some(action) = parse_action(name) else {
            continue;
        };
        for key_str in binding.as_slice() {
            if let Some(code) = parse_key(key_str) {
                map.insert(code, action);
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
        assert_eq!(parse_key("q"), Some(KeyCode::Char('q')));
        assert_eq!(parse_key("Enter"), Some(KeyCode::Enter));
        assert_eq!(parse_key("Esc"), Some(KeyCode::Esc));
        assert_eq!(parse_key("Down"), Some(KeyCode::Down));
        assert_eq!(parse_key("ab"), None);
        assert_eq!(parse_key(""), None);
    }

    #[test]
    fn test_build_keymap_from_default_config() {
        let cfg = Config::default();
        let map = build_keymap(&cfg);
        assert_eq!(map.get(&KeyCode::Char('q')), Some(&Action::Quit));
        assert_eq!(map.get(&KeyCode::Char('j')), Some(&Action::Down));
        assert_eq!(map.get(&KeyCode::Down), Some(&Action::Down));
        assert_eq!(map.get(&KeyCode::Enter), Some(&Action::Activate));
        assert_eq!(map.get(&KeyCode::Esc), Some(&Action::Back));
    }
}
