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
    let res = run_git_command(&["branch", "--list"])?;

    let branches: Vec<GitBranch> = res
      .lines()
      .map(|line| {
        let mut trimmed = line.trim();
        if trimmed.starts_with("*") {
          trimmed = &trimmed[1..trimmed.len()]
        }
        GitBranch::new(trimmed.trim().to_string())
      })
      .collect();

    Ok(branches)
  }

  fn stashes(&mut self) -> Result<Vec<GitStash>, Error> {
    let res = run_git_command(&["branch", "--list"])?;

    let stashes: Vec<GitStash> = res
      .lines()
      .enumerate()
      .map(|(index, line)| GitStash::new(index, String::from(line.trim()), String::new()))
      .collect();

    Ok(stashes)
  }

  fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error> {
    run_git_command(&["checkout", branch_name])?;
    Ok(())
  }

  fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    self.checkout_branch_from_name(&branch.name)
  }

  fn validate_branch_name(&self, name: &str) -> Result<bool, Error> {
    let res = run_git_command(&["check-ref-format", "--branch", name]);
    Ok(res.is_ok())
  }

  fn create_branch(&self, to_create: &GitBranch) -> Result<(), Error> {
    run_git_command(&["checkout", "-b", &to_create.name])?;
    Ok(())
  }

  fn delete_branch(&self, to_delete: &GitBranch) -> Result<(), Error> {
    run_git_command(&["branch", "-D", &to_delete.name])?;
    Ok(())
  }
}

fn run_git_command(args: &[&str]) -> Result<String, Error> {
  let res = Command::new("git").args(args).output();
  if res.is_err() {
    let message = format!("Failed to run git {:?}, error: {}", args, res.err().unwrap());
    return Err(Error::Git(message));
  }

  let output = res.unwrap();
  let err = String::from_utf8(output.stderr)?;
  if !output.status.success() && !err.is_empty() {
    let message = format!("Failed to run git {:?}, error: {}", args, err);
    return Err(Error::Git(message));
  }
  let content = String::from_utf8(output.stdout)?;
  Ok(content)
}