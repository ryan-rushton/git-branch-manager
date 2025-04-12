use std::sync::Arc;

use async_trait::async_trait;

use super::managed_item::ManagedItem;
use crate::{action::Action, git::types::GitRepo};

/// Defines the contract for handling validation and action creation
/// within the generic input component.
#[async_trait]
pub trait InputHandler<T: ManagedItem>: Send + Sync + 'static {
  /// Validates the proposed input string.
  /// This might involve checking against existing items or repository rules.
  async fn validate_input(&self, repo: Arc<dyn GitRepo>, current_items: &[T], input: &str) -> bool;

  /// Creates the appropriate Action to be dispatched when the input is submitted.
  fn create_submit_action(&self, input: String) -> Action;

  /// Returns the placeholder text or initial value for the input field, if any.
  fn get_input_prompt(&self) -> Option<String> {
    None // Default to no prompt
  }
}
