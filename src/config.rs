// LEARNING NOTE: Why does Config own its Strings instead of borrowing?
//
// A borrowed version would look like:
//   struct Config<'a> {
//       heading_color: &'a str,
//   }
//
// This requires the original TOML string to outlive Config.
// Since we load Config at startup and use it everywhere, that lifetime
// is hard to manage. Owned String (heap-allocated) is simpler and
// appropriate here — the config is small and loaded once.
//
// Use &str (borrowed) when: the caller owns the data, the function is
// short-lived, and you want to avoid cloning.
// Use String (owned) when: the data needs to outlive the call site.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub theme: ThemeConfig,
    pub keys: HashMap<String, KeyBinding>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    pub heading_color: String,
    pub code_color: String,
    #[serde(default = "default_line_number_color")]
    pub line_number_color: String,
    #[serde(default = "default_show_line_numbers")]
    pub show_line_numbers: bool,
}

fn default_line_number_color() -> String {
    "darkgray".to_string()
}

fn default_show_line_numbers() -> bool {
    true
}

// Accepts either `quit = "q"` or `down = ["j", "Down"]` in TOML.
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum KeyBinding {
    Single(String),
    Multiple(Vec<String>),
}

impl KeyBinding {
    pub fn as_slice(&self) -> &[String] {
        match self {
            KeyBinding::Single(s) => std::slice::from_ref(s),
            KeyBinding::Multiple(v) => v.as_slice(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig {
                heading_color: "cyan".to_string(),
                code_color: "yellow".to_string(),
                line_number_color: default_line_number_color(),
                show_line_numbers: default_show_line_numbers(),
            },
            keys: HashMap::from([
                ("quit".to_string(), KeyBinding::Single("q".to_string())),
                (
                    "down".to_string(),
                    KeyBinding::Multiple(vec!["j".to_string(), "Down".to_string()]),
                ),
                (
                    "up".to_string(),
                    KeyBinding::Multiple(vec!["k".to_string(), "Up".to_string()]),
                ),
                ("top".to_string(), KeyBinding::Single("g".to_string())),
                (
                    "activate".to_string(),
                    KeyBinding::Multiple(vec![
                        "Enter".to_string(),
                        "l".to_string(),
                        "Right".to_string(),
                    ]),
                ),
                (
                    "back".to_string(),
                    KeyBinding::Multiple(vec![
                        "Esc".to_string(),
                        "h".to_string(),
                        "Left".to_string(),
                    ]),
                ),
                (
                    "search_forward".to_string(),
                    KeyBinding::Single("/".to_string()),
                ),
                (
                    "search_backward".to_string(),
                    KeyBinding::Single("?".to_string()),
                ),
                (
                    "repeat_next".to_string(),
                    KeyBinding::Single("n".to_string()),
                ),
                (
                    "repeat_prev".to_string(),
                    KeyBinding::Single("N".to_string()),
                ),
                ("bottom".to_string(), KeyBinding::Single("G".to_string())),
                (
                    "halfpage_down".to_string(),
                    KeyBinding::Single("Ctrl+d".to_string()),
                ),
                (
                    "halfpage_up".to_string(),
                    KeyBinding::Single("Ctrl+u".to_string()),
                ),
            ]),
        }
    }
}

pub fn load() -> Config {
    let cfg = Config::default();
    let Some(path) = config_path() else {
        return cfg;
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return cfg;
    };
    let Ok(user) = toml::from_str::<Config>(&content) else {
        return cfg;
    };
    merge(cfg, user)
}

/// Apply user config over defaults: theme is replaced wholesale,
/// key bindings are merged so unspecified actions keep their default keys.
pub fn merge(mut base: Config, user: Config) -> Config {
    base.theme = user.theme;
    for (action, binding) in user.keys {
        base.keys.insert(action, binding);
    }
    base
}

fn config_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".config/mdr/config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_quit_key() {
        let cfg = Config::default();
        assert_eq!(cfg.keys.get("quit").unwrap().as_slice(), &["q".to_string()]);
    }

    #[test]
    fn test_parse_toml_config_string_form() {
        let toml_str = r#"
[theme]
heading_color = "blue"
code_color = "green"

[keys]
quit = "x"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.theme.heading_color, "blue");
        assert_eq!(cfg.keys.get("quit").unwrap().as_slice(), &["x".to_string()]);
    }

    #[test]
    fn test_merge_keeps_default_keys_for_unspecified_actions() {
        let user_toml = r#"
[theme]
heading_color = "red"
code_color = "blue"

[keys]
quit = "Q"
"#;
        let user: Config = toml::from_str(user_toml).unwrap();
        let merged = merge(Config::default(), user);
        // User override
        assert_eq!(merged.theme.heading_color, "red");
        assert_eq!(
            merged.keys.get("quit").unwrap().as_slice(),
            &["Q".to_string()]
        );
        // Defaults preserved
        assert!(merged.keys.contains_key("down"));
        assert!(merged.keys.contains_key("activate"));
        assert!(merged.keys.contains_key("back"));
    }

    #[test]
    fn test_parse_toml_config_array_form() {
        let toml_str = r#"
[theme]
heading_color = "blue"
code_color = "green"

[keys]
down = ["j", "Down"]
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            cfg.keys.get("down").unwrap().as_slice(),
            &["j".to_string(), "Down".to_string()]
        );
    }
}
