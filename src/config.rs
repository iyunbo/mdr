use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub theme: ThemeConfig,
    #[serde(default)]
    pub ui: UiConfig,
    pub keys: HashMap<String, KeyBinding>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UiConfig {
    #[serde(default = "default_mouse")]
    pub mouse: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            mouse: default_mouse(),
        }
    }
}

fn default_mouse() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    #[serde(default = "default_h1_color")]
    pub h1_color: String,
    pub heading_color: String,
    pub code_color: String,
    #[serde(default = "default_line_number_color")]
    pub line_number_color: String,
    #[serde(default = "default_show_line_numbers")]
    pub show_line_numbers: bool,
    #[serde(default = "default_image_height")]
    pub image_height: u16,
    #[serde(default = "default_syntax_highlight")]
    pub syntax_highlight: bool,
    #[serde(default = "default_syntax_theme")]
    pub syntax_theme: String,
}

fn default_h1_color() -> String {
    "lightred".to_string()
}

fn default_line_number_color() -> String {
    "darkgray".to_string()
}

fn default_show_line_numbers() -> bool {
    true
}

fn default_image_height() -> u16 {
    12
}

fn default_syntax_highlight() -> bool {
    true
}

fn default_syntax_theme() -> String {
    "base16-ocean.dark".to_string()
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
                h1_color: default_h1_color(),
                heading_color: "cyan".to_string(),
                code_color: "yellow".to_string(),
                line_number_color: default_line_number_color(),
                show_line_numbers: default_show_line_numbers(),
                image_height: default_image_height(),
                syntax_highlight: default_syntax_highlight(),
                syntax_theme: default_syntax_theme(),
            },
            ui: UiConfig::default(),
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
                    KeyBinding::Multiple(vec!["h".to_string(), "Left".to_string()]),
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
                (
                    "page_down".to_string(),
                    KeyBinding::Single("Ctrl+f".to_string()),
                ),
                (
                    "page_up".to_string(),
                    KeyBinding::Single("Ctrl+b".to_string()),
                ),
                (
                    "line_jump_prompt".to_string(),
                    KeyBinding::Single(":".to_string()),
                ),
                (
                    "next_link".to_string(),
                    KeyBinding::Single("Tab".to_string()),
                ),
                (
                    "prev_link".to_string(),
                    KeyBinding::Single("Shift+Tab".to_string()),
                ),
                (
                    "nav_back".to_string(),
                    KeyBinding::Multiple(vec!["Ctrl+o".to_string(), "[".to_string()]),
                ),
                (
                    "nav_forward".to_string(),
                    KeyBinding::Multiple(vec!["Ctrl+]".to_string(), "]".to_string()]),
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

/// Apply user config over defaults: theme and ui are replaced wholesale,
/// key bindings are merged so unspecified actions keep their default keys.
pub fn merge(mut base: Config, user: Config) -> Config {
    base.theme = user.theme;
    base.ui = user.ui;
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
