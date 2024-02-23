use std::env::current_dir;

use git2::{Branch, BranchType, Branches, Repository};

use super::error::Error;

pub struct GitBranch {
  pub name: String,
}

pub fn get_current_repo() -> Result<Repository, Error> {
  let path_buf = current_dir().expect("Unable to get current path");
  let repo = Repository::open(path_buf.as_path())?;
  return Ok(repo);
}

fn get_branch_name(result: Result<(Branch, BranchType), git2::Error>) -> Option<GitBranch> {
  let (branch, branch_type) = result.ok()?;
  let name = branch.name().ok()??;
  return Some(GitBranch { name: String::from(name) });
}

pub fn get_local_branches() -> Result<Vec<GitBranch>, Error> {
  let repo = get_current_repo()?;
  let branches = repo.branches(Some(BranchType::Local))?;
  let loaded_branches: Vec<GitBranch> = branches.filter_map(get_branch_name).collect();
  return Ok(loaded_branches);
}

pub fn delete_branch(repo: &Repository, to_delete: &GitBranch) -> Result<(), Error> {
  let branches = repo.branches(Some(BranchType::Local))?;
  for res in branches.into_iter() {
    if res.is_err() {
      continue;
    }
    let (mut branch, branch_type) = res.unwrap();
    if branch.name().is_err() {
      continue;
    }
    let name = branch.name().unwrap();
    if name.is_some() && to_delete.name == name.unwrap() {
      branch.delete().unwrap();
      break;
    }
  }
  return Ok(());
}
