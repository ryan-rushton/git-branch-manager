use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
  layout::Rect,
  prelude::Color,
  style::Style,
  widgets::{Block, Borders},
};
use tui_textarea::{CursorMove, Input, TextArea};

use crate::{action::Action, tui::Frame};

#[derive(Debug, Default, Clone)]
pub struct InputState {
  pub value: Option<String>,
  pub is_valid: Option<bool>,
}

#[derive(Debug, Default)]
pub struct TextInput {
  pub text_input: TextArea<'static>,
  pub input_state: InputState,
}

impl TextInput {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn init_style(&mut self) {
    self.text_input.set_style(Style::default().fg(Color::White));
    self.text_input.set_block(Block::default().borders(Borders::ALL));
  }

  pub fn get_text(&self) -> Option<String> {
    let input = String::from(self.text_input.lines().first()?.trim());
    if input.is_empty() {
      return None;
    }
    Some(input)
  }

  // Returns the submitted text if the input is valid and enter was pressed.
  pub fn handle_key_event<F>(&mut self, key_event: KeyEvent, validate_fn: F) -> Option<Action>
  where
    F: Fn(&str) -> bool,
  {
    match key_event {
      KeyEvent { code: KeyCode::Esc, .. } => {
        self.input_state.value = None;
        self.input_state.is_valid = None;
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();
        Some(Action::EndInputMode)
      },
      KeyEvent { code: KeyCode::Enter, .. } => {
        let input_text = self.get_text();
        if let Some(text) = input_text {
          if validate_fn(&text) {
            self.input_state.value = Some(text.clone());
            self.input_state.is_valid = Some(true);
            self.text_input.move_cursor(CursorMove::Head);
            self.text_input.delete_line_by_end();
            return Some(Action::InputSubmitted(text.clone()));
          } else {
            self.input_state.is_valid = Some(false);
          }
        }
        None
      },
      _ => {
        let changed = self.text_input.input(Input::from(key_event));
        if changed
          && let Some(new_text) = self.get_text() {
            self.input_state.value = Some(new_text.clone());
            self.input_state.is_valid = Some(validate_fn(&new_text));
          }
        None
      },
    }
  }

  pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    f.render_widget(&self.text_input, area);
  }
}

#[cfg(test)]
mod tests {
  use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

  use super::*;

  #[test]
  fn test_get_text() {
    let mut text_input = TextInput::new();
    text_input.text_input.insert_str("test input");

    assert_eq!(text_input.get_text(), Some("test input".to_string()));
  }

  #[test]
  fn test_get_text_empty() {
    let text_input = TextInput::new();

    assert_eq!(text_input.get_text(), None);
  }

  #[test]
  fn test_handle_key_event_enter_valid_input() {
    let mut text_input = TextInput::new();
    text_input.text_input.insert_str("valid input");

    let validate_fn = |input: &str| !input.is_empty();
    let action = text_input.handle_key_event(
      KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
      },
      validate_fn,
    );

    assert_eq!(action, Some(Action::InputSubmitted("valid input".to_string())));
    assert_eq!(text_input.input_state.value, Some("valid input".to_string()));
    assert_eq!(text_input.input_state.is_valid, Some(true));
  }

  #[test]
  fn test_handle_key_event_enter_invalid_input() {
    let mut text_input = TextInput::new();
    text_input.text_input.insert_str("invalid input");

    let validate_fn = |input: &str| !input.eq("invalid input");
    let action = text_input.handle_key_event(
      KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
      },
      validate_fn,
    );

    assert_eq!(action, None);
    assert_eq!(text_input.input_state.value, None);
    assert_eq!(text_input.input_state.is_valid, Some(false));
  }

  #[test]
  fn test_handle_key_event_escape() {
    let mut text_input = TextInput::new();
    text_input.text_input.insert_str("some input");

    let validate_fn = |_input: &str| true;
    let action = text_input.handle_key_event(
      KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
      },
      validate_fn,
    );

    assert_eq!(action, Some(Action::EndInputMode));
    assert_eq!(text_input.input_state.value, None);
    assert_eq!(text_input.input_state.is_valid, None);
  }

  #[test]
  fn test_handle_key_event_typing() {
    let mut text_input = TextInput::new();

    let validate_fn = |_input: &str| true;
    text_input.handle_key_event(
      KeyEvent {
        code: KeyCode::Char('h'),
        modifiers: KeyModifiers::NONE,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
      },
      validate_fn,
    );
    text_input.handle_key_event(
      KeyEvent {
        code: KeyCode::Char('i'),
        modifiers: KeyModifiers::NONE,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
      },
      validate_fn,
    );

    assert_eq!(text_input.get_text(), Some("hi".to_string()));
  }
}
