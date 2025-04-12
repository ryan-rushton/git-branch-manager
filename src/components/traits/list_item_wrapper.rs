use ratatui::widgets::ListItem;

use super::managed_item::ManagedItem;

/// Defines the contract for wrapper types around `ManagedItem`s
/// that are displayed in the generic list component.
pub trait ListItemWrapper<T: ManagedItem>: Clone + Send + Sync + 'static {
  /// Creates a new wrapper instance.
  fn new(item: T) -> Self;

  /// Renders the item as a `ratatui::widgets::ListItem`.
  fn render(&self) -> ListItem;

  /// Sets the staged-for-deletion status.
  fn stage_for_deletion(&mut self, stage: bool);

  /// Returns whether the item is staged for deletion.
  fn is_staged_for_deletion(&self) -> bool;

  /// Provides access to the underlying `ManagedItem`.
  fn inner_item(&self) -> &T;
}
