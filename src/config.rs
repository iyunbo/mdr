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
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    pub heading_color: String,
    pub code_color: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig {
                heading_color: "cyan".to_string(),
                code_color: "yellow".to_string(),
            },
            keys: HashMap::from([
                ("quit".to_string(), "q".to_string()),
                ("scroll_down".to_string(), "j".to_string()),
                ("scroll_up".to_string(), "k".to_string()),
                ("top".to_string(), "g".to_string()),
            ]),
        }
    }
}

pub fn load() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    toml::from_str::<Config>(&content).unwrap_or_default()
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
        assert_eq!(cfg.keys.get("quit"), Some(&"q".to_string()));
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_str = r#"
[theme]
heading_color = "blue"
code_color = "green"

[keys]
quit = "x"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.theme.heading_color, "blue");
        assert_eq!(cfg.keys.get("quit"), Some(&"x".to_string()));
    }
}
