use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
  layout::Rect,
  prelude::Color,
  style::Style,
  widgets::{Block, Borders},
};
use tui_textarea::{CursorMove, Input, TextArea};

use crate::{
  action::Action,
  components::traits::{input_handler::InputHandler, managed_item::ManagedItem},
  git::types::GitRepo, // Needed for validation context
  tui::Frame,
};

// Re-using the InputState struct, maybe move to a shared location later?
#[derive(Debug, Default, Clone)]
pub struct InputState {
  pub value: Option<String>,
  pub is_valid: Option<bool>,
}

pub struct GenericInputComponent<H, T>
where
  H: InputHandler<T>,
  T: ManagedItem,
{
  text_input: TextArea<'static>,
  input_state: InputState,
  handler: Arc<H>,                       // Use Arc to allow sharing the handler if needed, or just Box
  repo: Arc<dyn GitRepo>,                // Pass repo for validation context
  current_items: Arc<Vec<T>>,            // Pass current items for validation context
  _phantom: std::marker::PhantomData<T>, // Phantom data for T
}

impl<H, T> GenericInputComponent<H, T>
where
  H: InputHandler<T>,
  T: ManagedItem,
{
  pub fn new(handler: Arc<H>, repo: Arc<dyn GitRepo>, current_items: Arc<Vec<T>>) -> Self {
    let mut text_input = TextArea::default();
    text_input.set_style(Style::default().fg(Color::White));
    text_input.set_block(Block::default().borders(Borders::ALL).title(handler.get_input_prompt().unwrap_or_default()));
    Self {
      text_input,
      input_state: InputState::default(),
      handler,
      repo,
      current_items,
      _phantom: std::marker::PhantomData,
    }
  }

  fn get_text(&self) -> Option<String> {
    let input = String::from(self.text_input.lines().first()?.trim());
    if input.is_empty() { None } else { Some(input) }
  }

  async fn validate(&mut self) {
    if let Some(text) = self.get_text() {
      let is_valid = self.handler.validate_input(self.repo.clone(), &self.current_items, &text).await;
      self.input_state.is_valid = Some(is_valid);
      self.input_state.value = Some(text); // Keep value even if invalid for display
      if is_valid {
        self.text_input.set_style(Style::default().fg(Color::LightGreen));
      } else {
        self.text_input.set_style(Style::default().fg(Color::LightRed));
      }
    } else {
      // Empty input is generally considered invalid for submission
      self.input_state.is_valid = Some(false);
      self.input_state.value = None;
      self.text_input.set_style(Style::default().fg(Color::White)); // Reset style for empty
    }
  }

  // Renamed from handle_key_event to avoid conflict with AsyncComponent trait if used directly
  pub async fn handle_input_event(&mut self, key_event: KeyEvent) -> Option<Action> {
    match key_event {
      KeyEvent { code: KeyCode::Esc, .. } => {
        self.input_state.value = None;
        self.input_state.is_valid = None;
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();
        self.text_input.set_style(Style::default().fg(Color::White)); // Reset style
        Some(Action::EndInputMod) // Signal to parent component to exit input mode
      },
      KeyEvent { code: KeyCode::Enter, .. } => {
        if self.input_state.is_valid.unwrap_or(false) {
          if let Some(text) = self.get_text() {
            let action = self.handler.create_submit_action(text);
            // Clear input after submission
            self.text_input.move_cursor(CursorMove::Head);
            self.text_input.delete_line_by_end();
            self.input_state = InputState::default(); // Reset state
            self.text_input.set_style(Style::default().fg(Color::White)); // Reset style
            Some(action)
          } else {
            None // Should not happen if is_valid is true
          }
        } else {
          None // Do nothing on Enter if input is invalid
        }
      },
      _ => {
        let changed = self.text_input.input(Input::from(key_event));
        if changed {
          self.validate().await; // Re-validate on change
        }
        None // Input handling doesn't directly trigger other actions
      },
    }
  }

  pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
    // Note: TextArea is not mutable here, but widget() takes &self.
    // If styling needs to change dynamically based on state *during* render,
    // the text_input might need to be mutable or styles applied differently.
    f.render_widget(self.text_input.widget(), area);
  }

  // Method to reset the input field when entering input mode
  pub fn reset(&mut self) {
    self.input_state = InputState::default();
    self.text_input.move_cursor(CursorMove::Head);
    self.text_input.delete_line_by_end();
    self.text_input.set_style(Style::default().fg(Color::White));
    // Re-apply title in case it depends on handler state (though currently it doesn't)
    self
      .text_input
      .set_block(Block::default().borders(Borders::ALL).title(self.handler.get_input_prompt().unwrap_or_default()));
  }

  // Method to update current items if they change while input is active
  // (e.g., background refresh)
  pub fn update_current_items(&mut self, current_items: Arc<Vec<T>>) {
    self.current_items = current_items;
    // Optionally re-validate immediately if needed
    // tokio::spawn(self.validate()); // Needs async context or separate handling
  }
}
