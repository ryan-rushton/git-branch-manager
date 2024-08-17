use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error(transparent)]
  Git2(#[from] git2::Error),

  #[error("Git operation failed: {0}")]
  Git(String),

  #[error(transparent)]
  ParsingError(#[from] std::string::FromUtf8Error),
}
