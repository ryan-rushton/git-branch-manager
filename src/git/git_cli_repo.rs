use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;
use tokio::{process::Command as TokioCommand, sync::RwLock};
use tracing::{error, info, instrument};

use crate::{
  error::Error,
  git::types::{GitBranch, GitRemoteBranch, GitRepo, GitStash},
};

#[derive(Default, Clone)]
pub struct GitCliRepo {
  branch_cache: Arc<RwLock<Vec<GitBranch>>>,
  stash_cache: Arc<RwLock<Vec<GitStash>>>,
}

impl GitCliRepo {
  pub fn from_cwd() -> Result<GitCliRepo, Error> {
    info!("Creating GitCliRepo from current working directory");

    // Check if current directory is a git repository
    let output = std::process::Command::new("git")
      .args(["rev-parse", "--git-dir"])
      .output()
      .map_err(|e| Error::Git(e.to_string()))?;

    if !output.status.success() {
      return Err(Error::NotAGitRepository);
    }

    Ok(GitCliRepo { branch_cache: Arc::new(RwLock::new(Vec::new())), stash_cache: Arc::new(RwLock::new(Vec::new())) })
  }

  #[instrument(skip(self))]
  async fn run_git_command(&self, args: Vec<String>) -> Result<String, Error> {
    let args_log_command = args.join(" ");
    info!(command = %args_log_command, "Running git command");

    // Clone the command string for error reporting
    let args_log_command_clone = args_log_command.clone();

    // Spawn the command in a separate task
    let output = tokio::spawn(async move {
      TokioCommand::new("git").args(&args).output().await.map_err(|err| {
        error!(error = %err, command = %args_log_command, "Failed to run git command");
        Error::Git(err.to_string())
      })
    })
    .await
    .map_err(|e| Error::Git(format!("Task join error: {}", e)))??;

    if !output.status.success() {
      let err = String::from_utf8_lossy(&output.stderr);
      error!(error = %err, command = %args_log_command_clone, "Git command failed");
      return Err(Error::Git(err.to_string()));
    }

    let content = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(content)
  }

