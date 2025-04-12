use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::task::spawn; // Needed for spawning async tasks in closures
use tracing::{error, info}; // Assuming logging is still desired

use super::branch_item::BranchItem;
use crate::{
  action::Action,
  components::traits::{
    list_action_handler::ListActionHandler,
    list_item_wrapper::ListItemWrapper, // Import ListItemWrapper trait
    managed_item::ManagedItem,          // Import ManagedItem trait
  },
  git::types::{GitBranch, GitRepo},
};

#[derive(Default)]
pub struct BranchActionHandler;

// Helper function to create the async closure for operations
// This avoids repeating the spawn logic but requires careful handling of lifetimes and captures.
// Note: This is a simplified example; the actual implementation in GenericListComponent
// might handle state updates (loading, errors, render triggers) more centrally.
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
impl ListActionHandler<BranchItem, GitBranch> for BranchActionHandler {
  // Primary action: Checkout Branch
  fn handle_primary_action(&self, repo: Arc<dyn GitRepo>, item: BranchItem) -> Option<impl FnOnce() + Send> {
    // Take item by value
    let repo_clone = repo.clone();
    let branch_to_checkout = item.inner_item().clone(); // Clone the GitBranch
    info!("BranchActionHandler: Preparing checkout for '{}'", branch_to_checkout.name);

    Some(create_async_operation(move || {
      let branch_name = branch_to_checkout.name.clone();
      async move {
        repo_clone.checkout_branch(&branch_to_checkout).await?;
        info!("Branch checked out: {}", branch_name);
        // TODO: Trigger state refresh (load branches again to update HEAD status)
        Ok(())
      }
    }))
  }

  // Delete action: Delete Branch
  fn handle_delete_action(&self, repo: Arc<dyn GitRepo>, item: BranchItem) -> Option<impl FnOnce() + Send> {
    // Take item by value
    if item.inner_item().is_head {
      info!("BranchActionHandler: Cannot delete HEAD branch '{}'", item.inner_item().name);
      return None; // Cannot delete HEAD branch
    }
    let repo_clone = repo.clone();
    let branch_to_delete = item.inner_item().clone();
    info!("BranchActionHandler: Preparing delete for '{}'", branch_to_delete.name);

    Some(create_async_operation(move || {
      let branch_name = branch_to_delete.name.clone();
      async move {
        repo_clone.delete_branch(&branch_to_delete).await?;
        info!("Branch deleted: {}", branch_name);
        // TODO: Trigger state refresh
        Ok(())
      }
    }))
  }

  // Bulk delete action: Delete Staged Branches
  fn handle_bulk_delete_action(&self, repo: Arc<dyn GitRepo>, items: Vec<BranchItem>) -> Option<impl FnOnce() + Send> {
    let repo_clone = repo.clone();
    let branches_to_delete: Vec<GitBranch> = items
      .iter()
      .filter(|item| item.is_staged_for_deletion() && !item.inner_item().is_head) // Ensure staged and not HEAD
      .map(|item| item.inner_item().clone())
      .collect();

    if branches_to_delete.is_empty() {
      info!("BranchActionHandler: No staged branches to delete.");
      return None;
    }

    info!("BranchActionHandler: Preparing bulk delete for {} branches", branches_to_delete.len());

    Some(create_async_operation(move || {
      async move {
        let total = branches_to_delete.len();
        let mut deleted_count = 0;
        // TODO: Implement progress reporting similar to original list.rs
        for (i, branch) in branches_to_delete.iter().enumerate() {
          info!("Deleting branch {}/{} : {}", i + 1, total, branch.name);
          match repo_clone.delete_branch(branch).await {
            Ok(_) => {
              deleted_count += 1;
            },
            Err(e) => {
              error!("Failed to delete branch {}: {}", branch.name, e);
              // TODO: Collect errors to potentially display later
            },
          }
          // TODO: Update progress in shared state
        }
        info!("Bulk delete complete. Deleted {} branches.", deleted_count);
        // TODO: Trigger state refresh
        Ok(())
      }
    }))
  }

  fn get_create_action(&self) -> Action {
    Action::InitNewBranch // Action to switch to input mode
  }

  fn get_post_create_action(&self, name: String) -> Action {
    Action::CreateBranch(name) // Action dispatched after input submission
  }

  async fn handle_key_event(&self, key: KeyEvent, selected_item: Option<&BranchItem>) -> Result<Option<Action>> {
    // Navigation keys (Up/Down/PgUp/PgDown/Home/End) will be handled by the generic component.
    // Input mode switching (Esc, Enter) will be handled by the generic component/input component.
    let action = match key {
      // --- Branch Specific Actions ---
      // Initiate creation
      KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::SHIFT, .. } => Some(self.get_create_action()),
      // Checkout selected
      KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::NONE, .. } => {
        if selected_item.is_some() {
          Some(Action::CheckoutSelectedBranch) // Generic action name, handled by primary_action
        } else {
          None
        }
      },
      // Unstage for deletion
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::SHIFT, .. } => {
        if selected_item.map_or(false, |item| item.is_staged_for_deletion()) {
          Some(Action::UnstageBranchForDeletion) // Specific action needed for staging toggle
        } else {
          None
        }
      },
      // Delete all staged
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => {
        // Check if there are any staged items before enabling the action
        // This check might be better placed in the generic component based on the full list state
        Some(Action::DeleteStagedBranches) // Generic action name, handled by bulk_delete_action
      },
      // Stage for deletion / Delete immediately if already staged
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::NONE, .. } => {
        match selected_item {
          Some(item) if item.inner_item().is_head => None, // Cannot stage/delete HEAD
          Some(item) if item.is_staged_for_deletion() => Some(Action::DeleteBranch), /* Generic action, handled by delete_action */
          Some(_) => Some(Action::StageBranchForDeletion), // Specific action needed for staging toggle
          None => None,
        }
      },
      _ => None, // Ignore other keys or keys handled by generic component
    };
    Ok(action)
  }

  fn get_instructions(&self, selected_item: Option<&BranchItem>, has_staged_items: bool) -> Vec<&'static str> {
    let mut instructions = vec!["esc: Exit", "shift+c: Create New"];
    if let Some(selected) = selected_item {
      if selected.is_staged_for_deletion() {
        instructions.push("d: Delete"); // This 'd' triggers Action::DeleteBranch
        instructions.push("shift+d: Unstage"); // Triggers Action::UnstageBranchForDeletion
      } else if selected.inner_item().is_head {
        // Can't checkout or delete HEAD
      } else {
        instructions.push("c: Checkout"); // Triggers Action::CheckoutSelectedBranch
        instructions.push("d: Stage for Deletion"); // Triggers Action::StageBranchForDeletion
      }
    }

    if has_staged_items {
      instructions.push("ctrl+d: Delete All Staged"); // Triggers Action::DeleteStagedBranches
    }

    instructions.push("tab: Switch View"); // Assuming Tab is handled globally

    instructions
  }
}
