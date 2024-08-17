use std::process::Command;

use crate::{
  error::Error,
  git::git_repo::{GitBranch, GitRepo, GitStash},
};

pub struct GitCliRepo {}

impl GitCliRepo {
  pub fn from_cwd() -> Result<GitCliRepo, Error> {
    // TODO check that the user is in a repo and throw if not
    Ok(GitCliRepo {})
  }
}

impl GitRepo for GitCliRepo {
  fn local_branches(&self) -> Result<Vec<GitBranch>, Error> {
    let res = Command::new("git").args(["branch", "--list"]).output();
    if res.is_err() {
      return Err(Error::Git("Failed to get local branches".to_string()));
    }

    let output = res.unwrap();
    if !output.status.success() {
      return Err(Error::Git("Failed to get local branches".to_string()));
    }
    let err = String::from_utf8(output.stderr)?;
    let content = String::from_utf8(output.stdout)?;

    let mut branches: Vec<GitBranch> = Vec::new();
    content.lines().for_each(|line| {
      let mut trimmed = line.trim();
      if trimmed.starts_with("*") {
        trimmed = &trimmed[1..trimmed.len()]
      }
      branches.push(GitBranch::new(trimmed.trim().to_string()))
    });
    Ok(branches)
  }

  fn stashes(&mut self) -> Result<Vec<GitStash>, Error> {
    todo!()
  }

  fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error> {
    todo!()
  }

  fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    todo!()
  }

  fn validate_branch_name(&self, name: &str) -> Result<bool, Error> {
    todo!()
  }

  fn create_branch(&self, to_create: &GitBranch) -> Result<(), Error> {
    todo!()
  }

  fn delete_branch(&self, to_delete: &GitBranch) -> Result<(), Error> {
    todo!()
  }
}
