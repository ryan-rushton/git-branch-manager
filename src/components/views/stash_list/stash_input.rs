use crossterm::event::KeyEvent;
use ratatui::layout::Rect;

use crate::{action::Action, components::common::text_input::TextInput, tui::Frame};

#[derive(Debug, Default)]
pub struct StashInput {
  text_input: TextInput,
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

    self.text_input.handle_key_event(key_event, validate_fn).map(|action| {
      match action {
        Action::InputSubmitted(text) => Action::CreateStash(text),
        _ => action,
      }
    })
  }

  pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.text_input.render(f, area);
  }
}

#[cfg(test)]
mod tests {
  use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

  use super::*;

  #[test]
  fn test_handle_key_event_submit() {
    let mut stash_input = StashInput::new();
    stash_input.text_input.text_input.insert_str("test stash");

    let action = stash_input.handle_key_event(KeyEvent {
      code: KeyCode::Enter,
      modifiers: KeyModifiers::NONE,
      kind: crossterm::event::KeyEventKind::Press,
      state: crossterm::event::KeyEventState::NONE,
    });

    assert_eq!(action, Some(Action::CreateStash("test stash".to_string())));
  }

  #[test]
  fn test_handle_key_event_escape() {
    let mut stash_input = StashInput::new();
    stash_input.text_input.text_input.insert_str("some input");

    let action = stash_input.handle_key_event(KeyEvent {
      code: KeyCode::Esc,
      modifiers: KeyModifiers::NONE,
      kind: crossterm::event::KeyEventKind::Press,
      state: crossterm::event::KeyEventState::NONE,
    });

    assert_eq!(action, Some(Action::EndInputMode));
    assert!(stash_input.text_input.get_text().is_none());
  }

  #[test]
  fn test_handle_key_event_char() {
    let mut stash_input = StashInput::new();
    stash_input.text_input.text_input.insert_str("some inpu");

    stash_input.handle_key_event(KeyEvent {
      code: KeyCode::Char('t'),
      modifiers: KeyModifiers::NONE,
      kind: crossterm::event::KeyEventKind::Press,
      state: crossterm::event::KeyEventState::NONE,
    });

    assert_eq!(stash_input.text_input.get_text(), Some("some input".to_string()));
  }
}
