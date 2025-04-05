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
pub struct StashInput {
  pub text_input: TextArea<'static>,
  pub input_state: InputState,
}

impl StashInput {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn init_style(&mut self) {
    self.text_input.set_style(Style::default().fg(Color::White));
    self.text_input.set_block(Block::default().borders(Borders::ALL));
  }

  fn get_text(&self) -> Option<String> {
    let input = String::from(self.text_input.lines().first()?.trim());
    if input.is_empty() {
      return None;
    }
    Some(input)
  }

  pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<Action> {
    match key_event {
      KeyEvent { code: KeyCode::Esc, .. } => {
        self.input_state.value = None;
        self.input_state.is_valid = None;
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();
        Some(Action::EndInputMod)
      },
      KeyEvent { code: KeyCode::Enter, .. } => {
        let stash_message = self.get_text();

        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();

        stash_message.map(Action::CreateStash)
      },
      _ => {
        let changed = self.text_input.input(Input::from(key_event));
        if changed {
          if let Some(message) = self.get_text() {
            self.text_input.set_style(Style::default().fg(Color::LightGreen));
            self.input_state.value = Some(message);
            // Always valid for stash messages
            self.input_state.is_valid = Some(true);
          } else {
            self.text_input.set_style(Style::default().fg(Color::LightRed));
            self.input_state.is_valid = Some(false);
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
