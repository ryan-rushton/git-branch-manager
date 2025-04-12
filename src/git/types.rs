use async_trait::async_trait;

use crate::{components::traits::managed_item::ManagedItem, error::Error};

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

impl ManagedItem for GitBranch {
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitStash {
  pub index: usize,
  pub message: String,
  pub stash_id: String,
  pub branch_name: String,
}

impl GitStash {
  pub fn new(index: usize, message: String, stash_id: String, branch_name: String) -> Self {
    GitStash { index, message, stash_id, branch_name }
  }
}

impl ManagedItem for GitStash {
}

#[async_trait::async_trait]
pub trait GitRepo: Send + Sync {
  async fn local_branches(&self) -> Result<Vec<GitBranch>, Error>;
  async fn stashes(&self) -> Result<Vec<GitStash>, Error>;
  async fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error>;
  async fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error>;
  async fn validate_branch_name(&self, name: &str) -> Result<bool, Error>;
  async fn create_branch(&self, to_create: &GitBranch) -> Result<(), Error>;
  async fn delete_branch(&self, to_delete: &GitBranch) -> Result<(), Error>;
  async fn apply_stash(&self, stash: &GitStash) -> Result<(), Error>;
  async fn pop_stash(&self, stash: &GitStash) -> Result<(), Error>;
  async fn drop_stash(&self, stash: &GitStash) -> Result<(), Error>;
  async fn stash_with_message(&self, message: &str) -> Result<bool, Error>;
}

pub struct MockGitRepo;

#[async_trait]
impl GitRepo for MockGitRepo {
  async fn local_branches(&self) -> Result<Vec<GitBranch>, Error> {
    Ok(vec![GitBranch::new("main".to_string()), GitBranch::new("test".to_string())])
  }

  async fn stashes(&self) -> Result<Vec<GitStash>, Error> {
    Ok(vec![GitStash::new(0, "message1".to_string(), "stash@{0}".to_string(), "branch_name".to_string())])
  }

  async fn checkout_branch_from_name(&self, _branch_name: &str) -> Result<(), Error> {
    Ok(())
  }

  async fn checkout_branch(&self, _branch: &GitBranch) -> Result<(), Error> {
    Ok(())
  }

  async fn validate_branch_name(&self, _name: &str) -> Result<bool, Error> {
    Ok(true)
  }

  async fn create_branch(&self, _to_create: &GitBranch) -> Result<(), Error> {
    Ok(())
  }

  async fn delete_branch(&self, _to_delete: &GitBranch) -> Result<(), Error> {
    Ok(())
  }

  async fn apply_stash(&self, stash: &GitStash) -> Result<(), Error> {
    match stash {
      GitStash { message, .. } if message.to_lowercase().contains("fail") => {
        Err(Error::Git("Apply stash failed".to_string()))
      },
      _ => Ok(()),
    }
  }

  async fn pop_stash(&self, stash: &GitStash) -> Result<(), Error> {
    match stash {
      GitStash { message, .. } if message.to_lowercase().contains("fail") => {
        Err(Error::Git("Pop stash failed".to_string()))
      },
      _ => Ok(()),
    }
  }

  async fn drop_stash(&self, stash: &GitStash) -> Result<(), Error> {
    match stash {
      GitStash { message, .. } if message.to_lowercase().contains("fail") => {
        Err(Error::Git("Drop stash failed".to_string()))
      },
      _ => Ok(()),
    }
  }

  async fn stash_with_message(&self, message: &str) -> Result<bool, Error> {
    match message {
      "should fail" => Err(Error::Git("Stash with message failed".to_string())),
      _ => Ok(true),
    }
  }
}
