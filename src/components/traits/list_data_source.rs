use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;

use super::managed_item::ManagedItem;
use crate::git::types::GitRepo;

/// Defines the contract for fetching the underlying data (`ManagedItem`s)
/// for the generic list component.
#[async_trait]
pub trait ListDataSource<T: ManagedItem>: Send + Sync + 'static {
  /// Fetches the items from the Git repository.
  async fn fetch_items(&self, repo: Arc<dyn GitRepo>) -> Result<Vec<T>>;
}
