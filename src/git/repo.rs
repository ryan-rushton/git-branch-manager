use std::env::current_dir;

use git2::{Branch, BranchType, Repository};

use crate::error::Error;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitBranch {
  pub name: String,
}

pub struct GitRepo {
  repo: Repository,
}

impl GitRepo {
  pub fn from_cwd() -> Result<GitRepo, Error> {
    let path_buf = current_dir().expect("Unable to get current working directory");
    let repo = Repository::open(path_buf.as_path())?;
    Ok(GitRepo { repo })
  }

  fn get_branch_name(result: Result<(Branch, BranchType), git2::Error>) -> Option<GitBranch> {
    let (branch, branch_type) = result.ok()?;
    let name = branch.name().ok()??;
    Some(GitBranch { name: String::from(name) })
  }

  pub fn local_branches(&mut self) -> Result<Vec<GitBranch>, Error> {
    let branches = self.repo.branches(Some(BranchType::Local))?;
    let loaded_branches: Vec<GitBranch> = branches.filter_map(GitRepo::get_branch_name).collect();
    Ok(loaded_branches)
  }

  pub fn delete_branch(&mut self, to_delete: &GitBranch) -> Result<(), Error> {
    let branches = self.repo.branches(Some(BranchType::Local))?;
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
    Ok(())
  }
}
