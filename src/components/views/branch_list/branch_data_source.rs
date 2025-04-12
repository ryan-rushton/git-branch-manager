use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;

use crate::{
  components::traits::{list_data_source::ListDataSource, managed_item::ManagedItem},
  git::types::{GitBranch, GitRepo},
};

#[derive(Default)]
pub struct BranchDataSource;

#[async_trait]
impl ListDataSource<GitBranch> for BranchDataSource {
  async fn fetch_items(&self, repo: Arc<dyn GitRepo>) -> Result<Vec<GitBranch>> {
    // Removed redundant impl ManagedItem for GitBranch {}
    repo.local_branches().await.map_err(Into::into) // Map error type
  }
}
