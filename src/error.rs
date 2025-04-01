use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error("Not a git repository")]
  NotAGitRepository,
  #[error("{0}")]
  Git(String),
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),
}
