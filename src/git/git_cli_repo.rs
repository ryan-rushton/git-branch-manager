use std::process::Command;

use regex::Regex;
use tracing::{error, info};

use crate::{
  error::Error,
  git::git_repo::{GitBranch, GitRemoteBranch, GitRepo, GitStash},
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
    let res = run_git_command(&["branch", "--list", "-vv"])?;

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
          name,
          is_head,
          upstream: upstream.map(|upstream_name| GitRemoteBranch::new(String::from(upstream_name.as_str()))),
        }
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
  let args_log_command = args.join(" ");
  info!("Running `git {}`", args_log_command);
  let res = Command::new("git").args(args).output();
  if res.is_err() {
    let err = res.err().unwrap();
    error!("Failed to run `git {}`, error: {}", args_log_command, err);
    return Err(Error::Git(format!("{}", err)));
  }

  let output = res.unwrap();
  let err = String::from_utf8(output.stderr)?;
  if !output.status.success() && !err.is_empty() {
    error!("Failed to run `git {}`, error: {}", args_log_command, err);
    return Err(Error::Git(err));
  }
  let content = String::from_utf8(output.stdout)?;
  info!("Received git cli reply:\n{}", content);
  Ok(content)
}
