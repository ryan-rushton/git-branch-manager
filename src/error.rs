use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error(transparent)]
  Git(#[from] git2::Error),
  #[error("internal git error: {0}")]
  InternalGit(String),
}
