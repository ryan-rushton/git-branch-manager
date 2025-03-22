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
    let path_buf = current_dir().expect("Unable to get current working directory");
    let repo = Repository::discover(path_buf.as_path())?;
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
    let repo = self.repo.lock().await;
    let branches = repo.branches(Some(BranchType::Local))?;
    let loaded_branches: Vec<GitBranch> = branches.filter_map(|branch| self.create_git_branch(branch)).collect();
    Ok(loaded_branches)
  }

  async fn stashes(&mut self) -> Result<Vec<GitStash>, Error> {
    let mut repo = self.repo.lock().await;
    let mut stashes: Vec<GitStash> = vec![];
    repo.stash_foreach(|index, message, stash_id| {
      stashes.push(GitStash::new(index, String::from(message), stash_id.to_string()));
      true
    })?;
    Ok(stashes)
  }

  async fn checkout_branch_from_name(&self, branch_name: &str) -> Result<(), Error> {
    let repo = self.repo.lock().await;
    info!("Checking out branch {}", branch_name);
    let branch = repo.find_branch(branch_name, BranchType::Local)?;
    let branch_ref = branch.get();
    info!("Found branch with ref {}", branch_ref.name().unwrap());

    let tree = branch_ref.peel_to_tree()?;
    let checkout_result = repo.checkout_tree(tree.as_object(), None);

    if checkout_result.is_err() {
      error!("Failed to checkout tree: {}", checkout_result.unwrap_err());
      return Err(Error::Git("Failed to checkout tree".to_string()));
    }

    let set_head_result = repo.set_head(branch_ref.name().unwrap());
    if set_head_result.is_err() {
      error!("Failed to set head to: {}", branch_ref.name().unwrap());
      return Err(Error::Git("Failed to set HEAD".to_string()));
    }

    Ok(())
  }

  async fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    self.checkout_branch_from_name(&branch.name).await
  }

  async fn validate_branch_name(&self, name: &str) -> Result<bool, Error> {
    Ok(Branch::name_is_valid(name)?)
  }

  async fn create_branch(&self, to_create: &GitBranch) -> Result<(), Error> {
    let repo = self.repo.lock().await;
    info!("Creating branch {}", to_create.name);
    let head = repo.head()?;
    let head_oid = head.target();

    if head_oid.is_none() {
      error!("Attempted to create a branch from a symbolic reference: {}", head_oid.unwrap());
      return Err(Error::Git("Attempted to create a branch from a symbolic reference".to_string()));
    }

    let commit = repo.find_commit(head.target().unwrap())?;
    info!("Using commit for new branch {}", commit.id());
    repo.branch(&to_create.name, &commit, false)?;
    info!("Successfully created branch {}", to_create.name);
    Ok(())
  }

  async fn delete_branch(&self, to_delete: &GitBranch) -> Result<(), Error> {
    let repo = self.repo.lock().await;
    let branches = repo.branches(Some(BranchType::Local))?;
    for res in branches.into_iter() {
      if res.is_err() {
        continue;
      }
      let (mut branch, _branch_type) = res?;
      if branch.name().is_err() {
        continue;
      }
      let name = branch.name()?;
      if name.is_some() && to_delete.name == name.unwrap() {
        branch.delete()?;
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
