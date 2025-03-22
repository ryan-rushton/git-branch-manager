use async_trait::async_trait;
use regex::Regex;
use tokio::process::Command as TokioCommand;
use tracing::{error, info};

use crate::{
  error::Error,
  git::git_repo::{GitBranch, GitRemoteBranch, GitRepo, GitStash},
};

pub struct GitCliRepo {}

impl GitCliRepo {
  pub fn from_cwd() -> Result<GitCliRepo, Error> {
    info!("Creating GitCliRepo from current working directory");
    Ok(GitCliRepo {})
  }
}

async fn run_git_command(args: &[&str]) -> Result<String, Error> {
  let args_log_command = args.join(" ");
  info!("Running `git {}`", args_log_command);
  let output = TokioCommand::new("git").args(args).output().await;
  if output.is_err() {
    let err = output.err().unwrap();
    error!("Failed to run `git {}`, error: {}", args_log_command, err);
    return Err(Error::Git(format!("{}", err)));
  }

  let output = output.unwrap();
  let err = String::from_utf8(output.stderr)?;
  if !output.status.success() && !err.is_empty() {
    error!("Failed to run `git {}`, error: {}", args_log_command, err);
    return Err(Error::Git(err));
  }
  let content = String::from_utf8(output.stdout)?;
  info!("Received git cli reply:\n{}", content);
  Ok(content)
}

#[async_trait]
impl GitRepo for GitCliRepo {
  async fn local_branches(&self) -> Result<Vec<GitBranch>, Error> {
    info!("GitCliRepo: Fetching local branches");
    let res = run_git_command(&["branch", "--list", "-vv"]).await?;
    let branches: Vec<GitBranch> = res
      .lines()
      .map(|line| {
        let trimmed = line.trim();
        // A regex to capture the following git list outputs
        // * git-cli-repo 911ec26 [origin/git-cli-repo] Linting
        //   main         8fb5d9b [origin/main] Fix build
        //   stash-list   6442450 [origin/stash-list: gone] Formatting
        //   test         dbcf785 Updates
        let re = Regex::new(
          r"((?<head>\*)\s+)?(?<name>\S+)\s+(?<sha>[A-Fa-f0-9]+)\s+(\[(?<upstream>[^:|^\]]+)(?<gone>[:\sgone]+)?)?",
        )
        .unwrap();
        let Some(captures) = re.captures(trimmed) else {
          error!("Failed to capture git branch information for: {}", trimmed);
          return GitBranch::new(String::from(trimmed));
        };
        let is_head = captures.name("head").is_some();
        let name = String::from(captures.name("name").unwrap().as_str());
        let upstream = captures.name("upstream");
        GitBranch {
          name: name.clone(),
          is_head,
          upstream: upstream.map(|upstream_name| GitRemoteBranch::new(String::from(upstream_name.as_str()))),
        }
      })
      .collect();
    info!("GitCliRepo: Found {} local branches", branches.len());
    Ok(branches)
  }

  async fn stashes(&mut self) -> Result<Vec<GitStash>, Error> {
    info!("GitCliRepo: Fetching stashes");
    let res = run_git_command(&["stash", "list"]).await?;
    let stashes: Vec<GitStash> = res
      .lines()
      .enumerate()
      .map(|(index, line)| {
        let parts: Vec<&str> = line.splitn(2, ": ").collect();
        let stash_id = parts.first().unwrap_or(&"").to_string();
        let message = parts.get(1).unwrap_or(&"").to_string();
        GitStash::new(index, message, stash_id)
      })
      .collect();
    info!("GitCliRepo: Found {} stashes", stashes.len());
    Ok(stashes)
  }

  async fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error> {
    info!("GitCliRepo: Checking out branch '{}'", branch_name);
    run_git_command(&["checkout", branch_name]).await?;
    info!("GitCliRepo: Successfully checked out branch '{}'", branch_name);
    Ok(())
  }

  async fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    info!("GitCliRepo: Checking out branch '{}'", branch.name);
    self.checkout_branch_from_name(&branch.name).await?;
    info!("GitCliRepo: Successfully checked out branch '{}'", branch.name);
    Ok(())
  }

  async fn validate_branch_name(&self, name: &str) -> Result<bool, Error> {
    info!("GitCliRepo: Validating branch name '{}'", name);
    let res = run_git_command(&["check-ref-format", "--branch", name]).await;
    let is_valid = res.is_ok();
    info!("GitCliRepo: Branch name '{}' validation result: {}", name, is_valid);
    Ok(is_valid)
  }

  async fn create_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    info!("GitCliRepo: Creating and checking out new branch '{}'", branch.name);
    run_git_command(&["checkout", "-b", &branch.name]).await?;
    info!("GitCliRepo: Successfully created and checked out branch '{}'", branch.name);
    Ok(())
  }

  async fn delete_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    info!("GitCliRepo: Deleting branch '{}'", branch.name);
    run_git_command(&["branch", "-D", &branch.name]).await?;
    info!("GitCliRepo: Successfully deleted branch '{}'", branch.name);
    Ok(())
  }
}