  #[instrument(skip(self))]
  async fn parse_branches(&self, output: String) -> Vec<GitBranch> {
    output
      .lines()
      .map(|line| {
        let trimmed = line.trim();
        let re = Regex::new(
          r"((?<head>\*)\s+)?(?<name>\S+)\s+(?<sha>[A-Fa-f0-9]+)\s+(\[(?<upstream>[^:|^\]]+)(?<gone>[:\sgone]+)?)?",
        )
        .unwrap();

        let Some(captures) = re.captures(trimmed) else {
          error!(line = %trimmed, "Failed to parse branch information");
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
      .collect()
  }
}

#[async_trait]
impl GitRepo for GitCliRepo {
  #[instrument(skip(self))]
  async fn local_branches(&self) -> Result<Vec<GitBranch>, Error> {
    info!("Fetching local branches");

    // Try to read from cache first
    {
      let cache = self.branch_cache.read().await;
      if !cache.is_empty() {
        info!(count = cache.len(), "Returning cached branches");
        return Ok(cache.clone());
      }
    }

    // Spawn the branch fetching task
    let output = self.run_git_command(vec!["branch".to_string(), "--list".to_string(), "-vv".to_string()]).await?;
    let branches = self.parse_branches(output).await;

    // Update cache
    {
      let mut cache = self.branch_cache.write().await;
      *cache = branches.clone();
    }

    info!(count = branches.len(), "Found local branches");
    Ok(branches)
  }

  #[instrument(skip(self))]
  async fn stashes(&self) -> Result<Vec<GitStash>, Error> {
    info!("Fetching stashes");

    // Try to read from cache first
    {
      let cache = self.stash_cache.read().await;
      if !cache.is_empty() {
        info!(count = cache.len(), "Returning cached stashes");
        return Ok(cache.clone());
      }
    }

    // Spawn the stash fetching task
    let output = self
      .run_git_command(vec!["stash".to_string(), "list".to_string(), "--format=%gd: %gs (%gd)".to_string()])
      .await?;
    let stashes: Vec<GitStash> = output
      .lines()
      .enumerate()
      .map(|(index, line)| {
        let parts: Vec<&str> = line.splitn(2, ": ").collect();
        let stash_id = parts.first().unwrap_or(&"").to_string();
        let message = parts.get(1).unwrap_or(&"").to_string();
        let branch_name = message.split(" (on ").nth(1).unwrap_or("").trim_end_matches(")").to_string();
        let message = message.split(" (on ").next().unwrap_or("").to_string();
        GitStash::new(index, message, stash_id, branch_name)
      })
      .collect();

    // Update cache
    {
      let mut cache = self.stash_cache.write().await;
      *cache = stashes.clone();
    }

    info!(count = stashes.len(), "Found stashes");
    Ok(stashes)
  }

  #[instrument(skip(self))]
  async fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error> {
    info!(branch = %branch_name, "Checking out branch");
    let result = self.run_git_command(vec!["checkout".to_string(), branch_name.to_string()]).await;

    // Invalidate cache on successful checkout
    if result.is_ok() {
      let mut cache = self.branch_cache.write().await;
      cache.clear();
    }

    Ok(())
  }

  #[instrument(skip(self))]
  async fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    self.checkout_branch_from_name(&branch.name).await
  }

  #[instrument(skip(self))]
  async fn validate_branch_name(&self, name: &str) -> Result<bool, Error> {
    info!(branch_name = %name, "Validating branch name");
    self
      .run_git_command(vec!["check-ref-format".to_string(), "--branch".to_string(), name.to_string()])
      .await
      .map(|_| true)
  }

  #[instrument(skip(self))]
  async fn create_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    info!(branch = %branch.name, "Creating new branch");
    let result = self.run_git_command(vec!["checkout".to_string(), "-b".to_string(), branch.name.clone()]).await;

    // Invalidate cache on successful creation
    if result.is_ok() {
      let mut cache = self.branch_cache.write().await;
      cache.clear();
    }

    Ok(())
  }

  #[instrument(skip(self))]
  async fn delete_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    info!(branch = %branch.name, "Deleting branch");
    let result = self.run_git_command(vec!["branch".to_string(), "-D".to_string(), branch.name.clone()]).await;

    // Invalidate cache on successful deletion
    if result.is_ok() {
      let mut cache = self.branch_cache.write().await;
      cache.clear();
    }

    Ok(())
  }

  #[instrument(skip(self))]
  async fn apply_stash(&self, stash: &GitStash) -> Result<(), Error> {
    info!(stash = %stash.stash_id, "Applying stash");
    let result = self.run_git_command(vec!["stash".to_string(), "apply".to_string(), stash.stash_id.clone()]).await;

    // Invalidate cache on successful apply
    if result.is_ok() {
      let mut cache = self.stash_cache.write().await;
      cache.clear();
    }

    Ok(())
  }

  #[instrument(skip(self))]
  async fn pop_stash(&self, stash: &GitStash) -> Result<(), Error> {
    info!(stash = %stash.stash_id, "Popping stash");
    let result = self.run_git_command(vec!["stash".to_string(), "pop".to_string(), stash.stash_id.clone()]).await;

    // Invalidate cache on successful pop
    if result.is_ok() {
      let mut cache = self.stash_cache.write().await;
      cache.clear();
    }

    Ok(())
  }

  #[instrument(skip(self))]
  async fn drop_stash(&self, stash: &GitStash) -> Result<(), Error> {
    info!(stash = %stash.stash_id, "Dropping stash");
    let result = self.run_git_command(vec!["stash".to_string(), "drop".to_string(), stash.stash_id.clone()]).await;

    // Invalidate cache on successful drop
    if result.is_ok() {
      let mut cache = self.stash_cache.write().await;
      cache.clear();
    }

    Ok(())
  }

  #[instrument(skip(self))]
  async fn stash_with_message(&self, message: &str) -> Result<bool, Error> {
    info!(message = %message, "Stashing changes with message");
    let result =
      self.run_git_command(vec!["stash".to_string(), "push".to_string(), "-m".to_string(), message.to_string()]).await;

    match result {
      Ok(output) => {
        if output.contains("No local changes to save") {
          info!("No local changes to save, stash not created");
          return Ok(false);
        }

        // Invalidate cache on successful stash
        let mut cache = self.stash_cache.write().await;
        cache.clear();
        Ok(true)
      },
      Err(err) => Err(err),
    }
  }
}
