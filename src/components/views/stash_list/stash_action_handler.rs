use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::task::spawn;
use tracing::{error, info};

use super::stash_item::StashItem;
use crate::{
  action::Action,
  components::traits::{
    list_action_handler::ListActionHandler, list_item_wrapper::ListItemWrapper, managed_item::ManagedItem,
  },
  git::types::{GitRepo, GitStash},
};

#[derive(Default)]
pub struct StashActionHandler;

// Reusing the helper function concept from BranchActionHandler
// In a real scenario, this might be moved to a shared utility module.
fn create_async_operation<F, Fut>(future_factory: F) -> impl FnOnce() + Send
where
  F: FnOnce() -> Fut + Send + 'static,
  Fut: std::future::Future<Output = Result<(), color_eyre::Report>> + Send + 'static,
{
  move || {
    let future = async move {
      if let Err(err) = future_factory().await {
        error!("Async operation failed: {}", err);
        // TODO: Need a way to signal error back to the main component/shared state
      }
      // TODO: Need a way to signal completion/trigger render back to the main component/shared state
    };
    spawn(future);
  }
}

#[async_trait]
impl ListActionHandler<StashItem, GitStash> for StashActionHandler {
  // Primary action for stashes could be 'apply' or 'pop'. Let's default to 'apply'.
  // The key handler can dispatch different actions (ApplySelectedStash, PopSelectedStash).
  fn handle_primary_action(&self, repo: Arc<dyn GitRepo>, item: StashItem) -> Option<impl FnOnce() + Send> {
    let repo_clone = repo.clone();
    let stash_to_apply = item.inner_item().clone();
    info!("StashActionHandler: Preparing apply for stash '{}'", stash_to_apply.stash_id);

    Some(create_async_operation(move || {
      let stash_id = stash_to_apply.stash_id.clone();
      async move {
        repo_clone.apply_stash(&stash_to_apply).await?;
        info!("Stash applied: {}", stash_id);
        // TODO: Trigger state refresh
        Ok(())
      }
    }))
  }

  // Delete action: Drop Stash
  fn handle_delete_action(&self, repo: Arc<dyn GitRepo>, item: StashItem) -> Option<impl FnOnce() + Send> {
    let repo_clone = repo.clone();
    let stash_to_drop = item.inner_item().clone();
    info!("StashActionHandler: Preparing drop for stash '{}'", stash_to_drop.stash_id);

    Some(create_async_operation(move || {
      let stash_id = stash_to_drop.stash_id.clone();
      async move {
        repo_clone.drop_stash(&stash_to_drop).await?;
        info!("Stash dropped: {}", stash_id);
        // TODO: Trigger state refresh
        Ok(())
      }
    }))
  }

  // Bulk delete action: Drop Staged Stashes
  fn handle_bulk_delete_action(&self, repo: Arc<dyn GitRepo>, items: Vec<StashItem>) -> Option<impl FnOnce() + Send> {
    let repo_clone = repo.clone();
    let stashes_to_drop: Vec<GitStash> =
      items.iter().filter(|item| item.is_staged_for_deletion()).map(|item| item.inner_item().clone()).collect();

    if stashes_to_drop.is_empty() {
      info!("StashActionHandler: No staged stashes to drop.");
      return None;
    }

    info!("StashActionHandler: Preparing bulk drop for {} stashes", stashes_to_drop.len());

    Some(create_async_operation(move || {
      async move {
        let total = stashes_to_drop.len();
        let mut deleted_count = 0;
        // TODO: Implement progress reporting
        for (i, stash) in stashes_to_drop.iter().enumerate() {
          info!("Dropping stash {}/{} : {}", i + 1, total, stash.stash_id);
          match repo_clone.drop_stash(stash).await {
            Ok(_) => {
              deleted_count += 1;
            },
            Err(e) => {
              error!("Failed to drop stash {}: {}", stash.stash_id, e);
              // TODO: Collect errors
            },
          }
          // TODO: Update progress
        }
        info!("Bulk drop complete. Dropped {} stashes.", deleted_count);
        // TODO: Trigger state refresh
        Ok(())
      }
    }))
  }

  fn get_create_action(&self) -> Action {
    Action::InitNewStash // Action to switch to input mode
  }

  fn get_post_create_action(&self, message: String) -> Action {
    Action::CreateStash(message) // Action dispatched after input submission
  }

  async fn handle_key_event(&self, key: KeyEvent, selected_item: Option<&StashItem>) -> Result<Option<Action>> {
    let action = match key {
      // --- Stash Specific Actions ---
      // Initiate creation
      KeyEvent { code: KeyCode::Char('s' | 'S'), modifiers: KeyModifiers::NONE, .. } => Some(self.get_create_action()),
      // Apply selected
      KeyEvent { code: KeyCode::Char('a' | 'A'), modifiers: KeyModifiers::NONE, .. } => {
        if selected_item.is_some() {
          Some(Action::ApplySelectedStash) // Handled by primary_action (or could be specific if needed)
        } else {
          None
        }
      },
      // Pop selected
      KeyEvent { code: KeyCode::Char('p' | 'P'), modifiers: KeyModifiers::NONE, .. } => {
        if selected_item.is_some() {
          Some(Action::PopSelectedStash) // Needs a dedicated handler or modification to primary_action logic
        } else {
          None
        }
      },
      // Unstage for deletion
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::SHIFT, .. } => {
        if selected_item.map_or(false, |item| item.is_staged_for_deletion()) {
          Some(Action::UnstageStashForDeletion)
        } else {
          None
        }
      },
      // Delete all staged
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => {
        Some(Action::DeleteStagedStashes) // Handled by bulk_delete_action
      },
      // Stage for deletion / Drop immediately if already staged
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::NONE, .. } => {
        match selected_item {
          Some(item) if item.is_staged_for_deletion() => Some(Action::DropSelectedStash), // Handled by delete_action
          Some(_) => Some(Action::StageStashForDeletion),
          None => None,
        }
      },
      _ => None,
    };
    Ok(action)
  }

  fn get_instructions(&self, selected_item: Option<&StashItem>, has_staged_items: bool) -> Vec<&'static str> {
    let mut instructions = vec!["esc: Exit", "s: New Stash"];
    if let Some(selected) = selected_item {
      instructions.push("a: Apply");
      instructions.push("p: Pop");

      if selected.is_staged_for_deletion() {
        instructions.push("d: Drop"); // Triggers Action::DropSelectedStash
        instructions.push("shift+d: Unstage"); // Triggers Action::UnstageStashForDeletion
      } else {
        instructions.push("d: Stage for Deletion"); // Triggers Action::StageStashForDeletion
      }
    }

    if has_staged_items {
      instructions.push("ctrl+d: Drop All Staged"); // Triggers Action::DeleteStagedStashes
    }

    instructions.push("tab: Switch View");

    instructions
  }
}
