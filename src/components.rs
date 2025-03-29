use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  tui::{Event, Frame},
};

pub mod branch_list;
pub mod error_component;
pub mod stash_list;

#[async_trait::async_trait]
pub trait Component: Send + Sync {
  /// Register an action handler that can send actions for processing if necessary.
  ///
  /// # Arguments
  ///
  /// * `tx` - An unbounded sender that can send actions.
  ///
  /// # Returns
  ///
  /// * `Result<()>` - An Ok result or an error.
  fn register_action_handler(&mut self, _tx: UnboundedSender<Action>) -> Result<()> {
    Ok(())
  }

  /// Handle incoming events and produce actions if necessary.
  ///
  /// # Arguments
  ///
  /// * `event` - An optional event to be processed.
  ///
  /// # Returns
  ///
  /// * `Result<Option<Action>>` - An action to be processed or none.
  async fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
    match event {
      Some(Event::Key(key_event)) => self.handle_key_events(key_event).await,
      _ => Ok(None),
    }
  }

  /// Handle key events and produce actions if necessary.
  ///
  /// # Arguments
  ///
  /// * `key` - A key event to be processed.
  ///
  /// # Returns
  ///
  /// * `Result<Option<Action>>` - An action to be processed or none.
  async fn handle_key_events(&mut self, _key: KeyEvent) -> Result<Option<Action>> {
    Ok(None)
  }

  /// Update the state of the component based on a received action. (REQUIRED)
  ///
  /// # Arguments
  ///
  /// * `action` - An action that may modify the state of the component.
  ///
  /// # Returns
  ///
  /// * `Result<Option<Action>>` - An action to be processed or none.
  async fn update(&mut self, _action: Action) -> Result<Option<Action>> {
    Ok(None)
  }

  /// Render the component on the screen. (REQUIRED)
  ///
  /// # Arguments
  ///
  /// * `f` - A frame used for rendering.
  /// * `area` - The area in which the component should be drawn.
  ///
  /// # Returns
  ///
  /// * `Result<()>` - An Ok result or an error.
  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()>;
}
