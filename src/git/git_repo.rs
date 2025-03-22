use crate::error::Error;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitRemoteBranch {
  pub name: String,
}

impl GitRemoteBranch {
  pub fn new(name: String) -> Self {
    GitRemoteBranch { name }
  }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitBranch {
  pub name: String,
  pub is_head: bool,
  pub upstream: Option<GitRemoteBranch>,
}

impl GitBranch {
  pub fn new(name: String) -> Self {
    GitBranch { name, is_head: false, upstream: None }
  }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitStash {
  pub index: usize,
  pub message: String,
  pub stash_id: String,
}

impl GitStash {
  pub fn new(index: usize, message: String, stash_id: String) -> Self {
    GitStash { index, message, stash_id }
  }
}

#[async_trait::async_trait]
pub trait GitRepo: Send + Sync {
  async fn local_branches(&self) -> Result<Vec<GitBranch>, Error>;
  async fn stashes(&mut self) -> Result<Vec<GitStash>, Error>;
  async fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error>;
  async fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error>;
  async fn validate_branch_name(&self, name: &str) -> Result<bool, Error>;
  async fn create_branch(&self, to_create: &GitBranch) -> Result<(), Error>;
  async fn delete_branch(&self, to_delete: &GitBranch) -> Result<(), Error>;
}
