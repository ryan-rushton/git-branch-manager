use crossterm::event::KeyEvent;
use ratatui::layout::Rect;

use crate::{action::Action, components::common::text_input::TextInput, tui::Frame};

#[derive(Debug, Default)]
pub struct StashInput {
  pub text_input: TextInput,
}

impl StashInput {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn init_style(&mut self) {
    self.text_input.init_style();
  }

  pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<Action> {
    let validate_fn = |_message: &str| true;

    self.text_input.handle_key_event(key_event, validate_fn).map(Action::CreateStash)
  }

  pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.text_input.render(f, area);
  }
}
