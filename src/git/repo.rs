use std::env::current_dir;

use git2::{Branch, BranchType, Repository};

use crate::error::Error;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitBranch {
  pub name: String,
  pub is_head: bool,
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

  fn create_git_branch(&self, result: Result<(Branch, BranchType), git2::Error>) -> Option<GitBranch> {
    let (branch, _branch_type) = result.ok()?;
    let name = branch.name().ok()??;
    Some(GitBranch { name: String::from(name), is_head: branch.is_head() })
  }

  pub fn local_branches(&self) -> Result<Vec<GitBranch>, Error> {
    let branches = self.repo.branches(Some(BranchType::Local))?;
    let loaded_branches: Vec<GitBranch> = branches.filter_map(|branch| self.create_git_branch(branch)).collect();
    Ok(loaded_branches)
  }

  pub fn checkout_branch_from_name(&self, branch_name: &String) -> Result<(), Error> {
    let obj = self.repo.revparse_single(&("refs/heads/".to_owned() + branch_name)).unwrap();

    self.repo.checkout_tree(&obj, None)?;

    self.repo.set_head(&("refs/heads/".to_owned() + branch_name))?;
    Ok(())
  }

  pub fn checkout_branch(&self, branch: &GitBranch) -> Result<(), Error> {
    self.checkout_branch_from_name(&branch.name)
  }

  pub fn validate_branch_name(&self, name: &String) -> Result<bool, Error> {
    let local_branches = self.local_branches()?;
    let is_unique_name = !local_branches.iter().any(|b| b.name.eq(name));
    Ok(is_unique_name && Branch::name_is_valid(name)?)
  }

  pub fn create_branch(&self, to_create: &GitBranch) -> Result<(), Error> {
    let head = self.repo.head()?;
    let head_oid = head.target();
    if head_oid.is_none() {
      return Err(Error::InternalGit("Attempted to create a branch from a symbolic reference".to_string()));
    }
    let commit = self.repo.find_commit(head.target().unwrap())?;
    self.repo.branch(&to_create.name, &commit, false)?;
    Ok(())
  }

  pub fn delete_branch(&self, to_delete: &GitBranch) -> Result<(), Error> {
    let branches = self.repo.branches(Some(BranchType::Local))?;
    for res in branches.into_iter() {
      if res.is_err() {
        continue;
      }
      let (mut branch, _branch_type) = res.unwrap();
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
