use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;

use crate::{
  components::traits::{list_data_source::ListDataSource, managed_item::ManagedItem},
  git::types::{GitRepo, GitStash},
};

#[derive(Default)]
pub struct StashDataSource;

#[async_trait]
impl ListDataSource<GitStash> for StashDataSource {
  async fn fetch_items(&self, repo: Arc<dyn GitRepo>) -> Result<Vec<GitStash>> {
    // Removed redundant impl ManagedItem for GitStash {}
    repo.stashes().await.map_err(Into::into) // Map error type
  }
}
