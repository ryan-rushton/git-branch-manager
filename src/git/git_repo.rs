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

pub trait GitRepo {
  fn local_branches(&self) -> Result<Vec<GitBranch>, Error>;
  fn stashes(&mut self) -> Result<Vec<GitStash>, Error>;
  fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error>;
  fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error>;
  fn validate_branch_name(&self, name: &str) -> Result<bool, Error>;
  fn create_branch(&self, to_create: &GitBranch) -> Result<(), Error>;
  fn delete_branch(&self, to_delete: &GitBranch) -> Result<(), Error>;
}
