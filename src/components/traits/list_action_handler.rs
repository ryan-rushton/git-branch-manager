use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::KeyEvent;

use super::{list_item_wrapper::ListItemWrapper, managed_item::ManagedItem};
use crate::{action::Action, git::types::GitRepo};

/// Defines the contract for handling specific actions and key events
/// within the generic list component, tailored to the item type.
#[async_trait]
pub trait ListActionHandler<W, T>: Send + Sync + 'static
where
  W: ListItemWrapper<T> + Clone, // Add Clone bound for passing W by value
  T: ManagedItem,
{
  /// Handles the primary action for the selected item (e.g., checkout, apply).
  /// Returns a closure that performs the async operation.
  fn handle_primary_action(&self, repo: Arc<dyn GitRepo>, item: W) -> Option<impl FnOnce() + Send>; // Take W by value

  /// Handles the deletion action for the selected item (e.g., delete branch, drop stash).
  /// Returns a closure that performs the async operation.
  fn handle_delete_action(&self, repo: Arc<dyn GitRepo>, item: W) -> Option<impl FnOnce() + Send>; // Take W by value

  /// Handles the bulk deletion of staged items.
  /// Returns a closure that performs the async operation.
  fn handle_bulk_delete_action(&self, repo: Arc<dyn GitRepo>, items: Vec<W>) -> Option<impl FnOnce() + Send>;

  /// Returns the action to initiate the creation of a new item.
  fn get_create_action(&self) -> Action;

  /// Returns the action associated with creating a new item after input.
  /// This might be different from `get_create_action` if creation involves checkout etc.
  fn get_post_create_action(&self, name: String) -> Action;

  /// Maps a key event to a specific Action relevant to this list type.
  async fn handle_key_event(&self, key: KeyEvent, selected_item: Option<&W>) -> Result<Option<Action>>;

  /// Provides the list of keybinding instructions for the footer.
  fn get_instructions(&self, selected_item: Option<&W>, has_staged_items: bool) -> Vec<&'static str>;
}
