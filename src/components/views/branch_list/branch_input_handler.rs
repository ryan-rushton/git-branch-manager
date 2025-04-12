use std::sync::Arc;

use async_trait::async_trait;
use tracing::error; // Assuming logging is needed for validation errors

use crate::{
  action::Action,
  components::traits::{input_handler::InputHandler, managed_item::ManagedItem}, // Import ManagedItem
  git::types::{GitBranch, GitRepo},
};

#[derive(Default)]
pub struct BranchInputHandler;

#[async_trait]
impl InputHandler<GitBranch> for BranchInputHandler {
  async fn validate_input(&self, repo: Arc<dyn GitRepo>, current_items: &[GitBranch], input: &str) -> bool {
    let proposed_name = input.trim();
    if proposed_name.is_empty() {
      return false;
    }

    // Check uniqueness against current items (assuming current_items are GitBranch)
    let is_unique_name = !current_items.iter().any(|b| b.name.eq(proposed_name));
    if !is_unique_name {
      return false;
    }

    // Validate against Git rules
    match repo.validate_branch_name(proposed_name).await {
      Ok(valid) => valid,
      Err(e) => {
        error!("Branch name validation error: {}", e);
        false // Treat validation errors as invalid
      },
    }
  }

  fn create_submit_action(&self, input: String) -> Action {
    // The action handler will take care of the actual creation and checkout logic
    Action::CreateBranch(input.trim().to_string())
  }

  fn get_input_prompt(&self) -> Option<String> {
    Some("Enter new branch name:".to_string()) // Provide a prompt
  }
}
