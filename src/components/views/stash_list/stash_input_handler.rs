use std::sync::Arc;

use async_trait::async_trait;

use crate::{
  action::Action,
  components::traits::{input_handler::InputHandler, managed_item::ManagedItem}, // Import ManagedItem
  git::types::{GitRepo, GitStash},                                              // Import GitStash
};

#[derive(Default)]
pub struct StashInputHandler;

#[async_trait]
impl InputHandler<GitStash> for StashInputHandler {
  async fn validate_input(&self, _repo: Arc<dyn GitRepo>, _current_items: &[GitStash], input: &str) -> bool {
    // Stash messages generally don't need validation beyond being non-empty.
    !input.trim().is_empty()
  }

  fn create_submit_action(&self, input: String) -> Action {
    Action::CreateStash(input.trim().to_string())
  }

  fn get_input_prompt(&self) -> Option<String> {
    Some("Enter stash message:".to_string()) // Provide a prompt
  }
}
