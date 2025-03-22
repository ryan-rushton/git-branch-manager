use std::{env::current_dir, sync::Arc};

use async_trait::async_trait;
use git2::{Branch, BranchType, Repository};
use tokio::sync::Mutex;
use tracing::{error, info};

use super::git_repo::GitStash;
use crate::{
  error::Error,
  git::git_repo::{GitBranch, GitRemoteBranch, GitRepo},
};

pub struct Git2Repo {
  repo: Arc<Mutex<Repository>>,
}

impl Git2Repo {
  pub fn from_cwd() -> Result<Git2Repo, Error> {
    info!("Creating Git2Repo from current working directory");
    let repo = Repository::discover(".")?;
    info!("Git repository discovered");
    Ok(Git2Repo { repo: Arc::new(Mutex::new(repo)) })
  }

  fn create_git_branch(&self, result: Result<(Branch, BranchType), git2::Error>) -> Option<GitBranch> {
    let (branch, _branch_type) = result.ok()?;
    let name = branch.name().ok()??;
    let upstream = extract_upstream_branch(&branch);
    Some(GitBranch { name: String::from(name), is_head: branch.is_head(), upstream })
  }
}

#[async_trait]
impl GitRepo for Git2Repo {
  async fn local_branches(&self) -> Result<Vec<GitBranch>, Error> {
    info!("Git2Repo: Fetching local branches");
    let repo = self.repo.lock().await;
    let branches = repo.branches(Some(BranchType::Local))?;
    let loaded_branches: Vec<GitBranch> = branches.filter_map(|branch| self.create_git_branch(branch)).collect();
    info!("Git2Repo: Found {} local branches", loaded_branches.len());
    Ok(loaded_branches)
  }

  async fn stashes(&mut self) -> Result<Vec<GitStash>, Error> {
    info!("Git2Repo: Fetching stashes");
    let mut repo = self.repo.lock().await;
    let mut stashes: Vec<GitStash> = vec![];
    repo.stash_foreach(|index, message, stash_id| {
      stashes.push(GitStash::new(index, String::from(message), stash_id.to_string()));
      true
    })?;
    info!("Git2Repo: Found {} stashes", stashes.len());
    Ok(stashes)
  }

  async fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error> {
    info!("Git2Repo: Checking out branch '{}'", branch_name);
    let repo = self.repo.lock().await;
    let branch = repo.find_branch(branch_name, BranchType::Local)?;
    let branch_ref = branch.get();
    info!("Git2Repo: Found branch with ref {}", branch_ref.name().unwrap());

    let tree = branch_ref.peel_to_tree()?;
    let checkout_result = repo.checkout_tree(tree.as_object(), None);

    if checkout_result.is_err() {
      let err = checkout_result.unwrap_err();
      error!("Git2Repo: Failed to checkout tree: {}", err);
      return Err(Error::Git("Failed to checkout tree".to_string()));
    }

    let set_head_result = repo.set_head(branch_ref.name().unwrap());
    if set_head_result.is_err() {
      error!("Git2Repo: Failed to set head to: {}", branch_ref.name().unwrap());
      return Err(Error::Git("Failed to set HEAD".to_string()));
    }

    info!("Git2Repo: Successfully checked out branch '{}'", branch_name);
    Ok(())
  }

  async fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    info!("Git2Repo: Checking out branch '{}'", branch.name);
    self.checkout_branch_from_name(&branch.name).await?;
    info!("Git2Repo: Successfully checked out branch '{}'", branch.name);
    Ok(())
  }

  async fn validate_branch_name(&self, name: &str) -> Result<bool, Error> {
    info!("Git2Repo: Validating branch name '{}'", name);
    let is_valid = Branch::name_is_valid(name)?;
    info!("Git2Repo: Branch name '{}' validation result: {}", name, is_valid);
    Ok(is_valid)
  }

  async fn create_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    info!("Git2Repo: Creating branch '{}'", branch.name);
    let repo = self.repo.lock().await;
    let head = repo.head()?;
    let head_oid = head.target();

    if head_oid.is_none() {
      error!("Git2Repo: Attempted to create a branch from a symbolic reference");
      return Err(Error::Git("Attempted to create a branch from a symbolic reference".to_string()));
    }

    let commit = repo.find_commit(head.target().unwrap())?;
    info!("Git2Repo: Using commit {} for new branch", commit.id());
    repo.branch(&branch.name, &commit, false)?;
    info!("Git2Repo: Successfully created branch '{}'", branch.name);
    Ok(())
  }

  async fn delete_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    info!("Git2Repo: Deleting branch '{}'", branch.name);
    let repo = self.repo.lock().await;
    let branches = repo.branches(Some(BranchType::Local))?;
    for res in branches.into_iter() {
      if res.is_err() {
        continue;
      }
      let (mut git_branch, _branch_type) = res?;
      if git_branch.name().is_err() {
        continue;
      }
      let name = git_branch.name()?;
      if name.is_some() && branch.name == name.unwrap() {
        git_branch.delete()?;
        info!("Git2Repo: Successfully deleted branch '{}'", branch.name);
        break;
      }
    }
    Ok(())
  }
}

fn extract_upstream_branch(local_branch: &Branch) -> Option<GitRemoteBranch> {
  let upstream_branch = local_branch.upstream().ok()?;
  let upstream_name = upstream_branch.name().ok()??;
  Some(GitRemoteBranch { name: String::from(upstream_name) })
}
