use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, tui::Frame};

pub mod ui;
pub mod views;

pub use ui::ErrorComponent;
pub use views::{BranchList, StashList};

#[async_trait::async_trait]
pub trait AsyncComponent: Component {
  async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> color_eyre::Result<Option<Action>>;
  async fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>>;
}

pub trait Component: Send + Sync {
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()>;
  fn draw(&mut self, frame: &mut Frame<'_>, area: ratatui::layout::Rect) -> color_eyre::Result<()>;
}
