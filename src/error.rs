use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error("Git error: {0}")]
  Git(String),
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),
}
