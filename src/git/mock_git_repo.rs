use async_trait::async_trait;

use super::{GitBranch, GitRepo, GitStash};
use crate::error::Error;

#[derive(Clone, Debug)]
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
