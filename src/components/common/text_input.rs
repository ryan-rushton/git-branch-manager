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
        if changed {
          if let Some(new_text) = self.get_text() {
            self.input_state.value = Some(new_text.clone());
            self.input_state.is_valid = Some(validate_fn(&new_text));
          }
        }
        None
      },
    }
  }

  pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    f.render_widget(&self.text_input, area);
  }
}
