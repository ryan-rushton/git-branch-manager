use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error("git error: {0}")]
  Git(#[from] git2::Error),
  #[error("internal git error: {0}")]
  InternalGit(String),
}
