#[derive(Debug, Default, PartialEq)]
pub enum AppState {
    #[default]
    Browsing,
    Viewing,
}

pub struct App {
    pub running: bool,
    pub state: AppState,
    pub scroll: u16,
    pub content: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            state: AppState::default(),
            scroll: 0,
            content: None,
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }
}
